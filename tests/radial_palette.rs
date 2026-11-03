//! 【快捷色板测试】【几何绘制】十色环形按钮与中心预览按实际 Qt 控件几何验证

use mark_shot::{
    renderer::Painter,
    theme::Rgba,
    ui::{
        color_popover::{ColorPickerPopover, draw_color_popover},
        radial_palette::{COLORS, RadialPalette},
    },
};
use tiny_skia::Pixmap;

/// 【快捷色板测试】【按钮位置】原版十个按钮使用实际二十八像素大小，空白与中心不命中颜色
#[test]
fn button_geometry_matches_the_qt_widget_tree() {
    for (index, (x, y)) in [
        (74.0, 6.0),
        (114.0, 19.0),
        (139.0, 53.0),
        (139.0, 95.0),
        (114.0, 129.0),
        (74.0, 142.0),
        (34.0, 129.0),
        (9.0, 95.0),
        (9.0, 53.0),
        (34.0, 19.0),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = RadialPalette::button(index).unwrap();
        assert_eq!(
            (rect.left(), rect.top(), rect.width(), rect.height()),
            (x, y, 28.0, 28.0)
        );
        assert_eq!(
            RadialPalette::hit((x as f64 + 14.0, y as f64 + 14.0)),
            Some(index)
        );
    }
    assert_eq!(RadialPalette::hit((89.0, 89.0)), None);
    assert_eq!(RadialPalette::hit((0.0, 0.0)), None);
    assert!(RadialPalette::button(10).is_none());
}

/// 【快捷色板测试】【预览范围】工具预览钳制为八到三十四像素，面板四边保留八像素
#[test]
fn preview_and_position_use_the_original_constraints() {
    assert_eq!(RadialPalette::preview(1.0).width(), 8.0);
    assert_eq!(RadialPalette::preview(100.0).width(), 34.0);
    assert_eq!(
        RadialPalette {
            anchor: (300.0, 200.0),
            ..Default::default()
        }
        .position((640, 480)),
        (211.0, 111.0)
    );
    assert_eq!(RadialPalette::default().position((640, 480)), (8.0, 8.0));
}

/// 【快捷色板测试】【无底板绘制】真实取色面板交接不追加 LumineCapture 边框，十色中心不受透明度影响
#[test]
fn radial_mode_has_no_host_background_or_generic_panel_border() {
    let mut picker = ColorPickerPopover::new();
    picker.open = true;
    picker.opacity = 1.0;
    picker.dirty = true;
    picker.size = (178.0, 178.0);
    picker.radial = Some(RadialPalette {
        preview_size: 8.0,
        ..Default::default()
    });
    let mut canvas = Pixmap::new(178, 178).unwrap();
    draw_color_popover(&mut Painter::new(&mut canvas), &mut picker);
    for point in [(0, 0), (89, 0), (0, 89), (177, 89), (89, 177)] {
        assert_eq!(canvas.pixel(point.0, point.1).unwrap().alpha(), 0);
    }
    for (index, color) in COLORS.iter().enumerate() {
        let rect = RadialPalette::button(index).unwrap();
        let sample = canvas
            .pixel(rect.left() as u32 + 14, rect.top() as u32 + 14)
            .unwrap()
            .demultiply();
        assert_eq!(
            (sample.red(), sample.green(), sample.blue(), sample.alpha()),
            (color.0, color.1, color.2, 255)
        );
    }
}

/// 【快捷色板测试】【渲染证据】按 MARK_SHOT_COLOR_ARTIFACTS 输出环形面板，与实际 Qt 控件对照
#[test]
#[ignore = "requires an explicit artifact directory"]
fn render_reference_fixture() {
    let root = std::path::PathBuf::from(std::env::var("MARK_SHOT_COLOR_ARTIFACTS").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let mut canvas = Pixmap::new(178, 178).unwrap();
    mark_shot::ui::radial_palette::draw(
        &mut Painter::new(&mut canvas),
        &RadialPalette {
            preview_size: 8.0,
            ..Default::default()
        },
        Rgba(255, 77, 77, 255),
    );
    canvas.save_png(root.join("radial.png")).unwrap();
}
