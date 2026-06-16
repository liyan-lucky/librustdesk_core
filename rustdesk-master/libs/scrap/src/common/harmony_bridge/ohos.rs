use crate::{Frame, Pixfmt, TraitCapturer, TraitPixelBuffer};
use std::{
    io,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

pub const IS_CURSOR_EMBEDDED: bool = false;

static INCOMING_FRAME: OnceLock<Mutex<Option<IncomingFrame>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct IncomingFrame {
    frame_id: u64,
    width: usize,
    height: usize,
    stride: usize,
    bytes: Vec<u8>,
    pixfmt: Pixfmt,
}

fn incoming_frame() -> &'static Mutex<Option<IncomingFrame>> {
    INCOMING_FRAME.get_or_init(|| Mutex::new(None))
}

pub fn update_ohos_incoming_frame(
    frame_id: u64,
    width: usize,
    height: usize,
    stride: usize,
    format: &str,
    data: &[u8],
) -> bool {
    if frame_id == 0 || width == 0 || height == 0 || stride < width.saturating_mul(4) {
        return false;
    }
    let expected_len = stride.saturating_mul(height);
    if expected_len == 0 || data.len() < expected_len {
        return false;
    }
    let pixfmt = match format.trim().to_ascii_uppercase().as_str() {
        "BGRA" | "BGRA_8888" => Pixfmt::BGRA,
        _ => Pixfmt::RGBA,
    };
    *incoming_frame().lock().unwrap() = Some(IncomingFrame {
        frame_id,
        width,
        height,
        stride,
        bytes: data[..expected_len].to_vec(),
        pixfmt,
    });
    true
}

pub fn clear_ohos_incoming_frame() {
    *incoming_frame().lock().unwrap() = None;
}

pub fn has_ohos_incoming_frame() -> bool {
    incoming_frame().lock().unwrap().is_some()
}

pub struct Capturer {
    display: Display,
    last_frame_id: u64,
    buffer: Vec<u8>,
    pixfmt: Pixfmt,
    stride: usize,
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        Ok(Capturer {
            display,
            last_frame_id: 0,
            buffer: Vec::new(),
            pixfmt: Pixfmt::RGBA,
            stride: 0,
        })
    }

    pub fn width(&self) -> usize {
        self.display.width()
    }

    pub fn height(&self) -> usize {
        self.display.height()
    }
}

impl TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, timeout: Duration) -> io::Result<Frame<'a>> {
        let started_at = Instant::now();
        loop {
            if let Some(frame) = incoming_frame().lock().unwrap().clone() {
                if frame.frame_id != self.last_frame_id {
                    let width = frame.width;
                    let height = frame.height;
                    let pixfmt = frame.pixfmt;
                    let stride = frame.stride;
                    self.last_frame_id = frame.frame_id;
                    self.display.width = width;
                    self.display.height = height;
                    self.buffer = frame.bytes;
                    self.pixfmt = pixfmt;
                    self.stride = stride;
                    return Ok(Frame::PixelBuffer(PixelBuffer::new_with_stride(
                        &self.buffer,
                        self.pixfmt,
                        width,
                        height,
                        self.stride,
                    )));
                }
            }
            if started_at.elapsed() >= timeout {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "waiting for OHOS incoming screen frame",
                ));
            }
            thread::sleep(Duration::from_millis(8));
        }
    }
}

pub struct PixelBuffer<'a> {
    data: &'a [u8],
    pixfmt: Pixfmt,
    width: usize,
    height: usize,
    stride: Vec<usize>,
}

impl<'a> PixelBuffer<'a> {
    pub fn new(data: &'a [u8], pixfmt: Pixfmt, width: usize, height: usize) -> Self {
        let stride0 = if height == 0 { 0 } else { data.len() / height };
        Self::new_with_stride(data, pixfmt, width, height, stride0)
    }

    pub fn new_with_stride(
        data: &'a [u8],
        pixfmt: Pixfmt,
        width: usize,
        height: usize,
        stride0: usize,
    ) -> Self {
        Self {
            data,
            pixfmt,
            width,
            height,
            stride: vec![stride0],
        }
    }
}

impl<'a> TraitPixelBuffer for PixelBuffer<'a> {
    fn data(&self) -> &[u8] {
        self.data
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn stride(&self) -> Vec<usize> {
        self.stride.clone()
    }

    fn pixfmt(&self) -> Pixfmt {
        self.pixfmt
    }
}

#[derive(Clone)]
pub struct Display {
    width: usize,
    height: usize,
}

impl Display {
    pub fn primary() -> io::Result<Display> {
        if let Some(frame) = incoming_frame().lock().unwrap().clone() {
            return Ok(Display {
                width: frame.width,
                height: frame.height,
            });
        }
        Ok(Display {
            width: 720,
            height: 1280,
        })
    }

    pub fn all() -> io::Result<Vec<Display>> {
        Ok(vec![Self::primary()?])
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn origin(&self) -> (i32, i32) {
        (0, 0)
    }

    pub fn is_online(&self) -> bool {
        true
    }

    pub fn is_primary(&self) -> bool {
        true
    }

    pub fn name(&self) -> String {
        "OHOS".into()
    }

    pub fn logical_width(&self) -> usize {
        self.width()
    }

    pub fn logical_height(&self) -> usize {
        self.height()
    }

    pub fn scale(&self) -> f64 {
        1.0
    }
}
