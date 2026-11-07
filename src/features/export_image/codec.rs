//! 【图像导出】【透明编码】使用原版 Qt 的透明通道转换规则编码外框，其他截图编码器保持独立

/// 【图像导出】【倒数近似】返回 alpha 的 SIMD 近似倒数，用一次牛顿迭代恢复原版转换精度
#[cfg(target_arch = "x86_64")]
fn multiplier(alpha: u8) -> f32 {
    use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
    let value = f32::from(alpha);
    // 1. 【图像导出】【指令约束】x86_64 保证 SSE 指令可用，此处只操作寄存器，不访问裸指针或外部内存
    let reciprocal = unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(value))) };
    reciprocal * (2.0 - value * reciprocal) * 255.0
}

/// 【图像导出】【原版编码】消费 image 的预乘像素，返回 PNG；透明通道转换与本机 Qt 的浮点舍入保持一致
pub(super) fn encode(mut image: tiny_skia::Pixmap) -> Vec<u8> {
    #[cfg(target_arch = "x86_64")]
    let factors: [f32; 256] = std::array::from_fn(|alpha| {
        if alpha == 0 {
            0.0
        } else {
            multiplier(alpha as u8)
        }
    });
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
            #[cfg(target_arch = "x86_64")]
            {
                *channel = (f32::from(*channel) * factors[usize::from(alpha)])
                    .round_ties_even()
                    .clamp(0.0, 255.0) as u8;
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                // 2. 【图像导出】【标量回退】其他架构使用 Qt 的整数倒数规则，不要求 SIMD 或新增图形依赖
                let factor = 0x00ff00ff / u32::from(alpha);
                *channel = ((u32::from(*channel) * factor + 0x8000) >> 16).min(255) as u8;
            }
        }
    }
    crate::utils::encode_rgba(image.data(), image.width(), image.height())
}

#[cfg(all(test, target_arch = "x86_64"))]
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
