//! 【滤镜测试】【像素辅助】生成确定性预乘原图，并报告首个像素差异

use tiny_skia::Pixmap;

/// 【滤镜测试】【原图生成】返回 width、height 对应的确定性预乘彩色图，透明度在六十四至二百五十五之间变化
pub fn image(width: u32, height: u32) -> Pixmap {
    let mut image = Pixmap::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let alpha = 64 + (x * 7 + y * 11) % 192;
            let color = [
                (x * 13 + y * 19) % 256,
                (x * 23 + y * 3) % 256,
                (x * 5 + y * 29) % 256,
            ];
            let offset = (y as usize * width as usize + x as usize) * 4;
            for (channel, color) in color.into_iter().enumerate() {
                image.data_mut()[offset + channel] = ((color * alpha + 127) / 255) as u8;
            }
            image.data_mut()[offset + 3] = alpha as u8;
        }
    }
    image
}

/// 【滤镜测试】【像素读取】返回 image 中 x、y 的预乘 RGBA，不转换透明度
pub fn pixel(image: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let offset = (y as usize * image.width() as usize + x as usize) * 4;
    image.data()[offset..offset + 4].try_into().unwrap()
}

/// 【滤镜测试】【完整像素比较】验证 actual 与 expected 尺寸和全部通道一致；失败只显示首个差异，不输出整幅图像
pub fn same(actual: &Pixmap, expected: &Pixmap) {
    assert_eq!(
        (actual.width(), actual.height()),
        (expected.width(), expected.height())
    );
    let difference = actual
        .data()
        .iter()
        .zip(expected.data())
        .position(|(actual, expected)| actual != expected);
    if let Some(index) = difference {
        let offset = index / 4 * 4;
        panic!(
            "first differing pixel ({}, {}), channel {}: {:?} != {:?}",
            index / 4 % actual.width() as usize,
            index / 4 / actual.width() as usize,
            index % 4,
            &actual.data()[offset..offset + 4],
            &expected.data()[offset..offset + 4]
        );
    }
}
