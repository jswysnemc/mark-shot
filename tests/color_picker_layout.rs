//! 【取色器测试】【原版几何】几何取自实际 Qt 控件，不使用旧配置推算控件尺寸

use mark_shot::{
    renderer::Painter,
    ui::color_popover::{self, ColorField, ColorPickerPopover, layout, parse_hex_color},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【取色器测试】【显示状态】返回逻辑原点为零的已显示取色器，不连接显示服务器
fn picker() -> ColorPickerPopover {
    let mut picker = ColorPickerPopover::new();
    picker.open = true;
    picker.opacity = 1.0;
    picker.dirty = true;
    picker.select_color(Color::from_rgba8(255, 77, 77, 255));
    picker.history.clear();
    picker.refresh_palette();
    picker.sync_field_values();
    picker
}

/// 【取色器测试】【矩形比较】将 rect 转为可读坐标数组并和 expected 比较，无返回值
fn geometry(rect: Rect, expected: [f32; 4]) {
    assert_eq!(
        [rect.left(), rect.top(), rect.width(), rect.height()],
        expected
    );
}

/// 【取色器测试】【实际布局】颜色面板为二百乘二百二十八，SV 为横向矩形，只有一个文本字段
#[test]
fn geometry_matches_the_qt_widget_tree() {
    assert_eq!(
        (color_popover::width(), color_popover::height()),
        (200.0, 228.0)
    );
    geometry(layout::sv((0.0, 0.0)), [8.0, 8.0, 184.0, 112.0]);
    geometry(layout::hue((0.0, 0.0)), [8.0, 126.0, 184.0, 12.0]);
    geometry(layout::alpha((0.0, 0.0)), [8.0, 144.0, 184.0, 12.0]);
    geometry(layout::current((0.0, 0.0)), [8.0, 162.0, 26.0, 26.0]);
    geometry(layout::hex((0.0, 0.0)), [40.0, 162.0, 152.0, 26.0]);
    geometry(layout::content((0.0, 0.0)), [8.0, 8.0, 184.0, 212.0]);
    for (index, x) in [8.0, 32.0, 56.0, 79.0, 103.0, 127.0, 150.0, 174.0]
        .into_iter()
        .enumerate()
    {
        geometry(
            layout::history((0.0, 0.0), index).unwrap(),
            [x, 194.0, 26.0, 26.0],
        );
    }
    assert!(picker().rgba_field_rect(ColorField::A).is_none());
    assert!(!picker().eyedropper_hit((8.0, 175.0)));
}

/// 【取色器测试】【锚点布局】属性按钮下方不足时移到属性栏上方，四边与原版一致保留八像素
#[test]
fn position_follows_the_color_button_and_flips_above_properties() {
    let properties = Rect::from_xywh(200.0, 100.0, 220.0, 42.0).unwrap();
    assert_eq!(
        layout::position((220.0, 120.0), properties, (200.0, 228.0), (640, 480)),
        (120.0, 134.0)
    );
    let properties = Rect::from_xywh(200.0, 400.0, 220.0, 42.0).unwrap();
    assert_eq!(
        layout::position((220.0, 420.0), properties, (200.0, 228.0), (640, 480)),
        (120.0, 164.0)
    );
    assert_eq!(
        layout::position((5.0, 10.0), properties, (200.0, 228.0), (640, 480)).0,
        8.0
    );
}

/// 【取色器测试】【整数输入】滑条使用完整控件宽度减一，越界拖动钳制为端点，SV 两轴独立
#[test]
fn controls_use_qt_integer_mapping_and_clamp_outside_drags() {
    let mut picker = picker();
    picker.set_hue_from_local((191.0, 130.0));
    assert_eq!(picker.sv_square.hue, 359.0);
    picker.set_hue_from_local((8.0, 130.0));
    assert_eq!(picker.sv_square.hue, 0.0);
    picker.set_alpha_from_local((100.0, 150.0));
    assert_eq!(picker.sv_square.alpha, 128);
    picker.set_alpha_from_local((-100.0, 150.0));
    assert_eq!(picker.sv_square.alpha, 0);
    picker.set_sv_from_local((191.0, 119.0));
    assert_eq!(picker.sv_square.sv, (1.0, 0.0));
    picker.set_sv_from_local((-50.0, -50.0));
    assert_eq!(picker.sv_square.sv, (0.0, 1.0));
    assert_eq!(
        layout::value(layout::alpha((0.0, 0.0)), (8.49, 150.0), 255),
        0
    );
    picker.set_sv_from_local((8.49, 8.49));
    assert_eq!(picker.sv_square.sv, (0.0, 1.0));
}

/// 【取色器测试】【精确文本】十六进制输入保留原始 RGB 与 alpha，简写扩展正确，非 ASCII 输入不触发切片异常
#[test]
fn hex_input_preserves_exact_color_and_handles_unicode_safely() {
    let mut picker = picker();
    let value = parse_hex_color("#123ABC7F").unwrap();
    picker.select_color(value);
    picker.sync_field_values();
    assert_eq!(picker.sv_square.color(), value);
    assert_eq!(picker.field_text(ColorField::Hex), "#123ABC7F");
    assert_eq!(
        parse_hex_color("#abc"),
        Some(Color::from_rgba8(170, 187, 204, 255))
    );
    for text in ["#ＦＦ0000", "é00000", "#1234", "#xyz", "###abc"] {
        assert!(parse_hex_color(text).is_none());
    }
}

/// 【取色器测试】【Qt 色相整数】实际 QColor 的 cyan 样本返回一百八十八度，不能四舍五入到一百八十九度
#[test]
fn imported_colors_use_the_qt_integer_hue() {
    let mut picker = picker();
    picker.select_color(parse_hex_color("#06B6D4A0").unwrap());
    assert_eq!(picker.sv_square.hue, 188.0);
    assert_eq!(picker.sv_square.alpha, 160);
}

/// 【取色器测试】【历史排列】只显示当前色与去重历史，当前色禁用，重叠按后绘色块命中，末项裁切
#[test]
fn history_is_deduplicated_and_hit_testing_honors_qt_clipping() {
    let mut picker = picker();
    picker.history = (0..10)
        .map(|i| Color::from_rgba8(i * 20, 10, 20, 255))
        .collect();
    picker.history.insert(1, picker.history[0]);
    picker.refresh_palette();
    assert_eq!(picker.palette().len(), 8);
    assert_eq!(picker.swatch_hit((20.0, 207.0)), None);
    assert_eq!(picker.swatch_hit((33.0, 207.0)), Some(1));
    assert_eq!(picker.swatch_hit((57.0, 207.0)), Some(2));
    assert_eq!(picker.swatch_hit((191.0, 207.0)), Some(7));
    assert_eq!(picker.swatch_hit((192.0, 207.0)), None);
}

/// 【取色器测试】【主题底色】按 sample 核对宿主实际 RGBA，允许一单位预乘舍入；返回是否为未泄漏控件的底色
fn is_popup_background(sample: tiny_skia::ColorU8) -> bool {
    let color = mark_shot::theme::toolbar::Palette::current().popup_fill;
    [sample.red(), sample.green(), sample.blue(), sample.alpha()]
        .into_iter()
        .zip([color.0, color.1, color.2, color.3])
        .all(|(actual, expected)| actual.abs_diff(expected) <= 1)
}

/// 【取色器测试】【分数缩放】逻辑尺寸保持不变，子控件末端不泄漏历史色块，外部像素透明
#[test]
fn rendering_preserves_logical_geometry_at_fractional_scale() {
    for scale in [1.0, 1.2, 2.0] {
        let mut picker = picker();
        let mut pixmap = Pixmap::new((210.0 * scale) as u32, (238.0 * scale) as u32).unwrap();
        color_popover::draw_color_popover(
            &mut Painter::new(&mut pixmap).with_scale(scale),
            &mut picker,
        );
        assert_eq!(
            pixmap
                .pixel((205.0 * scale) as u32, (233.0 * scale) as u32)
                .unwrap()
                .alpha(),
            0
        );
        let sample = pixmap
            .pixel((196.0 * scale) as u32, (207.0 * scale) as u32)
            .unwrap()
            .demultiply();
        assert!(is_popup_background(sample));
    }
}

/// 【取色器测试】【握柄裁切】SV 与滑条的握柄不伸入相邻控件间距，独立控件裁切不能用宿主边界替代
#[test]
fn handles_are_clipped_to_each_individual_control() {
    let mut picker = picker();
    picker.set_sv_from_local((100.0, 119.0));
    picker.set_hue_from_local((100.0, 132.0));
    picker.set_alpha_from_local((100.0, 150.0));
    let mut canvas = Pixmap::new(200, 228).unwrap();
    color_popover::draw_color_popover(&mut Painter::new(&mut canvas), &mut picker);
    for (x, y) in [(100, 123), (100, 125), (100, 139), (100, 143), (100, 156)] {
        let sample = canvas.pixel(x, y).unwrap().demultiply();
        assert!(is_popup_background(sample), "pixel at {x},{y}: {sample:?}");
    }
}

/// 【取色器测试】【渲染证据】按 MARK_SHOT_COLOR_ARTIFACTS 输出零原点面板供实际 Qt 像素对照；普通测试不写文件
#[test]
#[ignore = "requires an explicit artifact directory"]
fn render_reference_fixture() {
    let root = std::path::PathBuf::from(std::env::var("MARK_SHOT_COLOR_ARTIFACTS").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let mut picker = picker();
    if let Ok(color) = std::env::var("MARK_SHOT_REFERENCE_COLOR") {
        picker.select_color(parse_hex_color(&color).unwrap());
    }
    if let Ok(history) = std::env::var("MARK_SHOT_REFERENCE_HISTORY") {
        picker.history = serde_json::from_str::<Vec<String>>(&history)
            .unwrap()
            .iter()
            .map(|color| parse_hex_color(color).unwrap())
            .collect();
    }
    picker.refresh_palette();
    picker.sync_field_values();
    let mut pixmap = Pixmap::new(200, 228).unwrap();
    color_popover::draw_color_popover(&mut Painter::new(&mut pixmap), &mut picker);
    pixmap.save_png(root.join("dark.png")).unwrap();
}
