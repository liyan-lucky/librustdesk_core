use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};

use hbb_common::{
    allow_err,
    config::{self, option2bool, Config, CONNECT_TIMEOUT, RENDEZVOUS_PORT},
    futures::future::join_all,
    log,
    protobuf::Message as _,
    rendezvous_proto::*,
    sleep,
    socket_client::{self, new_udp_for},
    tokio::{self, select, sync::Mutex, time::interval},
    udp::FramedSocket,
    AddrMangle, IntoTargetAddr, ResultType, TargetAddr,
};

use crate::server::{self, ServerPtr};

type Message = RendezvousMessage;

lazy_static::lazy_static! {
    static ref SOLVING_PK_MISMATCH: Mutex<String> = Default::default();
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
            Some(rendezvous_message::Union::RelayResponse(rr)) => {
                let rz = self.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    allow_err!(rz.handle_relay_response(rr, server).await);
                });
            }
            Some(rendezvous_message::Union::ConfigureUpdate(cu)) => {
                Config::set_option("rendezvous-servers".to_owned(), cu.rendezvous_servers.join(","));
                Config::set_serial(cu.serial);
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

    async fn handle_punch_hole(&self, ph: PunchHole, _server: ServerPtr) -> ResultType<()> {
        let addr_str = String::from_utf8_lossy(&ph.socket_addr);
        log::info!("OHOS received PunchHole from {}", addr_str);
        crate::harmony_bridge::core::queue_event(
            "incoming-connection",
            &format!("PunchHole request from {}", addr_str),
            "",
        );
        Ok(())
    }

    async fn handle_request_relay(&self, rr: RequestRelay, _server: ServerPtr) -> ResultType<()> {
        let addr_str = String::from_utf8_lossy(&rr.socket_addr);
        log::info!("OHOS received RequestRelay from {}", addr_str);
        crate::harmony_bridge::core::queue_event(
            "incoming-connection",
            &format!("Relay request from {}", addr_str),
            "",
        );
        Ok(())
    }

    async fn handle_relay_response(&self, rr: RelayResponse, _server: ServerPtr) -> ResultType<()> {
        let addr_str = String::from_utf8_lossy(&rr.socket_addr);
        log::info!("OHOS received RelayResponse from {}", addr_str);
        crate::harmony_bridge::core::queue_event(
            "incoming-connection",
            &format!("Relay response from {}", addr_str),
            "",
        );
        Ok(())
    }
}

fn new_server() -> ServerPtr {
    Arc::new(RwLock::new(server::Server {
        id_count: hbb_common::rand::random::<i32>() % 1000 + 1000,
    }))
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

pub(crate) fn reset_needs_deploy_notification() {
    NEEDS_DEPLOY.store(false, Ordering::SeqCst);
}
