#[cfg(not(target_os = "ios"))]
use hbb_common::whoami;
use hbb_common::{
    allow_err,
    anyhow::bail,
    config::Config,
    config::{self, RENDEZVOUS_PORT},
    log,
    protobuf::Message as _,
    rendezvous_proto::*,
    tokio::{
        self,
        sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    },
    ResultType,
};

use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    time::Instant,
};

type Message = RendezvousMessage;

#[cfg(not(target_os = "ios"))]
pub(super) fn start_listening() -> ResultType<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], get_broadcast_port()));
    let socket = UdpSocket::bind(addr)?;
    socket.set_read_timeout(Some(std::time::Duration::from_millis(1000)))?;
    log::info!("lan discovery listener started");
    loop {
        let mut buf = [0; 2048];
        if let Ok((len, addr)) = socket.recv_from(&mut buf) {
            if let Ok(msg_in) = Message::parse_from_bytes(&buf[0..len]) {
                match msg_in.union {
                    Some(rendezvous_message::Union::PeerDiscovery(p)) => {
                        if p.cmd == "ping"
                            && config::option2bool(
                                "enable-lan-discovery",
                                &Config::get_option("enable-lan-discovery"),
                            )
                        {
                            let id = Config::get_id();
                            if p.id == id {
                                continue;
                            }
                            if let Some(self_addr) = get_ipaddr_by_peer(&addr) {
                                let mut msg_out = Message::new();
                                let mut hostname = crate::whoami_hostname();
                                // The default hostname is "localhost" which is a bit confusing
                                if hostname == "localhost" {
                                    hostname = "unknown".to_owned();
                                }
                                let peer = PeerDiscovery {
                                    cmd: "pong".to_owned(),
                                    mac: get_mac(&self_addr),
                                    id,
                                    hostname,
                                    username: crate::platform::get_active_username(),
                                    platform: whoami::platform().to_string(),
                                    ..Default::default()
                                };
                                msg_out.set_peer_discovery(peer);
                                socket.send_to(&msg_out.write_to_bytes()?, addr).ok();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn discover() -> ResultType<()> {
    let sockets = send_query()?;
    let rx = spawn_wait_responses(sockets);
    handle_received_peers(rx).await?;

    log::info!("discover ping done");
    Ok(())
}

pub fn send_wol(id: String) {
    let interfaces = default_net::get_interfaces();
    for peer in &config::LanPeers::load().peers {
        if peer.id == id {
            for (_, mac) in peer.ip_mac.iter() {
                if let Ok(mac_addr) = mac.parse() {
                    for interface in &interfaces {
                        for ipv4 in &interface.ipv4 {
                            // remove below mask check to avoid unexpected bug
                            // if (u32::from(ipv4.addr) & u32::from(ipv4.netmask)) == (u32::from(peer_ip) & u32::from(ipv4.netmask))
                            log::info!("Send wol to {mac_addr} of {}", ipv4.addr);
                            allow_err!(wol::send_wol(mac_addr, None, Some(IpAddr::V4(ipv4.addr))));
                        }
                    }
                }
            }
            break;
        }
    }
}

#[inline]
fn get_broadcast_port() -> u16 {
    (RENDEZVOUS_PORT + 3) as _
}

#[cfg(target_env = "ohos")]
fn get_ohos_subnet_broadcasts() -> Vec<Ipv4Addr> {
    let mut broadcasts = Vec::new();
    // Read /proc/net/route to get interface IPs and generate subnet broadcasts
    if let Ok(content) = std::fs::read_to_string("/proc/net/route") {
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 {
                // fields[1] = destination, fields[2] = gateway
                // We want the interface's own IP which we can derive from the route
                // Actually, /proc/net/route has: Iface Dest Gateway Flags RefCnt Use Metric Mask MTU Window IRTT
                // Dest 00000000 = default route, Gateway has the router IP
                // We need the local IP - try getting it from the interface
            }
        }
    }
    // Fallback: try to get local IP by connecting to a public address
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:53").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                if let std::net::IpAddr::V4(v4) = local_addr.ip() {
                    let octets = v4.octets();
                    // Assume /24 subnet
                    broadcasts.push(Ipv4Addr::new(octets[0], octets[1], octets[2], 255));
                    log::info!("OHOS LAN: derived subnet broadcast {} from local IP {}", broadcasts.last().unwrap(), v4);
                }
            }
        }
    }
    broadcasts
}

fn get_mac(_ip: &IpAddr) -> String {
    #[cfg(not(target_os = "ios"))]
    if let Ok(mac) = get_mac_by_ip(_ip) {
        mac.to_string()
    } else {
        "".to_owned()
    }
    #[cfg(target_os = "ios")]
    "".to_owned()
}

#[cfg(not(target_os = "ios"))]
fn get_mac_by_ip(ip: &IpAddr) -> ResultType<String> {
    for interface in default_net::get_interfaces() {
        match ip {
            IpAddr::V4(local_ipv4) => {
                if interface.ipv4.iter().any(|x| x.addr == *local_ipv4) {
                    if let Some(mac_addr) = interface.mac_addr {
                        return Ok(mac_addr.address());
                    }
                }
            }
            IpAddr::V6(local_ipv6) => {
                if interface.ipv6.iter().any(|x| x.addr == *local_ipv6) {
                    if let Some(mac_addr) = interface.mac_addr {
                        return Ok(mac_addr.address());
                    }
                }
            }
        }
    }
    bail!("No interface found for ip: {:?}", ip);
}

// Mainly from https://github.com/shellrow/default-net/blob/cf7ca24e7e6e8e566ed32346c9cfddab3f47e2d6/src/interface/shared.rs#L4
fn get_ipaddr_by_peer<A: ToSocketAddrs>(peer: A) -> Option<IpAddr> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect(peer) {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip()),
        Err(_) => return None,
    };
}

