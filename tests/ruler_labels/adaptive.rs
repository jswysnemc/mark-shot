//! 【尺子测试】【自适应排版】比较实际尺身、裁切后的窗口和可用外侧空间

use super::*;
use mark_shot::features::ruler::{
    Readouts,
    geometry::{rect_center, rotated},
    labels::InfoLabel,
    tick_layout::Layout,
};

/// 【尺子测试】【单项布局】按 measurement 和 bounds 返回正式布局入口生成的唯一标签
fn label(measurement: Measurement, bounds: Rect) -> InfoLabel {
    prepare_all(
        [(measurement, 3.0)].into_iter(),
        bounds,
        &mut mark_shot::fonts::bundled(),
    )
    .pop()
    .unwrap()
}

/// 【尺子测试】【真实包含】将卡片 card 的四角反向旋转，判断它们是否全部位于 measurement 的局部尺身内
fn inside(measurement: Measurement, card: Rect) -> bool {
    let RulerGeometry::Length { rotation, .. } = measurement.geometry else {
        return false;
    };
    let body = measurement.unrotated_bounds().unwrap();
    [
        (card.left(), card.top()),
        (card.right(), card.top()),
        (card.right(), card.bottom()),
        (card.left(), card.bottom()),
    ]
    .into_iter()
    .all(|corner| {
        let (horizontal, vertical) = rotated(corner, rect_center(body), -rotation);
        horizontal >= body.left()
            && horizontal <= body.right()
            && vertical >= body.top()
            && vertical <= body.bottom()
    })
}

/// 【尺子测试】【宽高分行】空间足够时宽高可分行；仅容一行的扁尺仍优先内排而不是将信息移到外侧
#[test]
fn dimensions_stack_inside_unless_only_one_row_fits() {
    let bounds = Rect::from_xywh(0.0, 0.0, 900.0, 500.0).unwrap();
    for (end, multiline) in [((640.0, 270.0), true), ((640.0, 143.0), false)] {
        let mut measurement = ruler((120.0, 100.0), end);
        measurement.readouts = Readouts {
            width: true,
            height: true,
            ..Default::default()
        };
        let card = label(measurement, bounds);
        assert!(inside(measurement, card.rect));
        assert_eq!(card.lines.len() == 2, multiline);
        if multiline {
            assert_eq!(card.lines, ["width 1040 px", "height 340 px"]);
        } else {
            assert_eq!(card.lines, ["1040 x 86 px"]);
        }
    }
}

/// 【尺子测试】【旋转与裁切】裁切后的旋转尺身只在四角真实可见时才内排；细斜尺不能用外接框冒充内部
#[test]
fn rotated_and_cropped_rulers_use_the_visible_oriented_interior() {
    let large = ruler((40.0, 20.0), (620.0, 410.0)).rotated(31.0);
    let crop = Rect::from_xywh(340.0, 150.0, 245.0, 200.0).unwrap();
    let card = label(large, crop);
    assert!(inside(large, card.rect) && contains(crop, card.rect));
    assert!(card.lines.len() > 1);
    let thin = ruler((150.0, 205.0), (510.0, 223.0)).rotated(45.0);
    let bounds = Rect::from_xywh(0.0, 0.0, 900.0, 650.0).unwrap();
    let card = label(thin, bounds);
    let scale = Layout::new(thin, 3.0, Some(bounds), Default::default())
        .unwrap()
        .scale_bounds()
        .unwrap();
    assert!(card.rect.intersect(&scale).is_none() && contains(bounds, card.rect));
}

/// 【尺子测试】【外排方向】水平直尺根据窗口宽度选择单行或多行，放不下内排时也不覆盖刻度
#[test]
fn outside_readouts_wrap_only_as_needed_and_clear_the_ticks() {
    for (width, wrapped) in [(700.0, false), (166.0, true)] {
        let measurement = ruler((width * 0.5 - 62.5, 200.0), (width * 0.5 + 62.5, 200.0));
        let bounds = Rect::from_xywh(0.0, 0.0, width, 400.0).unwrap();
        let card = label(measurement, bounds);
        let scale = Layout::new(measurement, 3.0, Some(bounds), Default::default())
            .unwrap()
            .scale_bounds()
            .unwrap();
        assert!(contains(bounds, card.rect) && card.rect.intersect(&scale).is_none());
        assert_eq!(card.lines.len() > 1, wrapped, "{card:?}");
        assert_eq!(card.lines.join("   "), measurement.info_text().unwrap());
    }
}

/// 【尺子测试】【尺内换位】中央有另一把尺子时，剩余侧边能容纳读数就不移到外部
#[test]
fn blocked_center_uses_free_space_inside_the_same_ruler() {
    let primary = ruler((60.0, 50.0), (710.0, 420.0));
    let mut obstacle = ruler((335.0, 70.0), (440.0, 390.0));
    obstacle.readouts = Default::default();
    let bounds = Rect::from_xywh(0.0, 0.0, 800.0, 470.0).unwrap();
    let scale = Layout::new(obstacle, 3.0, Some(bounds), Default::default())
        .unwrap()
        .scale_bounds()
        .unwrap();
    assert!(label(primary, bounds).rect.intersect(&scale).is_some());
    let cards = prepare_all(
        [(primary, 3.0), (obstacle, 3.0)].into_iter(),
        bounds,
        &mut mark_shot::fonts::bundled(),
    );
    assert_eq!(cards.len(), 1);
    assert!(inside(primary, cards[0].rect) && cards[0].rect.intersect(&scale).is_none());
}
