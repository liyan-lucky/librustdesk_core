use crate::client;
use crate::client::file_trait::FileManager;
use crate::client::{Data, Interface, QualityStatus};
use crate::ui_session_interface::{io_loop, InvokeUiSession, Session};
use hbb_common::config::{self, LanPeers, LocalConfig, PeerConfig};
use hbb_common::message_proto::*;
use hbb_common::rendezvous_proto::ConnType;
use serde_json::json;
use std::collections::HashMap;
use std::os::raw::c_int;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

static CONNECT_STATE: OnceLock<Mutex<ConnectState>> = OnceLock::new();
static LOCAL_OPTIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static LATEST_VIDEO_FRAME: OnceLock<Mutex<Option<VideoFrameState>>> = OnceLock::new();
static ACTIVE_SESSION: OnceLock<Mutex<Option<Session<HarmonyHandler>>>> = OnceLock::new();
static INCOMING_SERVICE_STARTED: OnceLock<Mutex<bool>> = OnceLock::new();

#[derive(Clone, Debug)]
struct VideoFrameState {
    frame_id: u64,
    display: c_int,
    width: usize,
    height: usize,
    stride: usize,
    bytes: Vec<u8>,
    timestamp: i64,
    format: String,
}

#[derive(Clone, Debug, Default)]
struct ConnectState {
    session_stage: String,
    active_peer_id: String,
    status_summary: String,
    detail_message: String,
    last_error: String,
    events: Vec<String>,
}

fn connect_state() -> &'static Mutex<ConnectState> {
    CONNECT_STATE.get_or_init(|| Mutex::new(ConnectState::default()))
}

fn local_options() -> &'static Mutex<HashMap<String, String>> {
    LOCAL_OPTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn latest_video_frame() -> &'static Mutex<Option<VideoFrameState>> {
    LATEST_VIDEO_FRAME.get_or_init(|| Mutex::new(None))
}

fn active_session() -> &'static Mutex<Option<Session<HarmonyHandler>>> {
    ACTIVE_SESSION.get_or_init(|| Mutex::new(None))
}

fn incoming_service_started() -> &'static Mutex<bool> {
    INCOMING_SERVICE_STARTED.get_or_init(|| Mutex::new(false))
}

pub fn get_session_stage() -> String {
    connect_state().lock().unwrap().session_stage.clone()
}

pub fn get_active_peer_id() -> String {
    connect_state().lock().unwrap().active_peer_id.clone()
}

pub fn get_connect_status_summary() -> String {
    connect_state().lock().unwrap().status_summary.clone()
}

pub fn get_connect_detail_message() -> String {
    connect_state().lock().unwrap().detail_message.clone()
}

pub fn get_connect_last_error() -> String {
    connect_state().lock().unwrap().last_error.clone()
}

pub fn drain_connect_events_json() -> String {
    let mut guard = connect_state().lock().unwrap();
    let events: Vec<String> = guard.events.drain(..).collect();
    format!("[{}]", events.join(","))
}

fn update_connect_state(stage: &str, peer_id: &str, summary: &str, detail: &str, error: &str) {
    let mut guard = connect_state().lock().unwrap();
    guard.session_stage = stage.to_owned();
    guard.active_peer_id = peer_id.to_owned();
    guard.status_summary = summary.to_owned();
    guard.detail_message = detail.to_owned();
    guard.last_error = error.to_owned();
}

fn queue_event(kind: &str, detail: &str, peer_id: &str) {
    let event = format!(
        "{{\"kind\":\"{}\",\"detail\":\"{}\",\"peerId\":\"{}\",\"timestamp\":{}}}",
        escape_json(kind),
        escape_json(detail),
        escape_json(peer_id),
        current_timestamp_millis()
    );
    let mut guard = connect_state().lock().unwrap();
    guard.events.push(event);
    if guard.events.len() > 50 {
        let excess = guard.events.len() - 50;
        guard.events.drain(0..excess);
    }
}

fn current_timestamp_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_millis() as i64,
        Err(_) => 0,
    }
}

