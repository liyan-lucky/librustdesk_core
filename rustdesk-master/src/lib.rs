#[cfg(not(target_env = "ohos"))]
mod keyboard;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/keyboard_ohos.rs"]
mod keyboard;
/// cbindgen:ignore
#[cfg(not(target_env = "ohos"))]
pub mod platform;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/platform_ohos.rs"]
pub mod platform;
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
pub use platform::{
    clip_cursor, get_cursor, get_cursor_data, get_cursor_pos, get_focused_display,
    set_cursor_pos, start_os_service,
};
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
/// cbindgen:ignore
mod server;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/server_ohos.rs"]
mod server;
#[cfg(not(target_os = "ios"))]
pub use self::server::*;
mod client;
mod lan;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
mod rendezvous_mediator;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/rendezvous_mediator_ohos.rs"]
mod rendezvous_mediator;
#[cfg(not(target_os = "ios"))]
pub use self::rendezvous_mediator::*;
/// cbindgen:ignore
pub mod common;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
pub mod ipc;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/ipc_ohos.rs"]
pub mod ipc;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_env = "ohos",
    feature = "cli",
    feature = "flutter"
)))]
pub mod ui;
mod version;
pub use version::*;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
mod bridge_generated;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter_ffi;
use common::*;
mod auth_2fa;
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
mod clipboard;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/clipboard_ohos.rs"]
mod clipboard;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/clipboard_master_ohos.rs"]
mod clipboard_master;
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos", feature = "cli")))]
pub mod core_main;
mod custom_server;
mod lang;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod port_forward;

#[cfg(all(feature = "flutter", feature = "plugin_framework"))]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod plugin;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod tray;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod whiteboard;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod updater;

#[cfg(not(target_env = "ohos"))]
mod ui_cm_interface;
#[cfg(not(target_env = "ohos"))]
mod ui_interface;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/ui_interface_ohos.rs"]
mod ui_interface;
mod ui_session_interface;

mod hbbs_http;

#[cfg(all(any(target_os = "windows", target_os = "linux", target_os = "macos"), not(target_env = "ohos")))]
pub mod clipboard_file;
#[cfg(target_env = "ohos")]
#[path = "harmony_bridge/clipboard_file_ohos.rs"]
pub mod clipboard_file;

pub mod privacy_mode;

#[cfg(windows)]
pub mod virtual_display_manager;

mod kcp_stream;

pub mod harmony_bridge;