fn create_broadcast_sockets() -> Vec<UdpSocket> {
    let mut ipv4s = Vec::new();
    // TODO: maybe we should use a better way to get ipv4 addresses.
    // But currently, it's ok to use `[Ipv4Addr::UNSPECIFIED]` for discovery.
    // `default_net::get_interfaces()` causes undefined symbols error when `flutter build` on iOS simulator x86_64
    #[cfg(not(any(target_os = "ios", target_env = "ohos")))]
    for interface in default_net::get_interfaces() {
        for ipv4 in &interface.ipv4 {
            ipv4s.push(ipv4.addr.clone());
        }
    }
    // On OHOS, default_net::get_interfaces() may return empty or fail.
    // Try to get local IP via UDP connect trick, then bind to both specific IP and 0.0.0.0.
    #[cfg(target_env = "ohos")]
    {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:53").is_ok() {
                if let Ok(local_addr) = socket.local_addr() {
                    if let IpAddr::V4(v4) = local_addr.ip() {
                        if !v4.is_unspecified() {
                            ipv4s.push(v4);
                        }
                    }
                }
            }
        }
        ipv4s.push(Ipv4Addr::UNSPECIFIED);
    }
    ipv4s.push(Ipv4Addr::UNSPECIFIED); // for robustness
    let mut sockets = Vec::new();
    for v4_addr in ipv4s {
        // removing v4_addr.is_private() check, https://github.com/rustdesk/rustdesk/issues/4663
        if let Ok(s) = UdpSocket::bind(SocketAddr::from((v4_addr, 0))) {
            if s.set_broadcast(true).is_ok() {
                sockets.push(s);
            } else if v4_addr.is_unspecified() {
                // On some platforms (OHOS), set_broadcast may fail on 0.0.0.0
                // but broadcast can still work via send_to to broadcast address
                sockets.push(s);
            }
        }
    }
    sockets
}

