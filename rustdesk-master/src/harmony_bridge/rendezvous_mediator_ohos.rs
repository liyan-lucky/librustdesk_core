use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};

use hbb_common::{
    allow_err,
    bytes::Bytes,
    config::{self, option2bool, Config, CONNECT_TIMEOUT, RENDEZVOUS_PORT},
    futures::future::join_all,
    log,
    protobuf::{Enum as _, Message as _},
    rendezvous_proto::*,
    sleep,
    socket_client::{self, is_ipv4, new_udp_for},
    tokio::{self, select, sync::Mutex, time::interval},
    udp::FramedSocket,
    AddrMangle, IntoTargetAddr, ResultType, TargetAddr,
};

use crate::server::{self, ServerPtr};

type Message = RendezvousMessage;

lazy_static::lazy_static! {
    static ref SOLVING_PK_MISMATCH: Mutex<String> = Default::default();
    static ref LAST_MSG: Mutex<(std::net::SocketAddr, Instant)> = Mutex::new((std::net::SocketAddr::new([0; 4].into(), 0), Instant::now()));
    static ref LAST_RELAY_MSG: Mutex<(std::net::SocketAddr, Instant)> = Mutex::new((std::net::SocketAddr::new([0; 4].into(), 0), Instant::now()));
}

pub(crate) static NEEDS_DEPLOY: AtomicBool = AtomicBool::new(false);
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);
static MANUAL_RESTARTED: AtomicBool = AtomicBool::new(false);
static LAN_LISTENER_STARTED: AtomicBool = AtomicBool::new(false);

const MIN_REG_TIMEOUT: i64 = 3_000;
const MAX_REG_TIMEOUT: i64 = 30_000;
const REG_INTERVAL: i64 = 12_000;
const MAX_FAILS1: i64 = 2;
const MAX_FAILS2: i64 = 4;
const DNS_INTERVAL: i64 = 60_000;

#[derive(Clone)]
pub struct RendezvousMediator {
    addr: TargetAddr<'static>,
    host: String,
    host_prefix: String,
    keep_alive: i32,
}

impl RendezvousMediator {
    pub fn restart() {
        hbb_common::log::info!("OHOS rendezvous mediator restart requested");
        SHOULD_EXIT.store(true, Ordering::SeqCst);
        MANUAL_RESTARTED.store(true, Ordering::SeqCst);
        start_lan_listener_once();
    }

    pub async fn start_all() {
        if config::is_outgoing_only() {
            loop {
                sleep(1.).await;
            }
        }

        crate::hbbs_http::sync::start();

        let server = new_server();
        if config::option2bool("stop-service", &Config::get_option("stop-service")) {
            crate::test_rendezvous_server();
        }

        start_lan_listener_once();

        crate::harmony_bridge::core::queue_event(
            "server-started",
            "OHOS server started, connecting to signaling servers",
            "",
        );

        *SOLVING_PK_MISMATCH.lock().await = "".to_owned();
        loop {
            let conn_start_time = Instant::now();
            *SOLVING_PK_MISMATCH.lock().await = "".to_owned();
            if !config::option2bool("stop-service", &Config::get_option("stop-service")) {
                let mut futs = Vec::new();
                let servers = Config::get_rendezvous_servers();
                SHOULD_EXIT.store(false, Ordering::SeqCst);
                MANUAL_RESTARTED.store(false, Ordering::SeqCst);
                for host in servers.clone() {
                    let server = server.clone();
                    futs.push(tokio::spawn(async move {
                        if let Err(err) = Self::start(server, host).await {
                            hbb_common::log::error!("OHOS rendezvous mediator error: {err}");
                        }
                        SHOULD_EXIT.store(true, Ordering::SeqCst);
                    }));
                }
                join_all(futs).await;
            }
            Config::reset_online();
            if !MANUAL_RESTARTED.load(Ordering::SeqCst) {
                let elapsed = conn_start_time.elapsed().as_millis() as u64;
                if elapsed < CONNECT_TIMEOUT as u64 {
                    sleep(((CONNECT_TIMEOUT as u64 - elapsed) / 1000) as _).await;
                }
            } else {
                sleep(0.033).await;
            }
        }
    }

    fn get_host_prefix(host: &str) -> String {
        host.split(".")
            .next()
            .map(|x| {
                if x.parse::<i32>().is_ok() {
                    host.to_owned()
                } else {
                    x.to_owned()
                }
            })
            .unwrap_or_default()
    }

