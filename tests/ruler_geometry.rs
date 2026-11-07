//! 【尺子测试】【测量几何】验证原图单位、退化范围、角度定点与导出比例

use mark_shot::features::ruler::angle_draft::{self, DraftStep};
use mark_shot::features::ruler::geometry::{angle_degrees, distance};
use mark_shot::features::ruler::model::width_unit;
use mark_shot::features::ruler::ticks::{major_step, ticks};
use mark_shot::features::ruler::{Measurement, RulerGeometry, RulerMode};

/// 【尺子测试】【长度构造】返回以 scale 换算原图像素的 3×4 逻辑单位长度尺
fn length(scale: f32) -> Measurement {
    let mut ruler = Measurement::new(RulerMode::Length, (20.0, 30.0), scale).unwrap();
    ruler.geometry = RulerGeometry::Length {
        start: (20.0, 30.0),
        end: (23.0, 34.0),
        rotation: 0.0,
    };
    ruler
}

/// 【尺子测试】【源图比例】宽、高、对角线与面积使用源图像素，不使用预览或显示器逻辑像素
#[test]
fn length_metrics_use_original_pixels_and_survive_export_mapping() {
    let ruler = length(2.0);
    assert_eq!(ruler.length_metrics(), Some((6, 8, 10, 48)));
    assert_eq!(
        ruler.info_text().as_deref(),
        Some("6 x 8 px   diag 10 px   area 48 px")
    );
    let export = ruler.scaled(2.0);
    assert_eq!(export.length_metrics(), ruler.length_metrics());
    assert_eq!(export.info_text(), ruler.info_text());
    assert_eq!(export.unit(3.0), 2.0);
    assert_eq!(export.unit(6.0), 8.0 / 3.0);
    assert_eq!(export.unit(24.0), ruler.unit(12.0) * 2.0);
}

/// 【尺子测试】【分数映射】多次分数比例映射保留测量值与样式，不把导出笔宽重新代入粗细公式
#[test]
fn fractional_mapping_keeps_measurements_and_style_units() {
    let ruler = length(1.5);
    let transformed = ruler.scaled(1.2).scaled(1.5);
    assert_eq!(transformed.length_metrics(), ruler.length_metrics());
    assert!((transformed.unit(9.0 * 1.8) - ruler.unit(9.0) * 1.8).abs() < 0.0001);
    assert_eq!(ruler.scaled(f32::NAN), ruler);
    assert_eq!(ruler.scaled(0.0), ruler);
}

/// 【尺子测试】【旋转稳定】旋转长度尺只改变可见边界，不改变原图测量值
#[test]
fn rotation_preserves_length_and_changes_visual_corners() {
    let ruler = length(1.0);
    let rotated = ruler.rotated(90.0);
    assert_eq!(rotated.length_metrics(), ruler.length_metrics());
    let bounds = rotated.bounds().unwrap();
    assert!((bounds.width() - 4.0).abs() < 0.0001);
    assert!((bounds.height() - 3.0).abs() < 0.0001);
    assert!(
        (distance(rotated.corners().unwrap()[0], rotated.corners().unwrap()[1]) - 3.0).abs()
            < 0.0001
    );
}

/// 【尺子测试】【平直测量】横向和纵向尺子可以提交，零长度单击不能产生标注，边界至少覆盖一个原图像素
#[test]
fn straight_rulers_are_valid_but_empty_gestures_are_not() {
    let mut ruler = Measurement::new(RulerMode::Length, (50.0, 50.0), 2.0).unwrap();
    assert!(!ruler.measures());
    assert_eq!(ruler.info_text(), None);
    ruler.geometry = RulerGeometry::Length {
        start: (50.0, 50.0),
        end: (100.0, 50.0),
        rotation: 0.0,
    };
    assert!(ruler.measures());
    assert_eq!(ruler.length_metrics(), Some((100, 0, 100, 0)));
    assert_eq!(ruler.bounds().unwrap().height(), 0.5);
    ruler.geometry = RulerGeometry::Length {
        start: (50.0, 50.0),
        end: (50.0, 100.0),
        rotation: 0.0,
    };
    assert!(ruler.measures());
    assert_eq!(ruler.bounds().unwrap().width(), 0.5);
}

