//! 【区域识别】【形态运算】半径一的方形结构元素

use super::Gray;

/// 【区域识别】【邻域运算】按 image 和 dilate 计算三乘三邻域，返回膨胀或腐蚀结果，图像外侧不参与
fn apply(image: &Gray, dilate: bool) -> Gray {
    let (width, height) = (image.width, image.height);
    let mut output = Gray::new(width, height);
    if width == 0 || height == 0 {
        return output;
    }
    for y in 0..height {
        for x in 0..width {
            let mut value = if dilate { 0 } else { 255 };
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                if dilate && value != 0 || !dilate && value == 0 {
                    break;
                }
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    value = if dilate {
                        value.max(image.at(nx, ny))
                    } else {
                        value.min(image.at(nx, ny))
                    };
                }
            }
            output.pixels[y * width + x] = value;
        }
    }
    output
}

/// 【区域识别】【图像膨胀】对 image 执行半径一的最大值运算，返回膨胀图像
pub(in crate::features::region_detection) fn dilate(image: &Gray) -> Gray {
    apply(image, true)
}

/// 【区域识别】【图像腐蚀】对 image 执行半径一的最小值运算，返回腐蚀图像
pub(in crate::features::region_detection) fn erode(image: &Gray) -> Gray {
    apply(image, false)
}

/// 【区域识别】【图像闭合】对 image 先膨胀再腐蚀，返回闭运算结果
pub(in crate::features::region_detection) fn close(image: &Gray) -> Gray {
    erode(&dilate(image))
}