    pub async fn start(server: ServerPtr, host: String) -> ResultType<()> {
        let host = crate::check_port(&host, RENDEZVOUS_PORT);
        log::info!("OHOS start rendezvous mediator of {}", host);

        let (mut socket, mut addr) = new_udp_for(&host, CONNECT_TIMEOUT).await?;
        let mut rz = Self {
            addr: addr.clone(),
            host: host.clone(),
            host_prefix: Self::get_host_prefix(&host),
            keep_alive: crate::DEFAULT_KEEP_ALIVE,
        };

        let mut timer = crate::rustdesk_interval(interval(crate::TIMER_OUT));
        let mut reg_timeout = MIN_REG_TIMEOUT;
        let mut fails = 0;
        let mut last_register_resp: Option<Instant> = None;
        let mut last_register_sent: Option<Instant> = None;
        let mut last_dns_check = Instant::now();

        loop {
            select! {
                n = socket.next() => {
                    match n {
                        Some(Ok((bytes, _))) => {
                            if let Ok(msg) = Message::parse_from_bytes(&bytes) {
                                allow_err!(
                                    rz.handle_resp(msg.union, &mut socket, &addr, &server).await
                                );
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("OHOS socket recv error: {}", e);
                            break;
                        }
                        None => {
                            log::warn!("OHOS socket closed");
                            break;
                        }
                    }
                }
                _ = timer.tick() => {
                    if SHOULD_EXIT.load(Ordering::SeqCst) {
                        break;
                    }
                    let now = Some(Instant::now());
                    let expired = last_register_resp.map(|x| x.elapsed().as_millis() as i64 >= REG_INTERVAL).unwrap_or(true);
                    let timeout = last_register_sent.map(|x| x.elapsed().as_millis() as i64 >= reg_timeout).unwrap_or(false);

                    if timeout {
                        fails += 1;
                        if fails >= MAX_FAILS2 {
                            Config::update_latency(&host, -1);
                            if last_dns_check.elapsed().as_millis() as i64 > DNS_INTERVAL {
                                if let Some((s, new_addr)) = socket_client::rebind_udp_for(&rz.host).await? {
                                    socket = s;
                                    rz.addr = new_addr.clone();
                                    addr = new_addr;
                                }
                                last_dns_check = Instant::now();
                            }
                        } else if fails >= MAX_FAILS1 {
                            Config::update_latency(&host, 0);
                        }
                    }

                    if timeout || (last_register_sent.is_none() && expired) {
                        rz.register_peer(&mut socket, &addr).await?;
                        last_register_sent = now;
                    }
                }
            }
        }
        Ok(())
    }

    async fn register_peer(&self, socket: &mut FramedSocket, addr: &TargetAddr<'static>) -> ResultType<()> {
        if !Config::get_key_confirmed() || !Config::get_host_key_confirmed(&self.host_prefix) {
            log::info!("OHOS register_pk for {}", self.host_prefix);
            self.register_pk(socket, addr).await?;
            return Ok(());
        }
        let id = Config::get_id();
        let serial = Config::get_serial();
        let mut msg_out = Message::new();
        msg_out.set_register_peer(RegisterPeer {
            id,
            serial,
            ..Default::default()
        });
        socket.send(&msg_out, addr.clone()).await?;
        Ok(())
    }

    async fn register_pk(&self, socket: &mut FramedSocket, addr: &TargetAddr<'static>) -> ResultType<()> {
        let id = Config::get_id();
        let (_, pk) = Config::get_key_pair();
        let mut msg_out = Message::new();
        msg_out.set_register_pk(RegisterPk {
            id,
            pk: pk.into(),
            ..Default::default()
        });
        socket.send(&msg_out, addr.clone()).await?;
        log::info!("OHOS register_pk sent for {}", self.host_prefix);
        Ok(())
    }

    async fn handle_resp(
        &mut self,
        msg: Option<rendezvous_message::Union>,
        socket: &mut FramedSocket,
        addr: &TargetAddr<'static>,
        server: &ServerPtr,
    ) -> ResultType<()> {
        match msg {
            Some(rendezvous_message::Union::RegisterPeerResponse(rpr)) => {
                log::info!("OHOS RegisterPeerResponse from {}", self.host_prefix);
                if rpr.request_pk {
                    self.register_pk(socket, addr).await?;
                }
            }
            Some(rendezvous_message::Union::RegisterPkResponse(rpr)) => {
                match rpr.result.enum_value() {
                    Ok(register_pk_response::Result::OK) => {
                        Config::set_key_confirmed(true);
                        Config::set_host_key_confirmed(&self.host_prefix, true);
                        *SOLVING_PK_MISMATCH.lock().await = "".to_owned();
                        NEEDS_DEPLOY.store(false, Ordering::SeqCst);
                        log::info!("OHOS RegisterPk OK for {}", self.host_prefix);
                    }
                    Ok(register_pk_response::Result::NOT_DEPLOYED) => {
                        NEEDS_DEPLOY.store(true, Ordering::SeqCst);
                        Config::set_key_confirmed(false);
                        Config::set_host_key_confirmed(&self.host_prefix, false);
                        log::warn!("OHOS server requires deployment for {}", self.host_prefix);
                    }
                    _ => {
                        log::error!("OHOS unknown RegisterPkResponse");
                    }
                }
                if rpr.keep_alive > 0 {
                    self.keep_alive = rpr.keep_alive * 1000;
                }
            }
            Some(rendezvous_message::Union::PunchHole(ph)) => {
                let rz = self.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    allow_err!(rz.handle_punch_hole(ph, server).await);
                });
            }
            Some(rendezvous_message::Union::RequestRelay(rr)) => {
                let rz = self.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    allow_err!(rz.handle_request_relay(rr, server).await);
                });
            }
            Some(rendezvous_message::Union::ConfigureUpdate(cu)) => {
                Config::set_option("rendezvous-servers".to_owned(), cu.rendezvous_servers.join(","));
                Config::set_serial(cu.serial);
            }
            Some(rendezvous_message::Union::FetchLocalAddr(fla)) => {
                let rz = self.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    allow_err!(rz.handle_intranet(fla, server).await);
                });
            }
            Some(rendezvous_message::Union::TestNatRequest(_)) => {
                let mut msg_out = Message::new();
                msg_out.set_test_nat_response(TestNatResponse {
                    port: 0,
                    ..Default::default()
                });
                socket.send(&msg_out, addr.clone()).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_punch_hole(&self, ph: PunchHole, server: ServerPtr) -> ResultType<()> {
        let peer_addr = hbb_common::AddrMangle::decode(&ph.socket_addr);
        let last = *LAST_MSG.lock().await;
        *LAST_MSG.lock().await = (peer_addr, Instant::now());
        if last.0 == peer_addr && last.1.elapsed().as_millis() < 100 {
            return Ok(());
        }
        log::info!("OHOS received PunchHole from {:?}", peer_addr);
        crate::harmony_bridge::core::queue_event(
            "incoming-connection",
            &format!("PunchHole request from {:?}", peer_addr),
            "",
        );

        let peer_addr_v6 = hbb_common::AddrMangle::decode(&ph.socket_addr_v6);
        let relay = hbb_common::config::Config::is_proxy() || ph.force_relay;
        let relay_server = self.get_relay_server(ph.relay_server);
        let mut socket_addr_v6: Bytes = Default::default();
        if peer_addr_v6.port() > 0 && !relay {
            socket_addr_v6 = start_ipv6(peer_addr_v6, peer_addr, server.clone()).await;
        }

        if ph.nat_type.enum_value() == Ok(hbb_common::protos::rendezvous::NatType::SYMMETRIC)
            || hbb_common::config::Config::get_nat_type()
                == hbb_common::protos::rendezvous::NatType::SYMMETRIC as i32
            || relay
        {
            let uuid = hbb_common::rand::random::<i32>().to_string();
            return self
                .create_relay(
                    ph.socket_addr.into(),
                    relay_server,
                    uuid,
                    server,
                    true,
                    true,
                    socket_addr_v6,
                )
                .await;
        }

        let address_family_mismatch = is_ipv4(&self.addr) != peer_addr.is_ipv4();
        if address_family_mismatch {
            log::info!(
                "OHOS skip direct TCP to {} from {:?} because address families differ; relay_server: {}",
                peer_addr,
                self.addr,
                relay_server
            );
            let uuid = hbb_common::rand::random::<i32>().to_string();
            return self
                .create_relay(
                    ph.socket_addr.into(),
                    relay_server,
                    uuid,
                    server,
                    true,
                    true,
                    socket_addr_v6,
                )
                .await;
        }

        let nat_type = hbb_common::protobuf::Enum::from_i32(
            hbb_common::config::Config::get_nat_type(),
        )
        .unwrap_or(hbb_common::protos::rendezvous::NatType::UNKNOWN_NAT);
        let msg_punch = PunchHoleSent {
            socket_addr: ph.socket_addr,
            id: hbb_common::config::Config::get_id(),
            relay_server,
            nat_type: nat_type.into(),
            version: crate::VERSION.to_owned(),
            socket_addr_v6,
            ..Default::default()
        };

        log::info!("OHOS punch tcp hole to {:?}", peer_addr);
        let mut socket = {
            let socket =
                hbb_common::socket_client::connect_tcp(&*self.host, CONNECT_TIMEOUT).await?;
            let local_addr = socket.local_addr();
            allow_err!(
                hbb_common::socket_client::connect_tcp_local(peer_addr, Some(local_addr), 30).await
            );
            socket
        };
        let mut msg_out = Message::new();
        msg_out.set_punch_hole_sent(msg_punch);
        let bytes = msg_out.write_to_bytes()?;
        socket.send_raw(bytes).await?;
        crate::accept_connection(server, socket, peer_addr, true).await;
        Ok(())
    }

    async fn handle_request_relay(&self, rr: RequestRelay, server: ServerPtr) -> ResultType<()> {
        let peer_addr = hbb_common::AddrMangle::decode(&rr.socket_addr);
        let last = *LAST_RELAY_MSG.lock().await;
        *LAST_RELAY_MSG.lock().await = (peer_addr, Instant::now());
        if last.0 == peer_addr && last.1.elapsed().as_millis() < 100 {
            return Ok(());
        }
        log::info!("OHOS received RequestRelay from {:?}", peer_addr);
        crate::harmony_bridge::core::queue_event(
            "incoming-connection",
            &format!("Relay request from {:?}", peer_addr),
            "",
        );

        self.create_relay(
            rr.socket_addr.into(),
            rr.relay_server,
            rr.uuid,
            server,
            rr.secure,
            false,
            Default::default(),
        )
        .await
    }

    async fn create_relay(
        &self,
        socket_addr: Vec<u8>,
        relay_server: String,
        uuid: String,
        server: ServerPtr,
        secure: bool,
        initiate: bool,
        socket_addr_v6: hbb_common::bytes::Bytes,
    ) -> ResultType<()> {
        let peer_addr = hbb_common::AddrMangle::decode(&socket_addr);
        log::info!(
            "OHOS create_relay requested from {:?}, relay_server: {}, uuid: {}, secure: {}",
            peer_addr,
            relay_server,
            uuid,
            secure,
        );

        let mut socket =
            hbb_common::socket_client::connect_tcp(&*self.host, CONNECT_TIMEOUT).await?;

        let mut msg_out = Message::new();
        let mut rr = RelayResponse {
            socket_addr: socket_addr.into(),
            version: crate::VERSION.to_owned(),
            socket_addr_v6,
            ..Default::default()
        };
        if initiate {
            rr.uuid = uuid.clone();
            rr.relay_server = relay_server.clone();
            rr.set_id(hbb_common::config::Config::get_id());
        }
        msg_out.set_relay_response(rr);
        socket.send(&msg_out).await?;
        crate::create_relay_connection(
            server,
            relay_server,
            uuid,
            peer_addr,
            secure,
            hbb_common::socket_client::is_ipv4(&self.addr),
        )
        .await;
        Ok(())
    }

    fn get_relay_server(&self, provided_by_rendezvous_server: String) -> String {
        let mut relay_server = Config::get_option("relay-server");
        if relay_server.is_empty() {
            relay_server = provided_by_rendezvous_server;
        }
        if relay_server.is_empty() {
            relay_server = crate::increase_port(&self.host, 1);
        }
        relay_server
    }

    async fn handle_intranet(&self, fla: FetchLocalAddr, server: ServerPtr) -> ResultType<()> {
        let addr = hbb_common::AddrMangle::decode(&fla.socket_addr);
        let last = *LAST_MSG.lock().await;
        *LAST_MSG.lock().await = (addr, Instant::now());
        if last.0 == addr && last.1.elapsed().as_millis() < 100 {
            return Ok(());
        }
        log::info!("OHOS received FetchLocalAddr from {:?}", addr);

        let peer_addr_v6 = hbb_common::AddrMangle::decode(&fla.socket_addr_v6);
        let relay_server = self.get_relay_server(fla.relay_server.clone());
        let relay = Config::is_proxy();
        let mut socket_addr_v6: Bytes = Default::default();
        if peer_addr_v6.port() > 0 && !relay {
            socket_addr_v6 = start_ipv6(peer_addr_v6, addr, server.clone()).await;
        }

        let address_family_mismatch = is_ipv4(&self.addr) != addr.is_ipv4();
        if is_ipv4(&self.addr) && !relay && !config::is_disable_tcp_listen() && !address_family_mismatch {
            if let Err(err) = self
                .handle_intranet_(fla.clone(), server.clone(), relay_server.clone(), socket_addr_v6.clone())
                .await
            {
                log::debug!("OHOS Failed to handle intranet: {:?}, will try relay", err);
            } else {
                return Ok(());
            }
        }
        let uuid = hbb_common::rand::random::<i32>().to_string();
        self.create_relay(
            fla.socket_addr.into(),
            relay_server,
            uuid,
            server,
            true,
            true,
            socket_addr_v6,
        )
        .await
    }

    async fn handle_intranet_(
        &self,
        fla: FetchLocalAddr,
        server: ServerPtr,
        relay_server: String,
        socket_addr_v6: Bytes,
    ) -> ResultType<()> {
        let peer_addr = hbb_common::AddrMangle::decode(&fla.socket_addr);
        log::debug!("OHOS Handle intranet from {:?}", peer_addr);
        let mut socket = hbb_common::socket_client::connect_tcp(&*self.host, CONNECT_TIMEOUT).await?;
        let local_addr = socket.local_addr();
        let local_addr: std::net::SocketAddr =
            format!("{}:{}", local_addr.ip(), local_addr.port()).parse()?;
        let mut msg_out = Message::new();
        msg_out.set_local_addr(LocalAddr {
            id: Config::get_id(),
            socket_addr: hbb_common::AddrMangle::encode(peer_addr).into(),
            local_addr: hbb_common::AddrMangle::encode(local_addr).into(),
            relay_server,
            version: crate::VERSION.to_owned(),
            socket_addr_v6,
            ..Default::default()
        });
        let bytes = msg_out.write_to_bytes()?;
        socket.send_raw(bytes).await?;
        crate::accept_connection(server, socket, peer_addr, true).await;
        Ok(())
    }
}

