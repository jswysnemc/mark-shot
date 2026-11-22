use crate::types::NO_MATCH;
use shot_image::{Image, ImageError, MAX_PIXELS};

pub const BAND_COUNT: usize = 8;
pub type Signature = Vec<[f32; BAND_COUNT]>;
pub type Weights = [f32; BAND_COUNT];
pub const ALL_BANDS: Weights = [1.0; BAND_COUNT];
pub const ROW_MAX_DIFF: f32 = 2.0;

/// 【长图拼接】【采样带边界】按 width、band 返回连续覆盖选区的像素闭区间，小选区允许相邻带共用一个像素
pub fn band_columns(width: i32, band: usize) -> (i32, i32) {
    if width <= 0 {
        return (0, 0);
    }
    let first = (i64::from(width) * band as i64 / BAND_COUNT as i64) as i32;
    let last = (i64::from(width) * (band + 1) as i64 / BAND_COUNT as i64) as i32 - 1;
    (
        first.clamp(0, width - 1),
        last.clamp(first.clamp(0, width - 1), width - 1),
    )
}

/// 【长图拼接】【横向特征】按 frame 返回每行各带的亮度与对比度特征，完整读取带内像素，失败返回分配或尺寸错误
pub fn compute(frame: &Image) -> Result<Signature, ImageError> {
    let rows = frame.height() as usize;
    if (frame.width() as usize + BAND_COUNT)
        .checked_mul(rows)
        .is_none_or(|count| count > MAX_PIXELS)
    {
        return Err(ImageError::Dimensions);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(rows)
        .map_err(|_| ImageError::Allocation)?;
    result.resize(rows, [0.0; BAND_COUNT]);
    for band in 0..BAND_COUNT {
        let (start, end) = band_columns(frame.width(), band);
        let count = f64::from(end - start + 1);
        // 1. 【长图拼接】【完整像素】固定稀疏采样会遗漏细文字，横向统计覆盖带内每个像素
        for (y, row) in result.iter_mut().enumerate() {
            let mut sum = 0_u64;
            let mut squares = 0_u64;
            for &pixel in &frame.row(y as i32)[start as usize..=end as usize] {
                let luminance = u64::from(
                    (299 * ((pixel >> 16) & 255)
                        + 587 * ((pixel >> 8) & 255)
                        + 114 * (pixel & 255))
                        / 1000,
                );
                sum += luminance;
                squares += luminance * luminance;
            }
            // 2. 【长图拼接】【正文对比度】方差补充均值，避免宽空白将窄正文的行变化稀释为零
            let mean = sum as f64 / count;
            let variance = (squares as f64 / count - mean * mean).max(0.0);
            row[band] = (mean - variance.sqrt()) as f32;
        }
    }
    Ok(result)
}

fn scaled_ignore(height: i32, ratio: f32) -> i32 {
    if height < 80 {
        0
    } else {
        (height / 4).min(16.max((height as f32 * ratio) as i32))
    }
}

pub fn top_ignore(height: i32) -> i32 {
    scaled_ignore(height, 0.10)
}

pub fn bottom_ignore(height: i32) -> i32 {
    scaled_ignore(height, 0.08)
}

pub fn side_ignore(width: i32) -> i32 {
    if width <= 0 {
        0
    } else {
        50.max(width / 20)
            .min(width / 3)
            .min(((width - 1) / 2).max(0))
    }
}

pub fn row_diff(a: &[u32], b: &[u32], start: i32, width: i32) -> f32 {
    if start < 0
        || width <= 0
        || (i64::from(start) + i64::from(width)) as usize > a.len().min(b.len())
    {
        return NO_MATCH;
    }
    let mut sum = 0.0;
    let mut count = 0;
    for x in (start as usize..(start + width) as usize).step_by((width / 256).max(1) as usize) {
        for shift in [16, 8, 0] {
            sum += (((a[x] >> shift) & 255) as i32 - ((b[x] >> shift) & 255) as i32).abs() as f32;
            count += 1;
        }
    }
    if count > 0 {
        sum / count as f32
    } else {
        NO_MATCH
    }
}