fn send_query() -> ResultType<Vec<UdpSocket>> {
    let sockets = create_broadcast_sockets();
    if sockets.is_empty() {
        bail!("Found no bindable ipv4 addresses");
    }

    let mut msg_out = Message::new();
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let id = crate::ui_interface::get_id();
    #[cfg(all(not(any(target_os = "android", target_os = "ios")), target_env = "ohos"))]
    let id = config::Config::get_id();
    #[cfg(all(not(any(target_os = "android", target_os = "ios")), not(target_env = "ohos")))]
    let id = "".to_owned();
    let peer = PeerDiscovery {
        cmd: "ping".to_owned(),
        id: id.clone(),
        ..Default::default()
    };
    msg_out.set_peer_discovery(peer);
    let out = msg_out.write_to_bytes()?;
    let maddr = SocketAddr::from(([255, 255, 255, 255], get_broadcast_port()));
    let mut sent_any = false;
    let mut global_ok = false;
    for socket in &sockets {
        if let Ok(n) = socket.send_to(&out, maddr) {
            sent_any = true;
            global_ok = true;
            log::info!("global broadcast sent {} bytes to {}", n, maddr);
        } else {
            log::warn!("global broadcast send_to failed for socket {:?}", socket.local_addr());
        }
    }
    #[cfg(target_env = "ohos")]
    {
        let subnet_broadcasts = get_ohos_subnet_broadcasts();
        let mut subnet_ok_count = 0u32;
        for bcast in &subnet_broadcasts {
            let baddr = SocketAddr::from((*bcast, get_broadcast_port()));
            for socket in &sockets {
                if socket.send_to(&out, baddr).is_ok() {
                    sent_any = true;
                    subnet_ok_count += 1;
                    log::info!("discover ping sent to subnet broadcast {}", bcast);
                }
            }
        }
        let diag = format!(
            "sockets={} global_ok={} subnet_broadcasts={} subnet_ok={} id={} local_addrs={}",
            sockets.len(), global_ok, subnet_broadcasts.len(), subnet_ok_count, id,
            sockets.iter().map(|s| format!("{:?}", s.local_addr())).collect::<Vec<_>>().join(",")
        );
        crate::harmony_bridge::core::queue_event("lan-discover-diag", &diag, "");
    }
    if sent_any {
        log::info!("discover ping sent");
    } else {
        log::warn!("discover ping failed on all sockets");
    }
    Ok(sockets)
}

