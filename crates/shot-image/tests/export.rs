use shot_image::{
    Image, ImageView,
    export::{FrameOptions, Layout, render},
};

/// 返回测试外框参数，无参数，默认启用带阴影的圆角外框。
fn options() -> FrameOptions {
    FrameOptions {
        enabled: true,
        padding: 8,
        corner_radius: 4.0,
        shadow_radius: 6,
        shadow_offset_y: 3,
        shadow_opacity: 0.45,
        shadow_color: 0xff000000,
    }
}

/// 按 width/height 和 pixel 创建预乘纯色图像，返回测试输入。
fn solid(width: i32, height: i32, pixel: u32) -> Image {
    Image::from_pixels(width, height, vec![pixel; width as usize * height as usize]).unwrap()
}

#[test]
/// 验证关闭外框时保持全部像素并去除输入行填充，无参数、无返回值。
fn disabled_frame_copies_pixels_without_stride_padding() {
    let source = [0xff123456, 0x80402010, 0, 0, 0xff654321, 0x40080402];
    let view = ImageView::new(&source, 2, 2, 4).unwrap();
    let result = render(
        view,
        &FrameOptions {
            enabled: false,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(
        result.pixels(),
        &[source[0], source[1], source[4], source[5]]
    );
}

#[test]
/// 验证外框布局只改变边距，不缩放原图、不重复叠加半透明内容，无参数、无返回值。
fn translucent_content_is_composited_exactly_once() {
    let source = solid(16, 10, 0x80402010);
    let options = FrameOptions {
        corner_radius: 0.0,
        padding: 50,
        shadow_offset_y: 35,
        ..options()
    };
    let result = render(source.view(), &options).unwrap();
    assert_eq!((result.width(), result.height()), (116, 110));
    for y in 0..source.height() {
        assert_eq!(&result.row(y + 50)[50..66], source.row(y));
    }
}

#[test]
/// 验证两像素捕获边缘会在圆角模式剔除，中心保持原色，无参数、无返回值。
fn rounded_mask_removes_window_border_without_changing_center() {
    let source = solid(34, 24, 0xff123456);
    let options = FrameOptions {
        padding: 12,
        corner_radius: 8.0,
        shadow_opacity: 0.0,
        ..options()
    };
    let result = render(source.view(), &options).unwrap();
    assert_eq!(result.row(24)[13], 0);
    assert_eq!(result.row(13)[29], 0);
    assert_eq!(result.row(24)[29], 0xff123456);
    assert_eq!(result.pixels()[0], 0);
}

#[test]
/// 验证阴影在最后裁切，边距变化不改变相同物理位置的颜色，无参数、无返回值。
fn shadow_is_blurred_before_canvas_clipping() {
    let source = solid(30, 20, 0x80402010);
    let large = FrameOptions {
        padding: 40,
        shadow_offset_y: -5,
        ..options()
    };
    let small = FrameOptions {
        padding: 0,
        ..large
    };
    let large = render(source.view(), &large).unwrap();
    let small = render(source.view(), &small).unwrap();
    for y in 0..source.height() {
        assert_eq!(&large.row(y + 40)[40..70], small.row(y));
    }
}

#[test]
/// 验证透明输入不会生成阴影，输出始终满足预乘通道约束，无参数、无返回值。
fn transparency_and_premultiplied_colors_are_preserved() {
    let transparent = solid(20, 12, 0);
    assert!(
        render(transparent.view(), &options())
            .unwrap()
            .pixels()
            .iter()
            .all(|pixel| *pixel == 0)
    );
    let source = solid(20, 12, 0x80402010);
    let result = render(
        source.view(),
        &FrameOptions {
            shadow_color: 0xff80c040,
            ..options()
        },
    )
    .unwrap();
    for pixel in result.pixels() {
        let alpha = pixel >> 24;
        for shift in [0, 8, 16] {
            assert!(((pixel >> shift) & 255) <= alpha);
        }
    }
}

#[test]
/// 验证尺寸溢出和非有限配置在分配图像前被拒绝，无参数、无返回值。
fn invalid_layout_and_nonfinite_options_are_rejected() {
    assert!(
        Layout::new(
            1,
            1,
            &FrameOptions {
                padding: i32::MAX,
                ..options()
            }
        )
        .is_err()
    );
    assert!(Layout::new(i32::MAX, i32::MAX, &options()).is_err());
    assert!(Layout::new(0, 20, &options()).is_err());
    assert!(
        Layout::new(
            10,
            10,
            &FrameOptions {
                corner_radius: f64::NAN,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        Layout::new(
            10,
            10,
            &FrameOptions {
                shadow_opacity: f64::INFINITY,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        Layout::new(
            10,
            10,
            &FrameOptions {
                shadow_radius: i32::MAX,
                ..options()
            }
        )
        .is_err()
    );
    assert_eq!(
        Layout::new(
            10,
            10,
            &FrameOptions {
                padding: -1,
                ..options()
            }
        )
        .unwrap()
        .padding,
        0
    );
}
