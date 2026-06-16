use std::sync::{Arc, Mutex};

use hbb_common::{
    anyhow::{anyhow, Context},
    bytes::Bytes,
    log,
    message_proto::{
        supported_decoding::PreferCodec, video_frame, Chroma, EncodedVideoFrame,
        EncodedVideoFrames, SupportedDecoding, SupportedEncoding, VideoFrame,
    },
    ResultType,
};

use super::{
    vpxcodec::{self, VpxDecoder, VpxDecoderConfig, VpxEncoder, VpxEncoderConfig, VpxVideoCodecId},
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

pub struct Encoder {
    vpx: VpxEncoder,
}

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

fn create_frame(frame: &vpxcodec::EncodeFrame) -> EncodedVideoFrame {
    EncodedVideoFrame {
        data: Bytes::from(frame.data.to_vec()),
        key: frame.key,
        pts: frame.pts,
        ..Default::default()
    }
}

impl Encoder {
    pub fn new(config: EncoderCfg, i444: bool) -> ResultType<Encoder> {
        log::info!("OHOS new encoder: {config:?}, i444: {i444}");
        let vpx = VpxEncoder::new(config, i444)?;
        *ENCODE_CODEC_FORMAT.lock().unwrap() = match vpx.codec_id() {
            VpxVideoCodecId::VP8 => CodecFormat::VP8,
            VpxVideoCodecId::VP9 => CodecFormat::VP9,
        };
        Ok(Encoder { vpx })
    }

    pub fn usable_encoding() -> SupportedEncoding {
        SupportedEncoding {
            vp8: true,
            av1: false,
            h264: false,
            h265: false,
            ..Default::default()
        }
    }

    pub fn update(update: EncodingUpdate) {
        log::info!("OHOS update:{:?}", update);
        let mut decodings = PEER_DECODINGS.lock().unwrap();
        match update {
            EncodingUpdate::Update(id, decoding) => {
                decodings.insert(id, decoding);
            }
            EncodingUpdate::Remove(id) => {
                decodings.remove(&id);
            }
            EncodingUpdate::NewOnlyVP9(id) => {
                decodings.insert(
                    id,
                    SupportedDecoding {
                        ability_vp9: 1,
                        prefer: PreferCodec::VP9.into(),
                        ..Default::default()
                    },
                );
            }
            EncodingUpdate::Check => {}
        }
        let decodings = decodings.clone();
        let mut encoding = Self::supported_encoding();
        let decodable_vp8 = decodings.iter().all(|d| d.1.ability_vp8 > 0);
        if !decodable_vp8 {
            encoding.vp8 = false;
        }
        *USABLE_ENCODING.lock().unwrap() = Some(encoding);
    }

    pub fn supported_encoding() -> SupportedEncoding {
        SupportedEncoding {
            vp8: true,
            av1: false,
            h264: false,
            h265: false,
            ..Default::default()
        }
    }

    pub fn set_bitrate(&mut self, bitrate: u32) {
        let _ = self.vpx.set_bitrate(bitrate);
    }

    pub fn encode_to_message(&mut self, frame: EncodeInput<'_>, ms: i64) -> ResultType<VideoFrame> {
        let mut frames = Vec::new();
        for ref f in self
            .vpx
            .encode(ms, frame.yuv()?, crate::STRIDE_ALIGN)
            .with_context(|| "Failed to encode")?
        {
            frames.push(create_frame(f));
        }
        for ref f in self.vpx.flush().with_context(|| "Failed to flush")? {
            frames.push(create_frame(f));
        }

        if !frames.is_empty() {
            Ok(VpxEncoder::create_video_frame(self.vpx.codec_id(), frames))
        } else {
            Err(anyhow!("no valid frame"))
        }
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
                    Err(anyhow!("vp8 decoder not available on ohos"))
                }
            }
            video_frame::Union::Vp9s(vp9s) => {
                if let Some(vp9) = &mut self.vp9 {
                    Self::handle_vpxs_video_frame(vp9, vp9s, rgb, chroma)
                } else {
                    Err(anyhow!("vp9 decoder not available on ohos"))
                }
            }
            _ => Err(anyhow!("unsupported video frame type on ohos")),
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
                    Err(anyhow!("vp8 decoder not available on ohos"))
                }
            }
            CodecFormat::VP9 => {
                if let Some(vp9) = &mut self.vp9 {
                    Self::handle_vpxs_video_frame(vp9, &frames, rgb, &mut chroma)
                } else {
                    Err(anyhow!("vp9 decoder not available on ohos"))
                }
            }
            _ => Err(anyhow!("unsupported decoder format on ohos")),
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

pub fn base_bitrate(width: u32, height: u32) -> u32 {
    const RESOLUTION_PRESETS: &[(u32, u32, u32)] = &[
        (640, 480, 400),
        (800, 600, 500),
        (1024, 768, 800),
        (1280, 720, 1000),
        (1366, 768, 1100),
        (1440, 900, 1300),
        (1600, 900, 1500),
        (1920, 1080, 2073),
        (2048, 1080, 2200),
        (2560, 1440, 3000),
        (3440, 1440, 4000),
        (3840, 2160, 5000),
        (7680, 4320, 12000),
    ];
    let pixels = width * height;

    let (preset_pixels, preset_bitrate) = RESOLUTION_PRESETS
        .iter()
        .map(|(w, h, bitrate)| (w * h, bitrate))
        .min_by_key(|(preset_pixels, _)| {
            if *preset_pixels >= pixels {
                preset_pixels - pixels
            } else {
                pixels - preset_pixels
            }
        })
        .unwrap_or(((1920 * 1080) as u32, &2073));

    (*preset_bitrate as f32 * (pixels as f32 / preset_pixels as f32)).round() as u32
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

pub const BR_BEST: f32 = 1.5;
pub const BR_BALANCED: f32 = 0.67;
pub const BR_SPEED: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Best,
    Balanced,
    Low,
    Custom(f32),
}

impl Default for Quality {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Quality {
    pub fn is_custom(&self) -> bool {
        match self {
            Quality::Custom(_) => true,
            _ => false,
        }
    }

    pub fn ratio(&self) -> f32 {
        match self {
            Quality::Best => BR_BEST,
            Quality::Balanced => BR_BALANCED,
            Quality::Low => BR_SPEED,
            Quality::Custom(v) => *v,
        }
    }
}
