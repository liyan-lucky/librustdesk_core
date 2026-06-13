use std::sync::{Arc, Mutex};

use hbb_common::{
    log,
    message_proto::{
        supported_decoding::PreferCodec, video_frame, Chroma, EncodedVideoFrame,
        EncodedVideoFrames, SupportedDecoding, SupportedEncoding, VideoFrame,
    },
    ResultType,
};

use super::{
    vpxcodec::{self, VpxDecoder, VpxDecoderConfig, VpxEncoderConfig, VpxVideoCodecId},
    GoogleImage,
};
use crate::{CodecFormat, EncodeInput, EncodeYuvFormat, ImageRgb, ImageTexture};

lazy_static::lazy_static! {
    static ref PEER_DECODINGS: Arc<Mutex<std::collections::HashMap<i32, SupportedDecoding>>> = Default::default();
    static ref ENCODE_CODEC_FORMAT: Arc<Mutex<CodecFormat>> = Arc::new(Mutex::new(CodecFormat::VP9));
    static ref USABLE_ENCODING: Arc<Mutex<Option<SupportedEncoding>>> = Arc::new(Mutex::new(None));
}

pub const ENCODE_NEED_SWITCH: &'static str = "ENCODE_NEED_SWITCH";

#[derive(Debug, Clone)]
pub enum EncoderCfg {
    VPX(VpxEncoderConfig),
}

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
    vp8: Option<VpxDecoder>,
    vp9: Option<VpxDecoder>,
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
            ability_vp8: 1,
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
        let vp8 = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP8,
        })
        .map_err(|err| {
            log::error!("failed to create OHOS VP8 decoder: {err}");
            err
        })
        .ok();
        let vp9 = VpxDecoder::new(VpxDecoderConfig {
            codec: VpxVideoCodecId::VP9,
        })
        .map_err(|err| {
            log::error!("failed to create OHOS VP9 decoder: {err}");
            err
        })
        .ok();
        let valid = match format {
            CodecFormat::VP8 => vp8.is_some(),
            CodecFormat::VP9 => vp9.is_some(),
            _ => false,
        };
        Decoder {
            format,
            valid,
            vp8,
            vp9,
        }
    }

    pub fn format(&self) -> CodecFormat {
        self.format
    }

    pub fn valid(&self) -> bool {
        self.valid
    }

    pub fn handle_video_frame(
        &mut self,
        frame: &video_frame::Union,
        rgb: &mut ImageRgb,
        _texture: &mut ImageTexture,
        _pixelbuffer: &mut bool,
        chroma: &mut Option<Chroma>,
    ) -> ResultType<bool> {
        match frame {
            video_frame::Union::Vp8s(vp8s) => {
                if let Some(vp8) = &mut self.vp8 {
                    Self::handle_vpxs_video_frame(vp8, vp8s, rgb, chroma)
                } else {
                    Err(hbb_common::anyhow::anyhow!("vp8 decoder not available on ohos"))
                }
            }
            video_frame::Union::Vp9s(vp9s) => {
                if let Some(vp9) = &mut self.vp9 {
                    Self::handle_vpxs_video_frame(vp9, vp9s, rgb, chroma)
                } else {
                    Err(hbb_common::anyhow::anyhow!("vp9 decoder not available on ohos"))
                }
            }
            _ => Err(hbb_common::anyhow::anyhow!(
                "unsupported video frame type on ohos"
            )),
        }
    }

    pub fn decode(&mut self, data: &[u8], rgb: &mut ImageRgb) -> ResultType<bool> {
        let mut frames = EncodedVideoFrames::new();
        frames.frames.push(EncodedVideoFrame {
            data: hbb_common::bytes::Bytes::from(data.to_vec()),
            ..Default::default()
        });
        let mut chroma = None;
        match self.format {
            CodecFormat::VP8 => {
                if let Some(vp8) = &mut self.vp8 {
                    Self::handle_vpxs_video_frame(vp8, &frames, rgb, &mut chroma)
                } else {
                    Err(hbb_common::anyhow::anyhow!("vp8 decoder not available on ohos"))
                }
            }
            CodecFormat::VP9 => {
                if let Some(vp9) = &mut self.vp9 {
                    Self::handle_vpxs_video_frame(vp9, &frames, rgb, &mut chroma)
                } else {
                    Err(hbb_common::anyhow::anyhow!("vp9 decoder not available on ohos"))
                }
            }
            _ => Err(hbb_common::anyhow::anyhow!(
                "unsupported decoder format on ohos"
            )),
        }
    }

    fn handle_vpxs_video_frame(
        decoder: &mut VpxDecoder,
        vpxs: &EncodedVideoFrames,
        rgb: &mut ImageRgb,
        chroma: &mut Option<Chroma>,
    ) -> ResultType<bool> {
        let mut last_frame = vpxcodec::Image::new();
        for vpx in vpxs.frames.iter() {
            for frame in decoder.decode(&vpx.data)? {
                drop(last_frame);
                last_frame = frame;
            }
        }
        for frame in decoder.flush()? {
            drop(last_frame);
            last_frame = frame;
        }
        if last_frame.is_null() {
            Ok(false)
        } else {
            *chroma = Some(last_frame.chroma());
            last_frame.to(rgb);
            Ok(true)
        }
    }
}

pub fn base_bitrate(_w: u32, _h: u32) -> u32 {
    0
}

pub fn codec_thread_num(limit: usize) -> usize {
    std::cmp::max(1, std::cmp::min(limit, 4))
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