fn next_bridge_job_id() -> i32 {
    (current_timestamp_millis() & 0x7fff_ffff) as i32
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Returns a JSON snapshot of the core state for the given server.
pub fn get_core_snapshot_json(server: &str) -> String {
    let incoming_ready = *incoming_service_started().lock().unwrap();
    json!({
        "adapter": "official-native",
        "coreReady": true,
        "incomingReady": incoming_ready,
        "displayId": get_local_option("id"),
        "fingerprint": "",
        "directAddress": "",
        "server": server,
        "statusSummary": if incoming_ready { "Incoming service requested" } else { "Official Harmony bridge ready" },
        "detailMessage": if incoming_ready {
            "Harmony bridge applied incoming service options. Desktop server thread launch is disabled on Harmony to avoid appspawn exit."
        } else {
            "Official Harmony bridge is initialized."
        },
        "lastError": "",
        "sessionStage": get_session_stage(),
        "activePeerId": get_active_peer_id(),
    })
    .to_string()
}

/// Initializes the runtime with the given app directory and custom client config.
/// Returns a JSON string with initialization result.
pub fn initialize_runtime(app_dir: &str, _custom_client_config: &str) -> String {
    set_local_option("app_dir", app_dir);
    if !app_dir.trim().is_empty() {
        *config::APP_DIR.write().unwrap() = app_dir.trim().to_owned();
    }
    "{}".to_owned()
}

/// Pulls pending session events as a JSON string.
pub fn pull_session_events_json() -> String {
    drain_connect_events_json()
}

/// Pulls pending audio frames as a JSON string.
pub fn pull_audio_frames_json() -> String {
    "[]".to_owned()
}

/// Returns the latest video frame metadata as JSON since the given frame ID.
pub fn get_latest_video_frame_metadata_json(since_frame_id: u64) -> String {
    let guard = latest_video_frame().lock().unwrap();
    let Some(frame) = guard.as_ref() else {
        return "{}".to_owned();
    };
    if frame.frame_id <= since_frame_id {
        return "{}".to_owned();
    }
    json!({
        "frameId": frame.frame_id,
        "display": frame.display,
        "width": frame.width,
        "height": frame.height,
        "stride": frame.stride,
        "bytes": frame.bytes.len(),
        "timestamp": frame.timestamp,
        "format": frame.format,
    })
    .to_string()
}

/// Copies the latest video frame data into the provided buffer.
/// Returns the number of bytes written, or 0 on failure.
pub fn copy_latest_video_frame(frame_id: u64, buffer: &mut [u8]) -> c_int {
    let guard = latest_video_frame().lock().unwrap();
    let Some(frame) = guard.as_ref() else {
        return 0;
    };
    if frame.frame_id != frame_id || buffer.len() < frame.bytes.len() {
        return 0;
    }
    buffer[..frame.bytes.len()].copy_from_slice(&frame.bytes);
    frame.bytes.len() as c_int
}

/// Refreshes the session video for the given display.
/// Returns true if the refresh was successful.
pub fn refresh_session_video(display: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        return false;
    };
    let display = display.max(0);
    session.request_init_msgs(display as usize);
    session.refresh_video(display);
    queue_event(
        "video-refresh-requested",
        &format!("display={display}"),
        &get_active_peer_id(),
    );
    true
}

/// Advances to the next RGBA frame for the given display index.
pub fn harmony_next_rgba(display: usize) {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        return;
    };
    session.ui_handler.next_rgba(display);
}

/// Enables or disables the incoming service with the given server configuration.
/// Returns a JSON string with the result.
pub fn set_incoming_service_enabled(
    enabled: bool,
    server: &str,
    relay_server: &str,
    api_server: &str,
) -> String {
    apply_server_options(server, relay_server, api_server);

    if enabled {
        config::Config::set_option("stop-service".to_owned(), "Y".to_owned());
        *incoming_service_started().lock().unwrap() = false;
        crate::common::set_server_running(false);
        crate::RendezvousMediator::restart();
        let detail = "Harmony incoming service is unavailable because the desktop server and screen capture pipeline are not wired on this target.";
        queue_event(
            "incoming-service-unavailable",
            detail,
            "",
        );
        json!({
            "adapter": "official-native",
            "coreReady": true,
            "incomingReady": false,
            "displayId": get_local_option("id"),
            "fingerprint": "",
            "directAddress": "",
            "server": server,
            "statusSummary": "Incoming service unavailable",
            "detailMessage": detail,
            "lastError": detail,
            "sessionStage": get_session_stage(),
            "activePeerId": get_active_peer_id(),
        })
        .to_string()
    } else {
        config::Config::set_option("stop-service".to_owned(), "Y".to_owned());
        crate::common::set_server_running(false);
        crate::RendezvousMediator::restart();
        *incoming_service_started().lock().unwrap() = false;
        json!({
            "adapter": "official-native",
            "coreReady": true,
            "incomingReady": false,
            "displayId": get_local_option("id"),
            "fingerprint": "",
            "directAddress": "",
            "server": server,
            "statusSummary": "Incoming service stopped",
            "detailMessage": "Incoming service stop has been requested by the Harmony bridge.",
            "lastError": "",
            "sessionStage": get_session_stage(),
            "activePeerId": get_active_peer_id(),
        })
        .to_string()
    }
}

/// Bootstraps a core snapshot with the given connection parameters.
/// Returns a JSON string with the result.
pub fn bootstrap_core_snapshot(
    _display_id: &str,
    _fingerprint: &str,
    _direct_address: &str,
    _server: &str,
) -> String {
    "{}".to_owned()
}

/// Sends mouse input with the given mask and coordinates.
/// Returns true if the input was sent successfully.
pub fn send_mouse_input(mask: c_int, x: c_int, y: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("mouse-input", "failed=no-active-session", "");
        return false;
    };
    session.send_mouse(mask as i32, x as i32, y as i32, false, false, false, false);
    queue_event(
        "mouse-input",
        &format!("mask={mask};x={x};y={y}"),
        &get_active_peer_id(),
    );
    true
}

