use std::collections::HashSet;

use mark_shot::features::marker::{draw_marker, path_for_shape};
use mark_shot::renderer::Painter;
use mark_shot::types::MarkerShape;
use tiny_skia::{Color, Pixmap, Rect};

/// 【标记测试】【渲染图像】使用 shape、filled、width 在透明底绘制；返回固定 100 像素图像
fn render(shape: MarkerShape, filled: bool, width: f32) -> Pixmap {
    let mut pixels = Pixmap::new(100, 100).unwrap();
    draw_marker(
        &mut Painter::new(&mut pixels),
        shape,
        Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
        Color::from_rgba8(255, 40, 20, 128),
        width,
        filled,
    );
    pixels
}

/// 【标记测试】【完整目录】24 个稳定编号和配置名称互不重复，显示顺序不改变原版编号
#[test]
fn all_shapes_have_unique_stable_ids_and_round_trip_names() {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for shape in MarkerShape::ALL {
        assert!(ids.insert(shape as i64));
        assert!(names.insert(shape.key()));
        assert_eq!(shape.key().parse::<MarkerShape>().unwrap(), shape);
        assert_eq!(MarkerShape::from_legacy_id(shape as i64), Some(shape));
        assert_eq!(
            serde_json::from_str::<MarkerShape>(&serde_json::to_string(&shape).unwrap()).unwrap(),
            shape
        );
    }
    assert_eq!(MarkerShape::from_legacy_id(6), Some(MarkerShape::Hexagon));
    assert_eq!(MarkerShape::ALL[6], MarkerShape::Spade);
    assert!(MarkerShape::from_legacy_id(-1).is_none());
    assert!(MarkerShape::from_legacy_id(24).is_none());
    assert!(MarkerShape::from_legacy_id(i64::MAX).is_none());
    assert_eq!(
        " ARROW_UP ".parse::<MarkerShape>().unwrap(),
        MarkerShape::ArrowUp
    );
    assert!("unknown".parse::<MarkerShape>().is_err());
}

/// 【标记测试】【全部路径】全部形状存在非空路径，并限制在带偏移的目标矩形内
#[test]
fn all_twenty_four_paths_fit_the_target_rectangle() {
    let rect = Rect::from_xywh(-30.0, 80.0, 240.0, 180.0).unwrap();
    for shape in MarkerShape::ALL {
        let path = path_for_shape(shape, rect).unwrap_or_else(|| panic!("missing {}", shape.key()));
        let bounds = path.bounds();
        assert!(bounds.left() >= rect.left() - 0.001, "{}", shape.key());
        assert!(bounds.right() <= rect.right() + 0.001, "{}", shape.key());
        assert!(bounds.top() >= rect.top() - 0.001, "{}", shape.key());
        assert!(bounds.bottom() <= rect.bottom() + 0.001, "{}", shape.key());
        assert!(
            render(shape, true, 3.0)
                .pixels()
                .iter()
                .any(|pixel| pixel.alpha() > 0),
            "{}",
            shape.key()
        );
    }
}

/// 【标记测试】【比例映射】非正方形框保留单位轮廓比例映射与全局偏移
#[test]
fn path_mapping_preserves_image_rectangle_coordinates() {
    let path = path_for_shape(
        MarkerShape::Triangle,
        Rect::from_xywh(12.0, 20.0, 100.0, 50.0).unwrap(),
    )
    .unwrap();
    let bounds = path.bounds();
    assert!((bounds.left() - 22.0).abs() < 0.001);
    assert!((bounds.right() - 102.0).abs() < 0.001);
    assert!((bounds.top() - 25.0).abs() < 0.001);
    assert!((bounds.bottom() - 64.0).abs() < 0.001);
}

/// 【标记测试】【空心填充】描边和填充分别改变内部像素，非填充笔宽增大时轮廓面积增加
#[test]
fn filled_and_outline_styles_keep_independent_stroke_widths() {
    let thin = render(MarkerShape::Square, false, 1.5);
    let thick = render(MarkerShape::Square, false, 7.0);
    let filled = render(MarkerShape::Square, true, 3.0);
    assert_eq!(thin.pixel(50, 50).unwrap().alpha(), 0);
    assert_eq!(thick.pixel(50, 50).unwrap().alpha(), 0);
    assert_eq!(filled.pixel(50, 50).unwrap().alpha(), 128);
    let count = |pixels: &Pixmap| {
        pixels
            .pixels()
            .iter()
            .filter(|pixel| pixel.alpha() > 0)
            .count()
    };
    assert!(count(&thick) > count(&thin));
}

/// 【标记测试】【并集透明度】交叉、加号和梅花的部件交叠不增加中心透明度或产生内部描边
#[test]
fn merged_parts_do_not_double_blend_their_intersection() {
    for shape in [MarkerShape::Cross, MarkerShape::Plus, MarkerShape::Club] {
        let image = render(shape, true, 3.0);
        assert_eq!(image.pixel(50, 50).unwrap().alpha(), 128, "{}", shape.key());
    }
}

/// 【标记测试】【保留孔洞】定位针中心保持透明，禁止图案的斜杠可见且旁侧孔洞不填实
#[test]
fn pin_and_ban_preserve_their_holes() {
    let pin = render(MarkerShape::Pin, true, 3.0);
    assert_eq!(pin.pixel(50, 38).unwrap().alpha(), 0);
    assert!(pin.pixel(50, 60).unwrap().alpha() > 0);
    let ban = render(MarkerShape::Ban, true, 3.0);
    assert_eq!(ban.pixel(50, 50).unwrap().alpha(), 128);
    assert_eq!(ban.pixel(50, 29).unwrap().alpha(), 0);
    assert!(ban.pixel(50, 10).unwrap().alpha() > 0);
}

/// 【标记测试】【曲线裁切】新月只保留大圆减去偏移圆后的区域
#[test]
fn crescent_keeps_its_subtracted_side_clear() {
    let crescent = render(MarkerShape::Crescent, true, 3.0);
    assert!(crescent.pixel(15, 50).unwrap().alpha() > 0);
    assert_eq!(crescent.pixel(80, 50).unwrap().alpha(), 0);
}
