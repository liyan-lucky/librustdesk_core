use hbb_common::{message_proto::*, ResultType};

use crate::harmony_bridge::{get_active_peer_id, queue_event};

pub const CLIPBOARD_NAME: &str = "clipboard";
pub const FILE_CLIPBOARD_NAME: &str = "file-clipboard";
pub const CLIPBOARD_INTERVAL: u64 = 333;

#[derive(Clone, Default)]
pub struct ClipboardContext;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ClipboardSide {
    Host,
    Client,
}

impl ClipboardContext {
    pub fn new() -> ResultType<Self> {
        Ok(Self)
    }
}

pub fn check_clipboard(
    _ctx: &mut Option<ClipboardContext>,
    _side: ClipboardSide,
    _force: bool,
) -> Option<Message> {
    None
}

pub fn peek_clipboard(
    _ctx: &mut Option<ClipboardContext>,
    _side: ClipboardSide,
    _force: bool,
) -> Option<Message> {
    None
}

pub fn check_clipboard_files(
    _ctx: &mut Option<ClipboardContext>,
    _side: ClipboardSide,
    _force: bool,
) -> Option<Vec<String>> {
    None
}

pub fn try_empty_clipboard_files(_side: ClipboardSide, _conn_id: i32) {}

pub fn update_clipboard(_clipboards: Vec<Clipboard>, _side: ClipboardSide) {}

pub fn get_current_clipboard_msg(
    _peer_version: &str,
    _peer_platform: &str,
    _side: ClipboardSide,
) -> Option<Message> {
    None
}

pub fn get_msg_if_not_support_multi_clip(
    _version: &str,
    _platform: &str,
    _multi_clipboards: &MultiClipboards,
) -> Option<Message> {
    None
}

fn queue_text_clipboard(mut cb: Clipboard) {
    if cb.format.enum_value() != Ok(ClipboardFormat::Text) {
        queue_event(
            "clipboard-decode-diag",
            &format!("result=ignored-format;format={:?}", cb.format.enum_value()),
            &get_active_peer_id(),
        );
        return;
    }
    let compressed = cb.compress;
    let source_bytes = cb.content.len();
    let content = if cb.compress {
        hbb_common::compress::decompress(&cb.content)
    } else {
        cb.content.to_vec()
    };
    if let Ok(text) = String::from_utf8(content) {
        if !text.is_empty() {
            queue_event(
                "clipboard-decode-diag",
                &format!(
                    "result=text;compress={compressed};sourceBytes={source_bytes};chars={}",
                    text.chars().count()
                ),
                &get_active_peer_id(),
            );
            queue_event("clipboard-incoming", &text, &get_active_peer_id());
        } else {
            queue_event("clipboard-decode-diag", "result=empty", &get_active_peer_id());
        }
    } else {
        queue_event("clipboard-decode-diag", "result=invalid-utf8", &get_active_peer_id());
    }
}

pub fn handle_msg_clipboard(cb: Clipboard) {
    queue_text_clipboard(cb);
}

pub fn handle_msg_multi_clipboards(mcb: MultiClipboards) {
    if let Some(cb) = mcb
        .clipboards
        .into_iter()
        .find(|cb| cb.format.enum_value() == Ok(ClipboardFormat::Text))
    {
        queue_text_clipboard(cb);
    }
}

pub fn get_clipboards_msg(_client: bool) -> Option<Message> {
    None
}

pub fn is_file_url_set_by_rustdesk(_url: &Vec<String>) -> bool {
    false
}

pub fn set_text_clipboard_with_owner_sync(_text: &str, _side: ClipboardSide) -> ResultType<()> {
    Ok(())
}

pub mod clipboard_listener {
    use hbb_common::ResultType;

    pub fn subscribe<T>(_name: String, _tx: T) -> ResultType<()> {
        Ok(())
    }

    pub fn unsubscribe(_name: &str) {}
}

pub mod platform {
    pub mod unix {
        pub mod fuse {
            pub fn init_fuse_context(_client: bool) {}

            pub fn uninit_fuse_context(_client: bool) {}
        }

        pub mod serv_files {
            pub fn sync_files(_urls: &Vec<String>) -> hbb_common::ResultType<()> {
                Ok(())
            }
        }

        pub mod macos {
            pub fn should_handle_msg<T>(_clip: &T) -> bool {
                false
            }
        }
    }
}
