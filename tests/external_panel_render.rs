//! 【外部面板绘制测试】【局部一致】透明面板、文字、主题图标和分数缩放的局部重绘与整帧逐像素比较

use mark_shot::{
    features::external_actions::{Action, desktop_apps::DesktopApp},
    renderer::Painter,
    ui::external_panel::{ExternalPanel, Kind, draw, layout::Layout},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【外部面板绘制测试】【局部恢复】把 canvas 的 area 物理像素恢复到 background，返回实际损坏范围
fn restore(canvas: &mut Pixmap, background: &Pixmap, area: Rect, scale: f32) -> Rect {
    let (x, y, width, height) = mark_shot::renderer::rect_bounds(
        &mark_shot::renderer::scale_rect(&area, scale).unwrap(),
        canvas.width(),
        canvas.height(),
    )
    .unwrap();
    let stride = canvas.width() as usize * 4;
    for row in y..y + height {
        let start = row as usize * stride + x as usize * 4;
        canvas.data_mut()[start..start + width as usize * 4]
            .copy_from_slice(&background.data()[start..start + width as usize * 4]);
    }
    Rect::from_xywh(
        x as f32 / scale,
        y as f32 / scale,
        width as f32 / scale,
        height as f32 / scale,
    )
    .unwrap()
}

/// 【外部面板绘制测试】【透明局部】四种比例下重绘部分文字和图标，正文外像素与完整面板均保持一致
#[test]
fn translucent_external_panel_partial_redraw_matches_full_frame() {
    let root = tempfile::tempdir().unwrap();
    let icon = root.path().join("test-icon.png");
    let mut source = Pixmap::new(22, 22).unwrap();
    source.fill(Color::from_rgba8(90, 30, 140, 120));
    source.save_png(&icon).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    for scale in [0.75, 1.0, 1.2, 2.0] {
        let mut background = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
        background.fill(Color::from_rgba8(42, 48, 63, 255));
        let mut canvas = background.clone();
        let actions = (0..24)
            .map(|index| {
                Action::Desktop(DesktopApp {
                    name: format!("Long image application number {index}"),
                    desktop_path: "/app.desktop".into(),
                    exec: "viewer %f".into(),
                    icon: icon.to_str().unwrap().into(),
                })
            })
            .collect();
        let layout = Layout::new(Kind::OpenWith, 24, 290.0, 36.0, (640.0, 480.0));
        let mut panel = ExternalPanel::open(
            Kind::OpenWith,
            actions,
            String::new(),
            layout,
            (23.0, 41.0),
            0,
        );
        panel.offset = 17.0;
        panel.hovered = Some(1);
        panel.active = Some(2);
        draw::draw(
            &mut Painter::new(&mut canvas)
                .with_scale(scale)
                .with_text(&mut fonts, &mut cache),
            &mut panel,
        );
        let expected = canvas.clone();
        for area in [
            Rect::from_xywh(20.0, 56.0, 65.0, 117.0).unwrap(),
            Rect::from_xywh(125.0, 109.0, 40.0, 85.0).unwrap(),
            Rect::from_xywh(278.0, 60.0, 95.0, 193.0).unwrap(),
        ] {
            let actual = restore(&mut canvas, &background, area, scale);
            panel.dirty = true;
            Painter::new(&mut canvas)
                .with_scale(scale)
                .with_text(&mut fonts, &mut cache)
                .clipped_scene(Some(actual), |p| draw::draw(p, &mut panel));
            assert_eq!(
                canvas.data(),
                expected.data(),
                "scale={scale}, area={area:?}"
            );
        }
    }
}
