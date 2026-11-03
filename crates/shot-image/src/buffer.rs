use std::sync::Arc;

pub const MAX_PIXELS: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    Dimensions,
    Stride,
    BufferTooSmall,
    Allocation,
    Parameter,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Dimensions => "invalid image dimensions or pixel limit exceeded",
            Self::Stride => "invalid image stride",
            Self::BufferTooSmall => "pixel buffer is too small",
            Self::Allocation => "image allocation failed",
            Self::Parameter => "invalid image parameter",
        })
    }
}
impl std::error::Error for ImageError {}

/// Borrowed view into a pixel buffer with stride in u32 units.
#[derive(Clone, Copy)]
pub struct ImageView<'a> {
    pixels: &'a [u32],
    width: i32,
    height: i32,
    stride: usize,
}

impl<'a> ImageView<'a> {
    pub fn new(
        pixels: &'a [u32],
        width: i32,
        height: i32,
        stride: usize,
    ) -> Result<Self, ImageError> {
        pixel_count(width, height)?;
        if stride < width as usize {
            return Err(ImageError::Stride);
        }
        let needed = stride
            .checked_mul((height - 1) as usize)
            .and_then(|n| n.checked_add(width as usize))
            .ok_or(ImageError::Stride)?;
        if needed > pixels.len() {
            return Err(ImageError::BufferTooSmall);
        }
        Ok(Self {
            pixels,
            width,
            height,
            stride,
        })
    }

    pub fn width(self) -> i32 {
        self.width
    }

    pub fn height(self) -> i32 {
        self.height
    }

    pub fn row(self, y: i32) -> &'a [u32] {
        assert!(y >= 0 && y < self.height);
        let start = y as usize * self.stride;
        &self.pixels[start..start + self.width as usize]
    }

    /// Copies pixels into an owned Image, stripping padding.
    pub fn to_image(self) -> Result<Image, ImageError> {
        let mut pixels = allocate(pixel_count(self.width, self.height)?)?;
        for y in 0..self.height {
            let start = y as usize * self.width as usize;
            pixels[start..start + self.width as usize].copy_from_slice(self.row(y));
        }
        Image::from_pixels(self.width, self.height, pixels)
    }
}

/// Immutable shared image buffer holding packed 0xAARRGGBB pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: i32,
    height: i32,
    pixels: Arc<Vec<u32>>,
}

impl Image {
    pub fn from_pixels(width: i32, height: i32, pixels: Vec<u32>) -> Result<Self, ImageError> {
        if pixel_count(width, height)? != pixels.len() {
            return Err(ImageError::BufferTooSmall);
        }
        Ok(Self {
            width,
            height,
            pixels: Arc::new(pixels),
        })
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn view(&self) -> ImageView<'_> {
        ImageView {
            pixels: &self.pixels,
            width: self.width,
            height: self.height,
            stride: self.width as usize,
        }
    }

    pub fn row(&self, y: i32) -> &[u32] {
        self.view().row(y)
    }

    /// Block-tiled coordinate transpose for horizontal scrolling.
    pub fn transpose(&self) -> Result<Self, ImageError> {
        let mut output = allocate(self.pixels.len())?;
        for by in (0..self.height).step_by(32) {
            for bx in (0..self.width).step_by(32) {
                for y in by..(by + 32).min(self.height) {
                    for x in bx..(bx + 32).min(self.width) {
                        output[x as usize * self.height as usize + y as usize] =
                            self.row(y)[x as usize];
                    }
                }
            }
        }
        Self::from_pixels(self.height, self.width, output)
    }
}

pub fn pixel_count(width: i32, height: i32) -> Result<usize, ImageError> {
    if width <= 0 || height <= 0 {
        return Err(ImageError::Dimensions);
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(ImageError::Dimensions)?;
    if count > MAX_PIXELS {
        return Err(ImageError::Dimensions);
    }
    Ok(count)
}

pub fn allocate(count: usize) -> Result<Vec<u32>, ImageError> {
    if count > MAX_PIXELS {
        return Err(ImageError::Dimensions);
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| ImageError::Allocation)?;
    pixels.resize(count, 0);
    Ok(pixels)
}