fn new_server() -> ServerPtr {
    Arc::new(RwLock::new(server::Server::new()))
}

fn start_lan_listener_once() {
    if LAN_LISTENER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    hbb_common::log::info!("OHOS rendezvous mediator starting LAN listener");
    std::thread::spawn(move || {
        crate::harmony_bridge::core::queue_event("lan-listener", "starting", "");
        if let Err(err) = crate::lan::start_listening() {
            LAN_LISTENER_STARTED.store(false, Ordering::SeqCst);
            hbb_common::log::error!("LAN listener failed: {}", err);
            crate::harmony_bridge::core::queue_event("lan-listener", &format!("failed: {}", err), "");
        }
    });
}

async fn start_ipv6(
    peer_addr_v6: std::net::SocketAddr,
    peer_addr_v4: std::net::SocketAddr,
    server: ServerPtr,
) -> Bytes {
    crate::test_ipv6().await;
    if let Some((socket, local_addr_v6)) = crate::get_ipv6_socket().await {
        let server = server.clone();
        tokio::spawn(async move {
            allow_err!(udp_nat_listen(socket, peer_addr_v6, peer_addr_v4, server).await);
        });
        return local_addr_v6;
    }
    Default::default()
}

async fn udp_nat_listen(
    socket: Arc<tokio::net::UdpSocket>,
    peer_addr: std::net::SocketAddr,
    peer_addr_v4: std::net::SocketAddr,
    server: ServerPtr,
) -> ResultType<()> {
    let tm = Instant::now();
    let socket_cloned = socket.clone();
    let result: ResultType<()> = async {
        socket.connect(peer_addr).await?;
        let res = crate::punch_udp(socket.clone(), true).await?;
        let stream = crate::kcp_stream::KcpStream::accept(
            socket,
            std::time::Duration::from_millis(CONNECT_TIMEOUT as _),
            res,
        )
        .await?;
        crate::server::create_tcp_connection(server, stream.1, peer_addr_v4, true, None).await?;
        Ok(())
    }
    .await;
    if let Err(e) = result {
        log::error!(
            "OHOS stop listening on {:?} for remote {peer_addr} with KCP, {:?} elapsed: {e}",
            socket_cloned.local_addr(),
            tm.elapsed()
        );
    }
    Ok(())
}

pub(crate) fn reset_needs_deploy_notification() {
    NEEDS_DEPLOY.store(false, Ordering::SeqCst);
}
