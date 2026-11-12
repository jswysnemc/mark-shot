//! 【截图场景】【固定遮罩混合】透明前景段使用四通道整数槽，避免重复分支和逐通道除法

const CHANNELS: u64 = 0x00ff00ff00ff00ff;
const ROUND: u64 = 0x0080008000800080;

/// 【截图场景】【通道展开】按 pixel 将 RGBA 分别放入十六位槽，返回没有通道重叠的整数
#[inline]
fn packed(pixel: [u8; 4]) -> u64 {
    let pixel = u64::from(u32::from_le_bytes(pixel));
    (pixel & 0x00ff00ff) | ((pixel & 0xff00ff00) << 24)
}

/// 【截图场景】【固定色覆盖】按 background、target、shade 和 bgra 合成预乘遮罩，返回值为空，两组切片必须等长
pub(super) fn paint(background: &[u8], target: &mut [u8], shade: [u8; 4], bgra: bool) {
    assert_eq!(background.len(), target.len());
    let color = packed(shade);
    let inverse = u64::from(255 - shade[3]);
    for (original, target) in background
        .as_chunks::<4>()
        .0
        .iter()
        .zip(target.as_chunks_mut::<4>().0)
    {
        // 1. 【截图场景】【精确舍入】每个槽最大值为 65153，加上自身高八位仍小于 65536，等同 (通道乘积 + 127) / 255
        let scaled = packed(*original) * inverse + ROUND;
        let scaled = ((scaled + ((scaled >> 8) & CHANNELS)) >> 8) & CHANNELS;
        // 2. 【截图场景】【预乘范围】shade 通道不超过 alpha，覆盖结果最大为 255，各槽相加不会向相邻通道进位
        let mixed = color + scaled;
        let pixel = ((mixed & 0x00ff00ff) | ((mixed >> 24) & 0xff00ff00)) as u32;
        let pixel = if bgra {
            (pixel & 0xff00ff00) | (pixel.rotate_right(16) & 0x00ff00ff)
        } else {
            pixel
        };
        *target = pixel.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【固定遮罩测试】【全部整数值】遍历全部遮罩透明度和原件通道值，两种通道顺序必须逐字节等同现有预乘合成
    #[test]
    fn packed_shade_preserves_every_alpha_and_channel_rounding() {
        let source: Vec<u8> = (0..=255u8)
            .flat_map(|value| {
                [
                    value,
                    255 - value,
                    value.wrapping_mul(7),
                    value.wrapping_mul(17),
                ]
            })
            .collect();
        for alpha in 0..=255 {
            for color in [[0, 0, 0], [94, 234, 212], [255, 19, 127]] {
                let shade = tiny_skia::ColorU8::from_rgba(color[0], color[1], color[2], alpha)
                    .premultiply();
                let shade = [shade.red(), shade.green(), shade.blue(), shade.alpha()];
                for bgra in [false, true] {
                    let mut actual = vec![0; source.len()];
                    paint(&source, &mut actual, shade, bgra);
                    for (original, actual) in source
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(actual.as_chunks::<4>().0)
                    {
                        let expected = super::super::over(shade, *original);
                        let expected = if bgra {
                            [expected[2], expected[1], expected[0], expected[3]]
                        } else {
                            expected
                        };
                        assert_eq!(
                            *actual, expected,
                            "alpha={alpha}, color={color:?}, bgra={bgra}, source={original:?}"
                        );
                    }
                }
            }
        }
    }
}
