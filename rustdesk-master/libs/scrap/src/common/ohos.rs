use crate::{Frame, Pixfmt, TraitCapturer, TraitPixelBuffer};
use std::{io, time::Duration};

pub const IS_CURSOR_EMBEDDED: bool = false;

pub struct Capturer {
    display: Display,
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        Ok(Capturer { display })
    }

    pub fn width(&self) -> usize {
        self.display.width()
    }

    pub fn height(&self) -> usize {
        self.display.height()
    }
}

impl TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, _timeout: Duration) -> io::Result<Frame<'a>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "screen capture is not available on OHOS",
        ))
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
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "display capture is not available on OHOS",
        ))
    }

    pub fn all() -> io::Result<Vec<Display>> {
        Ok(Vec::new())
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
        false
    }

    pub fn is_primary(&self) -> bool {
        false
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
