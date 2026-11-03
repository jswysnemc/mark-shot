//! 【尺子】【刻度布局】使用 1、2、5 间隔控制数字密度，刻度数量不随原图像素总量增长

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick {
    pub pixel: u32,
    pub position: f32,
    pub major: bool,
}

/// 【尺子】【主刻度间隔】依据 pixel_length、paint_length 和 minimum_gap 返回 1、2、5 系列像素间隔
pub fn major_step(pixel_length: f32, paint_length: f32, minimum_gap: f32) -> u32 {
    if !pixel_length.is_finite()
        || !paint_length.is_finite()
        || pixel_length <= 0.0
        || paint_length <= 0.0
    {
        return 1;
    }
    let required = (pixel_length / 10.0)
        .max(minimum_gap.max(1.0) * pixel_length / paint_length)
        .max(1.0);
    let magnitude = 10.0_f32.powf(required.log10().floor());
    let mantissa = required / magnitude;
    let nice = if mantissa <= 1.0 {
        magnitude
    } else if mantissa <= 2.0 {
        magnitude * 2.0
    } else if mantissa <= 5.0 {
        magnitude * 5.0
    } else {
        magnitude * 10.0
    };
    (nice.ceil() as u32).max(1)
}

/// 【尺子】【刻度序列】按原图 pixel_length、当前 paint_length 和 minimum_gap 生成有限刻度；零长度返回空列表
pub fn ticks(pixel_length: f32, paint_length: f32, minimum_gap: f32) -> Vec<Tick> {
    if !pixel_length.is_finite()
        || !paint_length.is_finite()
        || pixel_length <= 0.0
        || paint_length <= 0.0
    {
        return Vec::new();
    }
    let major = major_step(pixel_length, paint_length, minimum_gap);
    let minor = (major / 5).max(1);
    let end = (pixel_length.floor() as u32).min(u32::MAX - 1);
    let count = (end / minor).min(256);
    (0..=count)
        .map(|index| {
            let pixel = index.saturating_mul(minor);
            Tick {
                pixel,
                position: pixel as f32 * paint_length / pixel_length,
                major: pixel.is_multiple_of(major),
            }
        })
        .collect()
}
