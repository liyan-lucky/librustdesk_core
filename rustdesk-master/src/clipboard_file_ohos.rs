use hbb_common::message_proto::Message;

#[derive(Clone, Debug)]
pub enum ClipboardFile {
    Files { files: Vec<String> },
    NotifyCallback { id: i32 },
}

pub fn clip_2_msg(_clip: ClipboardFile) -> Message {
    Message::new()
}

pub fn msg_2_clip(_msg: hbb_common::message_proto::Cliprdr) -> Option<ClipboardFile> {
    None
}

pub mod unix_file_clip {
    pub fn is_stopping_allowed() -> bool {
        true
    }
}
