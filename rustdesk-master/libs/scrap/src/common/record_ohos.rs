use crate::CodecFormat;
use hbb_common::ResultType;
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone)]
pub struct RecorderContext {
    pub server: bool,
    pub id: String,
    pub dir: String,
    pub display_idx: usize,
    pub camera: bool,
    pub tx: Option<Sender<RecordState>>,
}

#[derive(Debug, Clone)]
pub struct RecorderContext2 {
    pub filename: String,
    pub width: usize,
    pub height: usize,
    pub format: CodecFormat,
}

pub trait RecorderApi {
    fn new(ctx: RecorderContext, ctx2: RecorderContext2) -> ResultType<Self>
    where
        Self: Sized;
    fn write_video(&mut self, frame: &hbb_common::message_proto::EncodedVideoFrame) -> bool;
}

#[derive(Debug)]
pub enum RecordState {
    NewFile(String),
    NewFrame,
    WriteTail,
    RemoveFile,
}

pub struct Recorder {
    pub inner: Option<Box<dyn RecorderApi>>,
}

unsafe impl Send for Recorder {}
unsafe impl Sync for Recorder {}

impl Deref for Recorder {
    type Target = Option<Box<dyn RecorderApi>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Recorder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self { inner: None }
    }
}

impl Recorder {
    pub fn new(_ctx: RecorderContext) -> ResultType<Self> {
        Ok(Self { inner: None })
    }

    pub fn write_frame(
        &mut self,
        _frame: &hbb_common::message_proto::video_frame::Union,
        _w: usize,
        _h: usize,
    ) -> ResultType<()> {
        Ok(())
    }
}
