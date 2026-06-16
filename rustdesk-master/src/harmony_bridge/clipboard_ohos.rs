use hbb_common::{message_proto::*, ResultType};

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

pub fn handle_msg_clipboard(_cb: Clipboard) {}

pub fn handle_msg_multi_clipboards(_mcb: MultiClipboards) {}

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
