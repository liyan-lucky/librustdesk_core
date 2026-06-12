use std::sync::{Arc, Mutex};

use hbb_common::{
    log,
    message_proto::{
        supported_decoding::PreferCodec, Chroma, EncodedVideoFrame, SupportedDecoding,
        SupportedEncoding, VideoFrame,
    },
    ResultType,
};

use crate::{CodecFormat, EncodeInput, EncodeYuvFormat, ImageRgb, ImageTexture};

lazy_static::lazy_static! {
    static ref PEER_DECODINGS: Arc<Mutex<std::collections::HashMap<i32, SupportedDecoding>>> = Default::default();
    static ref ENCODE_CODEC_FORMAT: Arc<Mutex<CodecFormat>> = Arc::new(Mutex::new(CodecFormat::VP9));
    static ref USABLE_ENCODING: Arc<Mutex<Option<SupportedEncoding>>> = Arc::new(Mutex::new(None));
}

pub const ENCODE_NEED_SWITCH: &'static str = "ENCODE_NEED_SWITCH";

#[derive(Debug, Clone)]
pub enum EncoderCfg {}

pub trait EncoderApi {
    fn new(cfg: EncoderCfg, i444: bool) -> ResultType<Self>
    where
        Self: Sized;
    fn encode_to_message(&mut self, frame: EncodeInput<'_>, ms: i64) -> ResultType<VideoFrame>;
    fn yuvfmt(&self) -> EncodeYuvFormat;
    fn set_quality(&mut self, ratio: f32) -> ResultType<()>;
    fn bitrate(&self) -> u32;
    fn support_changing_quality(&self) -> bool;
    fn latency_free(&self) -> bool;
    fn is_hardware(&self) -> bool;
    fn disable(&self);
}

pub struct Encoder;

pub struct Decoder {
    format: CodecFormat,
    valid: bool,
}

#[derive(Debug, Clone)]
pub enum EncodingUpdate {
    Update(i32, SupportedDecoding),
    Remove(i32),
    NewOnlyVP9(i32),
    Check,
}

impl Encoder {
    pub fn usable_encoding() -> SupportedEncoding {
        SupportedEncoding {
            vp8: true,
            av1: false,
            h264: false,
            h265: false,
            ..Default::default()
        }
    }

    pub fn update(_update: EncodingUpdate) {}

    pub fn supported_encoding() -> SupportedEncoding {
        SupportedEncoding {
            vp8: true,
            av1: false,
            h264: false,
            h265: false,
            ..Default::default()
        }
    }

    pub fn set_bitrate(&mut self, _bitrate: u32) {}

    pub fn encode_to_message(
        &mut self,
        _frame: EncodeInput<'_>,
        _ms: i64,
    ) -> ResultType<VideoFrame> {
        Err(hbb_common::anyhow::anyhow!(
            "encoding not supported on ohos"
        ))
    }
}

impl Decoder {
    pub fn supported_decodings(
        _id: Option<&str>,
        _use_texture_render: bool,
        _luid: Option<i64>,
        mark_unsupported: &[CodecFormat],
    ) -> SupportedDecoding {
        let mut decoding = SupportedDecoding {
            ability_vp8: 0,
            ability_vp9: 1,
            ability_av1: 0,
            ability_h264: 0,
            ability_h265: 0,
            prefer: PreferCodec::VP9.into(),
            ..Default::default()
        };
        for unsupported in mark_unsupported {
            match unsupported {
                CodecFormat::VP8 => decoding.ability_vp8 = 0,
                CodecFormat::VP9 => decoding.ability_vp9 = 0,
                CodecFormat::AV1 => decoding.ability_av1 = 0,
                CodecFormat::H264 => decoding.ability_h264 = 0,
                CodecFormat::H265 => decoding.ability_h265 = 0,
                _ => {}
            }
        }
        decoding
    }

    pub fn new(format: CodecFormat, _luid: Option<i64>) -> Decoder {
        log::info!("try create new decoder on ohos, format: {format:?}");
        let valid = format == CodecFormat::VP9;
        Decoder { format, valid }
    }

    pub fn format(&self) -> CodecFormat {
        self.format
    }

    pub fn valid(&self) -> bool {
        self.valid
    }

    pub fn handle_video_frame(
        &mut self,
        _frame: &hbb_common::message_proto::video_frame::Union,
        _rgb: &mut ImageRgb,
        _texture: &mut ImageTexture,
        _pixelbuffer: &mut bool,
        _chroma: &mut Option<Chroma>,
    ) -> ResultType<bool> {
        if !self.valid {
            return Err(hbb_common::anyhow::anyhow!("decoder not valid on ohos"));
        }
        Err(hbb_common::anyhow::anyhow!(
            "video decoding not fully supported on ohos"
        ))
    }

    pub fn decode(&mut self, data: &[u8], rgb: &mut ImageRgb) -> ResultType<bool> {
        if !self.valid {
            return Err(hbb_common::anyhow::anyhow!("decoder not valid on ohos"));
        }
        Err(hbb_common::anyhow::anyhow!(
            "decoding not fully supported on ohos"
        ))
    }
}

pub fn base_bitrate(_w: usize, _h: usize) -> u32 {
    0
}

pub fn codec_thread_num() -> usize {
    1
}

pub fn enable_hwcodec_option() -> bool {
    false
}

pub fn enable_directx_capture() -> bool {
    false
}

pub fn test_av1() {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Quality {
    #[default]
    Balance,
    Best,
    Speed,
}

pub const BR_BALANCED: u32 = 0;
pub const BR_BEST: u32 = 0;
pub const BR_SPEED: u32 = 0;