/// 【尺子测试】【夹角范围】夹角取较小一侧，覆盖直角、平角、重合边与退化边
#[test]
fn angle_uses_the_smaller_unsigned_span() {
    assert_eq!(angle_degrees((0.0, 0.0), (10.0, 0.0), (0.0, 10.0)), 90.0);
    assert_eq!(angle_degrees((0.0, 0.0), (10.0, 0.0), (-10.0, 0.0)), 180.0);
    assert_eq!(angle_degrees((0.0, 0.0), (10.0, 0.0), (20.0, 0.0)), 0.0);
    assert_eq!(angle_degrees((0.0, 0.0), (0.0, 0.0), (20.0, 0.0)), 0.0);
    assert!((angle_degrees((10.0, 10.0), (20.0, 20.0), (20.0, 10.0)) - 45.0).abs() < 0.001);
}

/// 【尺子测试】【三点草稿】单击继续跟随，第一边确认后追加第二边，三点完整才提交
#[test]
fn angle_draft_supports_click_click_click_and_drag_then_click() {
    let mut ruler = Measurement::new(RulerMode::Angle, (20.0, 30.0), 1.5).unwrap();
    assert_eq!(
        angle_draft::release(&mut ruler, (20.0, 30.0), 4.0),
        DraftStep::Continue
    );
    assert!(!ruler.measures());
    assert!(angle_draft::follow(&mut ruler, (100.0, 30.0)));
    assert_eq!(
        angle_draft::release(&mut ruler, (100.0, 30.0), 4.0),
        DraftStep::Continue
    );
    assert_eq!(
        angle_draft::release(&mut ruler, (100.0, 31.0), 4.0),
        DraftStep::Continue
    );
    assert_eq!(
        angle_draft::release(&mut ruler, (20.0, 130.0), 4.0),
        DraftStep::Commit
    );
    assert!(ruler.measures());
    assert_eq!(ruler.info_text().as_deref(), Some("90.0\u{b0}"));
    assert_eq!(ruler.scaled(3.0).info_text(), ruler.info_text());
    assert_eq!(ruler.rotated(37.0).info_text(), ruler.info_text());
}

/// 【尺子测试】【角度吸附】Shift 约束方向到 45 度倍数，同时保留原始边长
#[test]
fn angle_constraints_snap_direction_without_changing_arm_length() {
    let vertex = (30.0, 40.0);
    let point = (90.0, 78.0);
    let snapped = angle_draft::constrained_point(vertex, point, true);
    assert!((snapped.0 - vertex.0 - (snapped.1 - vertex.1)).abs() < 0.0001);
    assert!((distance(vertex, point) - distance(vertex, snapped)).abs() < 0.0001);
    assert_eq!(angle_draft::constrained_point(vertex, point, false), point);
}

/// 【尺子测试】【输入校验】非法源图比例和非有限起点不能构造测量
#[test]
fn invalid_coordinates_and_pixel_scales_are_rejected() {
    for scale in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert_eq!(
            Measurement::new(RulerMode::Length, (10.0, 20.0), scale),
            None
        );
    }
    assert_eq!(
        Measurement::new(RulerMode::Angle, (f32::NAN, 20.0), 1.0),
        None
    );
}

/// 【尺子测试】【粗细限制】粗细倍数与原版公式一致，极端笔宽仍保持可读尺寸
#[test]
fn width_style_matches_the_reference_formula() {
    assert_eq!(width_unit(3.0), 1.0);
    assert_eq!(width_unit(12.0), 2.0);
    assert_eq!(width_unit(-100.0), 0.75);
    assert_eq!(width_unit(1000.0), 3.5);
}

/// 【尺子测试】【刻度密度】主刻度使用 1、2、5 间隔，千万像素源图不会生成千万个刻度
#[test]
fn ticks_have_bounded_density_and_source_pixel_numbers() {
    assert_eq!(major_step(100.0, 100.0, 15.0), 20);
    assert_eq!(major_step(1000.0, 100.0, 15.0), 200);
    for (pixels, paint) in [(1.0, 10.0), (100.0, 100.0), (10_000_000.0, 500.0)] {
        let ticks = ticks(pixels, paint, 40.0);
        assert!(!ticks.is_empty() && ticks.len() <= 257);
        assert_eq!(ticks[0].pixel, 0);
        assert!(ticks[0].major);
        assert!(ticks.iter().all(|tick| tick.position <= paint));
        let major = major_step(pixels, paint, 40.0);
        assert!(
            ticks
                .iter()
                .filter(|tick| tick.major)
                .all(|tick| tick.pixel.is_multiple_of(major))
        );
    }
    assert!(ticks(0.0, 50.0, 20.0).is_empty());
    assert!(ticks(f32::NAN, 50.0, 20.0).is_empty());
}
