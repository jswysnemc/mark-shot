/// 将预乘 pixel 的全部通道乘以 alpha/255，返回按最近整数舍入的预乘像素。
pub(super) fn scale(pixel: u32, alpha: u32) -> u32 {
    (0..4).fold(0, |result, channel| {
        result | ((((pixel >> (channel * 8)) & 255) * alpha + 127) / 255) << (channel * 8)
    })
}

/// 把预乘 source 覆盖到 destination，返回一次 SourceOver 合成结果。
pub(super) fn over(source: u32, destination: u32) -> u32 {
    let remaining = 255 - (source >> 24);
    let background = scale(destination, remaining);
    (0..4).fold(0, |result, channel| {
        let shift = channel * 8;
        let value = ((source >> shift) & 255) + ((background >> shift) & 255);
        result | value.min(255) << shift
    })
}

/// 将 color 的 RGB 与独立 alpha 合成为预乘像素，返回阴影颜色，忽略 color 自带的透明度。
pub(super) fn shadow_color(color: u32, alpha: u32) -> u32 {
    scale(color | 0xff000000, alpha)
}
