pub mod wayland {
    pub fn init() {}

    pub fn common_get_error() -> String {
        String::new()
    }
}

pub mod input_service {
    pub fn fix_key_down_timeout_at_exit() {}
}

pub mod audio_service {
    pub const NAME: &str = "audio";

    pub fn set_voice_call_input_device(_device: Option<String>, _set_if_present: bool) {}
}

pub type Sender = hbb_common::tokio::sync::mpsc::UnboundedSender<(
    std::time::Instant,
    std::sync::Arc<hbb_common::message_proto::Message>,
)>;

#[derive(Clone, Default)]
pub struct ConnInner {
    id: i32,
    tx: Option<Sender>,
    tx_video: Option<Sender>,
}

impl ConnInner {
    pub fn new(id: i32, tx: Option<Sender>, tx_video: Option<Sender>) -> Self {
        Self { id, tx, tx_video }
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}

type ConnMap = std::collections::HashMap<i32, ConnInner>;

pub struct Server {
    connections: ConnMap,
    id_count: i32,
}

pub type ServerPtr = std::sync::Arc<std::sync::RwLock<Server>>;
pub type ServerPtrWeak = std::sync::Weak<std::sync::RwLock<Server>>;

impl Server {
    pub fn get_new_id(&mut self) -> i32 {
        self.id_count += 1;
        self.id_count
    }

    pub fn subscribe(&mut self, _name: &str, _conn: ConnInner, _sub: bool) {}

    pub fn add_connection(&mut self, conn: ConnInner) {
        self.connections.insert(conn.id(), conn);
    }

    pub fn remove_connection(&mut self, conn: &ConnInner) {
        self.connections.remove(&conn.id());
    }
}

lazy_static::lazy_static! {
    pub static ref CLIENT_SERVER: ServerPtr =
        std::sync::Arc::new(std::sync::RwLock::new(Server::new()));
}

impl Server {
    pub fn new() -> Self {
        Server {
            connections: ConnMap::new(),
            id_count: hbb_common::rand::random::<i32>() % 1000 + 1000,
        }
    }
}

#[derive(serde_derive::Serialize)]
pub struct Connection;

impl Connection {
    pub fn alive_conns() -> Vec<Connection> {
        Vec::new()
    }
}

pub const CLICK_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
pub const MOUSE_MOVE_TIME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

pub fn check_zombie() {}

pub async fn start_server(is_server: bool, _no_server: bool) {
    if is_server {
        crate::common::set_server_running(true);
        hbb_common::log::info!("OHOS server starting: set_server_running(true), starting RendezvousMediator");
        crate::harmony_bridge::core::queue_event(
            "server-starting",
            "OHOS incoming server starting",
            "",
        );
        crate::RendezvousMediator::start_all().await;
    } else {
        hbb_common::log::info!("OHOS server not starting (is_server=false)");
    }
}

pub async fn start_ipc_url_server() {}

pub async fn accept_connection(
    server: ServerPtr,
    socket: hbb_common::stream::Stream,
    peer_addr: std::net::SocketAddr,
    secure: bool,
) {
    hbb_common::log::info!("OHOS accept_connection from {}, secure: {}", peer_addr, secure);
    crate::harmony_bridge::core::queue_event(
        "accept-connection",
        &format!("from {} secure={}", peer_addr, secure),
        "",
    );
    let id = server.write().unwrap().get_new_id();
    hbb_common::log::info!("OHOS assigned connection id: {}", id);
}

pub async fn create_relay_connection(
    server: ServerPtr,
    relay_server: String,
    uuid: String,
    peer_addr: std::net::SocketAddr,
    secure: bool,
    ipv4: bool,
) {
    hbb_common::log::info!(
        "OHOS create_relay_connection to relay={}, peer={:?}, uuid={}, secure={}, ipv4={}",
        relay_server, peer_addr, uuid, secure, ipv4
    );
    crate::harmony_bridge::core::queue_event(
        "relay-connection",
        &format!("relay={} peer={:?}", relay_server, peer_addr),
        "",
    );
}