/// Sends keyboard input with the given key code, press state, and modifiers.
/// Returns true if the input was sent successfully.
pub fn send_keyboard_input(key_code: c_int, is_pressed: bool, modifiers: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event(
            "keyboard-input",
            &format!("failed=no-active-session;key={key_code};down={is_pressed}"),
            "",
        );
        return false;
    };
    let Some(name) = key_code_to_official_key_name(key_code) else {
        queue_event(
            "keyboard-input",
            &format!("failed=unsupported-key;key={key_code};down={is_pressed}"),
            &get_active_peer_id(),
        );
        return false;
    };
    let ctrl = modifiers & 1 != 0;
    let alt = modifiers & 2 != 0;
    let shift = modifiers & 4 != 0;
    let command = modifiers & 8 != 0;
    session.input_key(&name, is_pressed, false, alt, ctrl, shift, command);
    queue_event(
        "keyboard-input",
        &format!("key={key_code};name={name};down={is_pressed};modifiers={modifiers}"),
        &get_active_peer_id(),
    );
    true
}

fn key_code_to_official_key_name(key_code: c_int) -> Option<String> {
    let name = match key_code {
        8 => "VK_BACK",
        9 => "VK_TAB",
        13 => "VK_RETURN",
        16 => "VK_SHIFT",
        17 => "VK_CONTROL",
        18 => "VK_MENU",
        20 => "VK_CAPITAL",
        27 => "VK_ESCAPE",
        32 => "VK_SPACE",
        33 => "VK_PRIOR",
        34 => "VK_NEXT",
        35 => "VK_END",
        36 => "VK_HOME",
        37 => "VK_LEFT",
        38 => "VK_UP",
        39 => "VK_RIGHT",
        40 => "VK_DOWN",
        44 => "VK_SNAPSHOT",
        45 => "VK_INSERT",
        46 => "VK_DELETE",
        91 => "Meta",
        92 => "RWin",
        93 => "Apps",
        112 => "VK_F1",
        113 => "VK_F2",
        114 => "VK_F3",
        115 => "VK_F4",
        116 => "VK_F5",
        117 => "VK_F6",
        118 => "VK_F7",
        119 => "VK_F8",
        120 => "VK_F9",
        121 => "VK_F10",
        122 => "VK_F11",
        123 => "VK_F12",
        _ => "",
    };
    if !name.is_empty() {
        return Some(name.to_owned());
    }
    if (48..=57).contains(&key_code) {
        return Some(format!("VK_{}", (key_code as u8) as char));
    }
    if (65..=90).contains(&key_code) {
        return Some(format!("VK_{}", (key_code as u8) as char));
    }
    if (32..=126).contains(&key_code) {
        return Some((key_code as u8 as char).to_string());
    }
    None
}

/// Sends the official RustDesk Ctrl+Alt+Del control key event.
/// Returns true if an active session accepted the event.
pub fn send_ctrl_alt_del() -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("keyboard-input", "failed=no-active-session; command=ctrl-alt-del", "");
        return false;
    };
    session.ctrl_alt_del();
    queue_event(
        "keyboard-input",
        "command=ctrl-alt-del",
        &get_active_peer_id(),
    );
    true
}

/// Sends clipboard data with the given content and timestamp.
/// Returns true if the data was sent successfully.
pub fn send_clipboard_data(_content: &str, _timestamp: i64) -> bool {
    false
}

/// Sends video frame metadata.
/// Returns true if the metadata was sent successfully.
pub fn send_video_frame_metadata(
    _codec: c_int,
    _width: c_int,
    _height: c_int,
    _timestamp: i64,
    _key_frame: bool,
    _data_length: c_int,
) -> bool {
    false
}

/// Sends audio frame metadata.
/// Returns true if the metadata was sent successfully.
pub fn send_audio_frame_metadata(
    _codec: c_int,
    _sample_rate: c_int,
    _channels: c_int,
    _timestamp: i64,
    _data_length: c_int,
) -> bool {
    false
}

/// Sends a chat message with the given content.
/// Returns true if the message was sent successfully.
pub fn send_chat_message(content: &str) -> bool {
    let normalized = content.trim();
    if normalized.is_empty() {
        queue_event("chat-message", "failed=empty-content", &get_active_peer_id());
        return false;
    }
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("chat-message", "failed=no-active-session", "");
        return false;
    };
    session.send_chat(normalized.to_owned());
    queue_event("chat-message", normalized, &get_active_peer_id());
    true
}

/// Sends a file transfer request.
/// Returns true if the request was sent successfully.
pub fn send_file_transfer_request(
    task_id: &str,
    file_name: &str,
    total_bytes: i64,
    direction: &str,
) -> bool {
    queue_event(
        "file-transfer-request",
        &json!({
            "taskId": task_id,
            "fileName": file_name,
            "totalBytes": total_bytes,
            "direction": direction
        })
        .to_string(),
        &get_active_peer_id(),
    );
    active_session().lock().unwrap().is_some()
}

