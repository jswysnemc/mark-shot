//! 【图像导出】【透明编码】使用原版 Qt 的透明通道转换规则编码外框，其他截图编码器保持独立

#[path = "codec_factors.rs"]
mod factors;

/// 【图像导出】【原版编码】消费 image 的预乘像素，使用固定的 Qt 参考系数返回 PNG，不依赖本机近似倒数指令
pub(super) fn encode(mut image: tiny_skia::Pixmap) -> Vec<u8> {
    for pixel in image.data_mut().as_chunks_mut::<4>().0 {
        let alpha = pixel[3];
        if alpha == 255 {
            continue;
        }
        if alpha == 0 {
            pixel.fill(0);
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = (f32::from(*channel) * factors::MULTIPLIERS[usize::from(alpha)])
                .round_ties_even()
                .clamp(0.0, 255.0) as u8;
        }
    }
    crate::utils::encode_rgba(image.data(), image.width(), image.height())
}

#[cfg(test)]
mod tests {
    /// 【图像导出测试】【完整通道矩阵】验证全部合法预乘通道与透明度组合，期望 PNG 由真实 Qt 转换生成，无参数、无返回值
    #[test]
    fn alpha_conversion_matches_all_original_qt_channel_values() {
        let mut input = tiny_skia::Pixmap::new(256, 256).unwrap();
        for (index, pixel) in input
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
        {
            let alpha = (index / 256) as u8;
            let color = if index % 256 <= usize::from(alpha) {
                index as u8
            } else {
                0
            };
            pixel.copy_from_slice(&[color, color, color, alpha]);
        }
        let actual = image::load_from_memory(&super::encode(input))
            .unwrap()
            .into_rgba8();
        let expected = image::load_from_memory(include_bytes!(
            "../../../tests/fixtures/qt-export-image/channel-rounding.png"
        ))
        .unwrap()
        .into_rgba8();
        let difference = actual
            .enumerate_pixels()
            .find(|(x, y, color)| *color != expected.get_pixel(*x, *y));
        assert!(
            difference.is_none(),
            "first channel conversion mismatch: {difference:?}"
        );
    }
}