fn wait_response(
    socket: UdpSocket,
    timeout: Option<std::time::Duration>,
    tx: UnboundedSender<config::DiscoveryPeer>,
) -> ResultType<()> {
    let mut last_recv_time = Instant::now();

    let local_addr = socket.local_addr();
    let try_get_ip_by_peer = match local_addr.as_ref() {
        Err(..) => true,
        Ok(addr) => addr.ip().is_unspecified(),
    };
    let mut mac: Option<String> = None;

    socket.set_read_timeout(timeout)?;
    #[cfg(target_env = "ohos")]
    let mut recv_count = 0u32;
    #[cfg(target_env = "ohos")]
    let mut pong_count = 0u32;
    loop {
        let mut buf = [0; 2048];
        if let Ok((len, addr)) = socket.recv_from(&mut buf) {
            #[cfg(target_env = "ohos")]
            {
                recv_count += 1;
                if recv_count <= 20 {
                    let diag = format!("raw_recv len={} from={} local={:?}", len, addr, socket.local_addr());
                    crate::harmony_bridge::core::queue_event("lan-raw-recv", &diag, "");
                }
            }
            if let Ok(msg_in) = Message::parse_from_bytes(&buf[0..len]) {
                match msg_in.union {
                    Some(rendezvous_message::Union::PeerDiscovery(p)) => {
                        last_recv_time = Instant::now();
                        if p.cmd == "pong" {
                            #[cfg(target_env = "ohos")]
                            {
                                pong_count += 1;
                            }
                            let local_mac = if try_get_ip_by_peer {
                                if let Some(self_addr) = get_ipaddr_by_peer(&addr) {
                                    get_mac(&self_addr)
                                } else {
                                    "".to_owned()
                                }
                            } else {
                                match mac.as_ref() {
                                    Some(m) => m.clone(),
                                    None => {
                                        let m = if let Ok(local_addr) = local_addr {
                                            get_mac(&local_addr.ip())
                                        } else {
                                            "".to_owned()
                                        };
                                        mac = Some(m.clone());
                                        m
                                    }
                                }
                            };

                            let my_id = config::Config::get_id();
                            let accepted = p.id != my_id;
                            #[cfg(target_env = "ohos")]
                            {
                                let diag = format!(
                                    "from={} id={} platform={} my_id={} local_mac={} peer_mac={} accepted={}",
                                    addr, p.id, p.platform, my_id, local_mac, p.mac, accepted
                                );
                                crate::harmony_bridge::core::queue_event("lan-pong-diag", &diag, "");
                            }
                            if accepted {
                                allow_err!(tx.send(config::DiscoveryPeer {
                                    id: p.id.clone(),
                                    ip_mac: HashMap::from([
                                        (addr.ip().to_string(), p.mac.clone(),)
                                    ]),
                                    username: p.username.clone(),
                                    hostname: p.hostname.clone(),
                                    platform: p.platform.clone(),
                                    online: true,
                                }));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if last_recv_time.elapsed().as_millis() > 3_000 {
            #[cfg(target_env = "ohos")]
            {
                let diag = format!("recv_count={} pong_count={}", recv_count, pong_count);
                crate::harmony_bridge::core::queue_event("lan-wait-done", &diag, "");
            }
            break;
        }
    }
    Ok(())
}

fn spawn_wait_responses(sockets: Vec<UdpSocket>) -> UnboundedReceiver<config::DiscoveryPeer> {
    let (tx, rx) = unbounded_channel::<_>();
    for socket in sockets {
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            allow_err!(wait_response(
                socket,
                Some(std::time::Duration::from_millis(10)),
                tx_clone
            ));
        });
    }
    rx
}

async fn handle_received_peers(mut rx: UnboundedReceiver<config::DiscoveryPeer>) -> ResultType<()> {
    let mut peers = config::LanPeers::load().peers;
    peers.iter_mut().for_each(|peer| {
        peer.online = false;
    });

    let mut response_set = HashSet::new();
    let mut last_write_time: Option<Instant> = None;
    loop {
        tokio::select! {
            data = rx.recv() => match data {
                Some(mut peer) => {
                    let in_response_set = !response_set.insert(peer.id.clone());
                    if let Some(pos) = peers.iter().position(|x| x.is_same_peer(&peer) ) {
                        let peer1 = peers.remove(pos);
                        if in_response_set {
                            peer.ip_mac.extend(peer1.ip_mac);
                            peer.online = true;
                        }
                    }
                    peers.insert(0, peer);
                    if last_write_time.map(|t| t.elapsed().as_millis() > 300).unwrap_or(true)  {
                        config::LanPeers::store(&peers);
                        #[cfg(feature = "flutter")]
                        crate::flutter_ffi::main_load_lan_peers();
                        last_write_time = Some(Instant::now());
                    }
                }
                None => {
                    break
                }
            }
        }
    }

    config::LanPeers::store(&peers);
    #[cfg(feature = "flutter")]
    crate::flutter_ffi::main_load_lan_peers();
    #[cfg(target_env = "ohos")]
    {
        let diag = format!("peers_stored={} ids={}", peers.len(), peers.iter().map(|p| p.id.clone()).collect::<Vec<_>>().join(","));
        crate::harmony_bridge::core::queue_event("lan-discover-result", &diag, "");
    }
    Ok(())
}