/// Connects to a peer with the given connection parameters.
pub fn connect_to_peer(
    peer_id: &str,
    password: &str,
    server: &str,
    relay_server: &str,
    api_server: &str,
) {
    *latest_video_frame().lock().unwrap() = None;
    apply_server_options(server, relay_server, api_server);

    update_connect_state(
        "connecting",
        peer_id,
        &format!("Connecting to {peer_id}"),
        "Connecting to remote peer",
        "",
    );
    queue_event(
        "connecting",
        &format!(
            "Connecting to {}; server={}; relay={}; api={}",
            peer_id, server, relay_server, api_server
        ),
        peer_id,
    );

    let session = Session::<HarmonyHandler> {
        password: password.to_owned(),
        server_keyboard_enabled: Arc::new(RwLock::new(true)),
        server_file_transfer_enabled: Arc::new(RwLock::new(true)),
        server_clipboard_enabled: Arc::new(RwLock::new(true)),
        reconnect_count: Arc::new(AtomicUsize::new(0)),
        ui_handler: HarmonyHandler,
        ..Default::default()
    };
    session.lc.write().unwrap().initialize(
        peer_id.to_owned(),
        ConnType::DEFAULT_CONN,
        None,
        false,
        None,
        None,
        None,
    );
    let round = session.connection_round_state.lock().unwrap().new_round();
    *active_session().lock().unwrap() = Some(session.clone());
    std::thread::spawn(move || {
        io_loop(session, round);
    });
}

fn apply_server_options(server: &str, relay_server: &str, api_server: &str) {
    if !server.trim().is_empty() {
        config::Config::set_option(
            "custom-rendezvous-server".to_owned(),
            server.trim().to_owned(),
        );
    }
    if !relay_server.trim().is_empty() {
        config::Config::set_option("relay-server".to_owned(), relay_server.trim().to_owned());
    }
    if !api_server.trim().is_empty() {
        config::Config::set_option("api-server".to_owned(), api_server.trim().to_owned());
    }
}

/// Performs account authentication with the given parameters.
pub fn account_auth(
    _op: &str,
    _remember_me: bool,
    _server: &str,
    _relay_server: &str,
    _api_server: &str,
) {
}

/// Cancels an in-progress account authentication.
pub fn account_auth_cancel() {}

/// Returns the account authentication result as a JSON string.
pub fn account_auth_result_json() -> String {
    "{}".to_owned()
}

/// Returns the value of a local option by key.
pub fn get_local_option(key: &str) -> String {
    if key == "access_token" || key == "user_info" || key == "lang" {
        let value = LocalConfig::get_option(key);
        if !value.trim().is_empty() {
            return value;
        }
    }
    local_options()
        .lock()
        .unwrap()
        .get(key)
        .cloned()
        .unwrap_or_default()
}

/// Returns the value of a saved peer option by key.
pub fn get_peer_option(peer_id: &str, key: &str) -> String {
    let config = PeerConfig::load(peer_id);
    config.options.get(key).cloned().unwrap_or_default()
}

/// Returns saved peer display information as JSON.
pub fn get_peer_info(peer_id: &str) -> String {
    let config = PeerConfig::load(peer_id);
    let info = &config.info;
    let alias = config
        .options
        .get("alias")
        .map(String::as_str)
        .unwrap_or_default();

    json!({
        "hostname": info.hostname,
        "username": info.username,
        "platform": info.platform,
        "alias": alias,
    })
    .to_string()
}

fn read_cached_peer_info(peer_id: &str) -> (String, String, String, String) {
    let config = PeerConfig::load(peer_id);
    let info = config.info;
    let alias = config.options.get("alias").cloned().unwrap_or_default();
    (info.hostname, info.username, info.platform, alias)
}

fn peer_info_detail(peer_id: &str) -> String {
    let (hostname, username, platform, alias) = read_cached_peer_info(peer_id);
    let resolved_platform = if platform.trim().is_empty() {
        "RustDesk".to_owned()
    } else {
        platform
    };
    format!(
        "Session connected; hostname={}; username={}; platform={}; alias={}",
        hostname, username, resolved_platform, alias
    )
}

#[derive(Clone, Default)]
struct HarmonyHandler;

impl InvokeUiSession for HarmonyHandler {
    fn set_cursor_data(&self, _cd: CursorData) {}
    fn set_cursor_id(&self, _id: String) {}
    fn set_cursor_position(&self, _cp: CursorPosition) {}
    fn set_display(&self, _x: i32, _y: i32, _w: i32, _h: i32, _cursor_embedded: bool, _scale: f64) {
    }
    fn switch_display(&self, _display: &SwitchDisplay) {}

    fn set_peer_info(&self, peer_info: &PeerInfo) {
        let peer_id = get_active_peer_id();
        if let Some(session) = active_session().lock().unwrap().as_ref().cloned() {
            session.lc.write().unwrap().handle_peer_info(peer_info);
        }
        let detail = format!(
            "hostname={}; username={}; platform={}; version={}",
            peer_info.hostname, peer_info.username, peer_info.platform, peer_info.version
        );
        queue_event("peer-info", &detail, &peer_id);
        if get_session_stage() != "connected" {
            update_connect_state("connected", &peer_id, "Connected", &detail, "");
        } else {
            update_connect_state("connected", &peer_id, "Connected", &detail, "");
        }
    }

    fn set_displays(&self, _displays: &Vec<DisplayInfo>) {}
    fn set_platform_additions(&self, _data: &str) {}

    fn on_connected(&self, conn_type: ConnType) {
        let peer_id = get_active_peer_id();
        let detail = format!("Connected; connType={conn_type:?}");
        update_connect_state("connected", &peer_id, "Connected", &detail, "");
        queue_event("session-connected", &detail, &peer_id);
        if let Some(session) = active_session().lock().unwrap().as_ref().cloned() {
            session.request_init_msgs(0);
            session.refresh_video(0);
        }
    }

    fn update_privacy_mode(&self) {}
    fn set_permission(&self, _name: &str, _value: bool) {}

    fn close_success(&self) {
        let peer_id = get_active_peer_id();
        update_connect_state(
            "connected",
            &peer_id,
            "Connected",
            "Connection handshake completed",
            "",
        );
        queue_event(
            "connection-ready",
            "Connection handshake completed",
            &peer_id,
        );
    }

    fn update_quality_status(&self, qs: QualityStatus) {
        let detail = json!({
            "speed": qs.speed,
            "fps": qs.fps,
            "delay": qs.delay,
            "target_bitrate": qs.target_bitrate,
            "codec_format": qs.codec_format.map(|it| it.to_string()),
            "chroma": qs.chroma,
        })
        .to_string();
        queue_event("quality-status", &detail, &get_active_peer_id());
    }

    fn set_connection_type(&self, is_secured: bool, direct: bool, stream_type: &str) {
        let peer_id = get_active_peer_id();
        queue_event(
            "connection-type",
            &format!("secured={is_secured}; direct={direct}; stream={stream_type}"),
            &peer_id,
        );
    }

    fn set_fingerprint(&self, fingerprint: String) {
        queue_event("fingerprint", &fingerprint, &get_active_peer_id());
    }

    fn job_error(&self, _id: i32, _err: String, _file_num: i32) {}
    fn job_done(&self, _id: i32, _file_num: i32) {}
    fn job_progress(&self, _id: i32, _file_num: i32, _speed: f64, _finished_size: f64) {}
    fn clear_all_jobs(&self) {}
    fn new_message(&self, msg: String) {
        queue_event("chat-message", &msg, &get_active_peer_id());
    }
    fn update_transfer_list(&self) {}
    fn load_last_job(&self, _cnt: i32, _job_json: &str, _auto_start: bool) {}
    fn update_folder_files(
        &self,
        _id: i32,
        _entries: &Vec<FileEntry>,
        _path: String,
        _is_local: bool,
        _only_count: bool,
    ) {
    }
    fn confirm_delete_files(&self, _id: i32, _i: i32, _name: String) {}
    fn override_file_confirm(
        &self,
        _id: i32,
        _file_num: i32,
        _to: String,
        _is_upload: bool,
        _is_identical: bool,
    ) {
    }
    fn update_block_input_state(&self, _on: bool) {}

    fn adapt_size(&self) {}

    fn on_rgba(&self, display: usize, rgba: &mut scrap::ImageRgb) {
        publish_real_video_frame(display as c_int, rgba);
        queue_event(
            "video-frame",
            "Remote video frame received",
            &get_active_peer_id(),
        );
    }

    fn msgbox(&self, msgtype: &str, title: &str, text: &str, link: &str, retry: bool) {
        let peer_id = get_active_peer_id();
        let detail =
            format!("type={msgtype}; title={title}; text={text}; link={link}; retry={retry}");
        if msgtype == "error" {
            update_connect_state("error", &peer_id, title, &detail, text);
            queue_event("session-error", &detail, &peer_id);
            if retry {
                queue_event("msgbox", &detail, &peer_id);
            }
        } else {
            queue_event("msgbox", &detail, &peer_id);
        }
    }

    fn cancel_msgbox(&self, tag: &str) {
        queue_event("cancel-msgbox", tag, &get_active_peer_id());
    }
    fn switch_back(&self, _id: &str) {}
    fn portable_service_running(&self, _running: bool) {}
    fn on_voice_call_started(&self) {}
    fn on_voice_call_closed(&self, reason: &str) {
        queue_event("voice-call-closed", reason, &get_active_peer_id());
    }
    fn on_voice_call_waiting(&self) {}
    fn on_voice_call_incoming(&self) {}
    fn get_rgba(&self, _display: usize) -> *const u8 {
        std::ptr::null()
    }
    fn next_rgba(&self, _display: usize) {}
    fn set_multiple_windows_session(&self, _sessions: Vec<WindowsSession>) {}
    fn set_current_display(&self, _disp_idx: i32) {}
    fn update_record_status(&self, _start: bool) {}
    fn printer_request(&self, _id: i32, _path: String) {}
    fn handle_screenshot_resp(&self, _sid: String, _msg: String) {}
    fn handle_terminal_response(&self, response: TerminalResponse) {
        use hbb_common::message_proto::terminal_response::Union;

        let peer_id = get_active_peer_id();
        match response.union {
            Some(Union::Opened(opened)) => {
                let detail = json!({
                    "type": "opened",
                    "terminal_id": opened.terminal_id,
                    "success": opened.success,
                    "message": opened.message,
                    "pid": opened.pid,
                    "service_id": opened.service_id,
                    "persistent_sessions": opened.persistent_sessions,
                    "replay_terminal_output": opened.replay_terminal_output,
                })
                .to_string();
                queue_event("terminal-response", &detail, &peer_id);
                queue_event(
                    if opened.success {
                        "terminal-opened"
                    } else {
                        "terminal-error"
                    },
                    &detail,
                    &peer_id,
                );
            }
            Some(Union::Data(data)) => {
                let output_data = if data.compressed {
                    hbb_common::compress::decompress(&data.data)
                } else {
                    data.data.to_vec()
                };
                let detail = json!({
                    "type": "data",
                    "terminal_id": data.terminal_id,
                    "dataBase64": crate::encode64(&output_data),
                    "compressed": false,
                })
                .to_string();
                queue_event("terminal-response", &detail, &peer_id);
                queue_event("terminal-output", &detail, &peer_id);
            }
            Some(Union::Closed(closed)) => {
                let detail = json!({
                    "type": "closed",
                    "terminal_id": closed.terminal_id,
                    "exit_code": closed.exit_code,
                })
                .to_string();
                queue_event("terminal-response", &detail, &peer_id);
                queue_event("terminal-closed", &detail, &peer_id);
            }
            Some(Union::Error(error)) => {
                let detail = json!({
                    "type": "error",
                    "terminal_id": error.terminal_id,
                    "message": error.message,
                })
                .to_string();
                queue_event("terminal-response", &detail, &peer_id);
                queue_event("terminal-error", &detail, &peer_id);
            }
            None => {}
            Some(_) => {
                queue_event("terminal-error", "{\"type\":\"unknown\"}", &peer_id);
            }
        }
    }
}

fn mark_peer_connected_with_cached_info(peer_id: &str) {
    let detail = peer_info_detail(peer_id);
    update_connect_state("connected", peer_id, "Connected", &detail, "");
    queue_event("session-connected", &detail, peer_id);
    queue_event("peer-info", &detail, peer_id);
}

/// Returns the boolean value of a session toggle option by key.
pub fn get_session_toggle_option(key: &str) -> bool {
    if let Some(session) = active_session().lock().unwrap().as_ref().cloned() {
        let value = session.get_option(key.to_owned());
        return option_is_enabled(&value);
    }
    option_is_enabled(&get_local_option(key))
}

/// Sets a local option value by key.
pub fn set_local_option(key: &str, value: &str) {
    local_options()
        .lock()
        .unwrap()
        .insert(key.to_owned(), value.to_owned());
    if key == "access_token" || key == "user_info" || key == "lang" {
        LocalConfig::set_option(key.to_owned(), value.to_owned());
        return;
    }
    if key == "temporary-password" {
        *hbb_common::password_security::TEMPORARY_PASSWORD
            .write()
            .unwrap() = value.to_owned();
        return;
    }
    if key == "enable-lan-discovery"
        || key == "verification-method"
        || key == "approve-mode"
        || key.starts_with("custom-")
        || key == "rendezvous-servers"
    {
        config::Config::set_option(key.to_owned(), value.to_owned());
    }
}

/// Applies a session option with the given key and value.
/// Returns true if the option was applied successfully.
pub fn apply_session_option(key: &str, value: &str) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        set_local_option(key, value);
        queue_event(
            "session-option",
            &format!("queued-locally;key={key};value={value}"),
            "",
        );
        return false;
    };
    let applied = match key {
        "image-quality" => {
            session.save_image_quality(value.to_owned());
            true
        }
        "custom-image-quality" => match value.parse::<i32>() {
            Ok(v) => {
                session.save_custom_image_quality(v);
                true
            }
            Err(_) => false,
        },
        "custom-fps" => match value.parse::<i32>() {
            Ok(v) => {
                session.set_custom_fps(v);
                true
            }
            Err(_) => false,
        },
        "record-session" => {
            session.record_screen(option_is_enabled(value));
            true
        }
        "take-screenshot" => {
            session.take_screenshot(0, format!("harmony-{}", current_timestamp_millis()));
            true
        }
        "switch-sides" => false,
        "session-action" => false,
        _ => {
            session.set_option(key.to_owned(), value.to_owned());
            true
        }
    };
    if applied {
        set_local_option(key, value);
        queue_event(
            "session-option",
            &format!("key={key};value={value}"),
            &get_active_peer_id(),
        );
    } else {
        queue_event(
            "session-option",
            &format!("failed=unsupported;key={key};value={value}"),
            &get_active_peer_id(),
        );
    }
    applied
}

fn option_is_enabled(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == "y" || normalized == "yes" || normalized == "true" || normalized == "1"
}

/// Marks the session as connected for the given peer.
pub fn mark_session_connected(peer_id: &str) {
    mark_peer_connected_with_cached_info(peer_id);
}

/// Reconnects the active official session.
pub fn reconnect_session(force_relay: bool) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        return false;
    };
    let peer_id = get_active_peer_id();
    if peer_id.trim().is_empty() {
        return false;
    }
    let detail = if force_relay {
        "Reconnecting through relay"
    } else {
        "Retrying connection"
    };
    update_connect_state("connecting", &peer_id, "Connecting", detail, "");
    queue_event(
        if force_relay { "reconnect-relay" } else { "reconnect" },
        detail,
        &peer_id,
    );
    session.reconnect(force_relay);
    true
}

/// Marks the session with an error message.
pub fn mark_session_error(message: &str) {
    let peer_id = get_active_peer_id();
    update_connect_state("error", &peer_id, message, message, message);
    queue_event("error", message, &peer_id);
}

/// Closes the current session.
pub fn close_session() {
    let peer_id = get_active_peer_id();
    if let Some(session) = active_session().lock().unwrap().as_ref() {
        session.send(Data::Close);
    }
    *active_session().lock().unwrap() = None;
    update_connect_state("idle", "", "Session closed", "Session closed by user", "");
    queue_event("session-closed", "Session closed by user", &peer_id);
}

/// Submits a session password for authentication.
pub fn submit_session_password(password: &str, _remember: bool) -> bool {
    let peer_id = get_active_peer_id();
    if peer_id.is_empty() || password.is_empty() {
        return false;
    }
    update_connect_state(
        "login",
        &peer_id,
        "Password submitted; logging in",
        "Password submitted; logging in",
        "",
    );
    queue_event("login", "Password submitted; logging in", &peer_id);
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        return false;
    };
    session.send(Data::Login((
        String::new(),
        String::new(),
        password.to_owned(),
        _remember,
    )));
    true
}

fn publish_real_video_frame(display: c_int, rgba: &scrap::ImageRgb) {
    if rgba.w == 0 || rgba.h == 0 || rgba.raw.is_empty() {
        return;
    }
    let mut guard = latest_video_frame().lock().unwrap();
    let next_frame_id = guard.as_ref().map(|frame| frame.frame_id + 1).unwrap_or(1);
    let format = match rgba.fmt {
        scrap::ImageFormat::ABGR => "abgr",
        scrap::ImageFormat::ARGB => "argb",
        scrap::ImageFormat::Raw => "bgra",
    }
    .to_owned();
    let stride = if rgba.align > 0 {
        rgba.align
    } else {
        rgba.w * 4
    };
    *guard = Some(VideoFrameState {
        frame_id: next_frame_id,
        display,
        width: rgba.w,
        height: rgba.h,
        stride,
        bytes: rgba.raw.clone(),
        timestamp: current_timestamp_millis(),
        format,
    });
}

/// Restarts the remote device.
pub fn restart_remote_device() -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("session-command", "failed=no-active-session; command=restart", "");
        return false;
    };
    session.restart_remote_device();
    queue_event(
        "session-command",
        "command=restart",
        &get_active_peer_id(),
    );
    true
}

/// Locks the remote screen.
pub fn lock_remote_screen() -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("session-command", "failed=no-active-session; command=lock-screen", "");
        return false;
    };
    session.lock_screen();
    queue_event(
        "session-command",
        "command=lock-screen",
        &get_active_peer_id(),
    );
    true
}

/// Opens a terminal with the given ID and dimensions.
/// Returns true if the terminal was opened successfully.
pub fn open_terminal(terminal_id: c_int, rows: c_int, cols: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event(
            "terminal-error",
            &json!({"type":"error","terminal_id":terminal_id,"message":"no active session"})
                .to_string(),
            "",
        );
        return false;
    };
    let normalized_rows = if rows > 0 { rows as u32 } else { 24 };
    let normalized_cols = if cols > 0 { cols as u32 } else { 80 };
    session.open_terminal(terminal_id, normalized_rows, normalized_cols);
    queue_event(
        "terminal-response",
        &json!({
            "type":"open-requested",
            "terminal_id":terminal_id,
            "rows":normalized_rows,
            "cols":normalized_cols
        })
        .to_string(),
        &get_active_peer_id(),
    );
    true
}

/// Sends input data to the terminal with the given ID.
/// Returns true if the input was sent successfully.
pub fn send_terminal_input(terminal_id: c_int, data: &str) -> bool {
    if data.is_empty() {
        return false;
    }
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event(
            "terminal-error",
            &json!({"type":"error","terminal_id":terminal_id,"message":"no active session"})
                .to_string(),
            "",
        );
        return false;
    };
    session.send_terminal_input(terminal_id, data.to_owned());
    true
}

/// Resizes the terminal with the given ID to the specified dimensions.
/// Returns true if the resize was successful.
pub fn resize_terminal(terminal_id: c_int, rows: c_int, cols: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event(
            "terminal-error",
            &json!({"type":"error","terminal_id":terminal_id,"message":"no active session"})
                .to_string(),
            "",
        );
        return false;
    };
    let normalized_rows = if rows > 0 { rows as u32 } else { 24 };
    let normalized_cols = if cols > 0 { cols as u32 } else { 80 };
    session.resize_terminal(terminal_id, normalized_rows, normalized_cols);
    true
}

/// Closes the terminal with the given ID.
/// Returns true if the terminal was closed successfully.
pub fn close_terminal(terminal_id: c_int) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event(
            "terminal-error",
            &json!({"type":"error","terminal_id":terminal_id,"message":"no active session"})
                .to_string(),
            "",
        );
        return false;
    };
    session.close_terminal(terminal_id);
    true
}

/// Reads the remote directory at the given path.
/// Returns true if the read was initiated successfully.
pub fn read_remote_directory(path: &str, include_hidden: bool) -> bool {
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("file-transfer", "failed=no-active-session; action=read-dir", "");
        return false;
    };
    let normalized = if path.trim().is_empty() { "/" } else { path.trim() };
    session.read_remote_dir(normalized.to_owned(), include_hidden);
    queue_event(
        "file-transfer",
        &json!({"action":"read-dir","path":normalized,"includeHidden":include_hidden}).to_string(),
        &get_active_peer_id(),
    );
    true
}

/// Creates a remote directory at the given path.
/// Returns true if the directory was created successfully.
pub fn create_remote_directory(path: &str) -> bool {
    let normalized = path.trim();
    if normalized.is_empty() {
        queue_event("file-transfer", "failed=empty-path; action=create-dir", &get_active_peer_id());
        return false;
    }
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("file-transfer", "failed=no-active-session; action=create-dir", "");
        return false;
    };
    session.create_dir(next_bridge_job_id(), normalized.to_owned(), true);
    queue_event(
        "file-transfer",
        &json!({"action":"create-dir","path":normalized}).to_string(),
        &get_active_peer_id(),
    );
    true
}

/// Deletes a remote path (file or directory).
/// Returns true if the deletion was successful.
pub fn delete_remote_path(path: &str, is_directory: bool) -> bool {
    let normalized = path.trim();
    if normalized.is_empty() {
        queue_event("file-transfer", "failed=empty-path; action=delete", &get_active_peer_id());
        return false;
    }
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("file-transfer", "failed=no-active-session; action=delete", "");
        return false;
    };
    let job_id = next_bridge_job_id();
    if is_directory {
        session.remove_dir_all(job_id, normalized.to_owned(), true, false);
    } else {
        session.remove_file(job_id, normalized.to_owned(), 0, true);
    }
    queue_event(
        "file-transfer",
        &json!({"action":"delete","path":normalized,"isDirectory":is_directory}).to_string(),
        &get_active_peer_id(),
    );
    true
}

/// Starts a file transfer task.
/// Returns true if the task was started successfully.
pub fn start_file_transfer(path: &str, to: &str, is_remote: bool) -> bool {
    let normalized_path = path.trim();
    let normalized_to = to.trim();
    if normalized_path.is_empty() || normalized_to.is_empty() {
        queue_event("file-transfer", "failed=empty-path; action=start", &get_active_peer_id());
        return false;
    }
    let Some(session) = active_session().lock().unwrap().as_ref().cloned() else {
        queue_event("file-transfer", "failed=no-active-session; action=start", "");
        return false;
    };
    session.send_files(
        next_bridge_job_id(),
        FileType::File as i32,
        normalized_path.to_owned(),
        normalized_to.to_owned(),
        1,
        false,
        is_remote,
    );
    queue_event(
        "file-transfer",
        &json!({"action":"start","path":normalized_path,"to":normalized_to,"isRemote":is_remote}).to_string(),
        &get_active_peer_id(),
    );
    true
}

/// Queries online status for the given peer ID JSON payload.
/// Returns true if the query was submitted successfully.
pub fn query_onlines(ids_json: &str) -> bool {
    let Ok(ids) = serde_json::from_str::<Vec<String>>(ids_json) else {
        queue_event(
            "query-onlines-result",
            "{\"onlines\":[],\"offlines\":[]}",
            "",
        );
        return false;
    };
    std::thread::spawn(move || {
        let ids_for_error = ids.clone();
        let Ok(rt) = hbb_common::tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            queue_event(
                "query-onlines-result",
                &json!({"onlines":[],"offlines":ids_for_error}).to_string(),
                "",
            );
            return;
        };
        rt.block_on(client::peer_online::query_online_states(
            ids,
            |onlines: Vec<String>, offlines: Vec<String>| {
                queue_event(
                    "query-onlines-result",
                    &json!({"onlines":onlines,"offlines":offlines}).to_string(),
                    "",
                );
            },
        ));
    });
    true
}

/// Starts LAN peer discovery.
pub fn discover_lan_peers() {
    std::thread::spawn(|| {
        if let Err(err) = crate::lan::discover() {
            queue_event(
                "lan-discovery-error",
                &format!("LAN discovery failed: {err}"),
                "",
            );
        } else {
            queue_event("lan-discovery-done", "LAN discovery completed", "");
        }
    });
}

/// Loads discovered LAN peers as JSON.
pub fn load_lan_peers() -> String {
    let peers = LanPeers::load().peers;
    json!(peers
        .into_iter()
        .map(|peer| json!({
            "id": peer.id,
            "username": peer.username,
            "hostname": peer.hostname,
            "platform": peer.platform,
            "online": peer.online,
            "ipMac": peer.ip_mac,
        }))
        .collect::<Vec<_>>())
    .to_string()
}

/// Removes a discovered peer from the LAN list.
/// Returns true if a peer was removed.
pub fn remove_discovered_peer(peer_id: &str) -> bool {
    let mut peers = LanPeers::load().peers;
    let before = peers.len();
    peers.retain(|peer| peer.id != peer_id);
    if peers.len() == before {
        return false;
    }
    LanPeers::store(&peers);
    true
}
