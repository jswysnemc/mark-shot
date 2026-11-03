//! 【贴图测试】【边缘缩放】独立核对八方向固定边、等比缩放与原图导出

use mark_shot::features::pinned_image::{
    model::PinnedImage,
    placement::Geometry,
    resize::{Edge, Resize},
};

/// 【贴图测试】【八方向命中】按实际窗口边与角落逐项核对，窗口内部与范围外不误判
#[test]
fn eight_edges_and_corners_match_qt_hit_order() {
    for (point, edge) in [
        ((2.0, 60.0), Edge::Left),
        ((238.0, 60.0), Edge::Right),
        ((120.0, 2.0), Edge::Top),
        ((120.0, 118.0), Edge::Bottom),
        ((2.0, 2.0), Edge::TopLeft),
        ((238.0, 2.0), Edge::TopRight),
        ((2.0, 118.0), Edge::BottomLeft),
        ((238.0, 118.0), Edge::BottomRight),
    ] {
        assert_eq!(Edge::at((240, 120), point, 8.0), Some(edge));
    }
    for point in [(120.0, 60.0), (-9.0, 60.0), (120.0, 129.0), (f64::NAN, 2.0)] {
        assert_eq!(Edge::at((240, 120), point, 8.0), None);
    }
    assert_eq!(Edge::at((12, 12), (6.0, 6.0), 8.0), Some(Edge::TopLeft));
}

/// 【贴图测试】【对边固定】八个手势放大两倍，几何必须与 Qt 的固定边和另一轴居中规则一致
#[test]
fn all_directions_keep_the_opposite_edge_or_axis_center() {
    let start = Geometry::new((100, 200), (240, 120));
    for (edge, pointer, position) in [
        (Edge::Left, (-240.0, 0.0), (-140, 140)),
        (Edge::Right, (240.0, 0.0), (100, 140)),
        (Edge::Top, (0.0, -120.0), (-20, 80)),
        (Edge::Bottom, (0.0, 120.0), (-20, 200)),
        (Edge::TopLeft, (-240.0, -120.0), (-140, 80)),
        (Edge::TopRight, (240.0, -120.0), (100, 80)),
        (Edge::BottomLeft, (-240.0, 120.0), (-140, 200)),
        (Edge::BottomRight, (240.0, 120.0), (100, 200)),
    ] {
        let drag = Resize {
            edge,
            start,
            pointer: (0.0, 0.0),
        };
        assert_eq!(
            drag.geometry(pointer),
            Geometry::new(position, (480, 240)),
            "{edge:?}"
        );
    }
}

/// 【贴图测试】【角落主轴】以比例变化较大的轴决定尺寸，不采用位移较大的轴或平均位移
#[test]
fn corner_uses_largest_relative_delta_and_height_tie_rule() {
    let resize = Resize {
        edge: Edge::BottomRight,
        start: Geometry::new((0, 0), (240, 120)),
        pointer: (0.0, 0.0),
    };
    assert_eq!(
        resize.geometry((24.0, 60.0)),
        Geometry::new((0, 0), (360, 180))
    );
    assert_eq!(
        resize.geometry((-120.0, 60.0)),
        Geometry::new((0, 0), (120, 60))
    );
}

/// 【贴图测试】【最小范围】越过固定边也不得翻转比例，两边最小二十四像素，累计回移恢复起始几何
#[test]
fn crossing_anchor_clamps_both_axes_and_does_not_accumulate_rounding() {
    let resize = Resize {
        edge: Edge::TopLeft,
        start: Geometry::new((-20, 100), (240, 120)),
        pointer: (5.0, 8.0),
    };
    assert_eq!(
        resize.geometry((1000.0, 1000.0)),
        Geometry::new((172, 196), (48, 24))
    );
    assert_eq!(resize.geometry((5.0, 8.0)), resize.start);
}

/// 【贴图测试】【贴屏分流】只有手势涉及的边贴住输出才改为移动，不阻止另一侧边缘缩放
#[test]
fn embedded_edges_follow_requested_direction() {
    let screen = Geometry::new((-1280, 0), (1280, 800));
    let image = Geometry::new((-1280, 60), (240, 120));
    assert!(Edge::Left.embedded(image, screen));
    assert!(Edge::TopLeft.embedded(image, screen));
    assert!(!Edge::Right.embedded(image, screen));
    assert!(!Edge::Top.embedded(image, screen));
}

/// 【贴图测试】【原图分离】拖动超过输出大小与六倍菜单倍率，复制和旋转仍保留全部原始通道
#[test]
fn edge_resize_can_exceed_output_without_changing_source_pixels() {
    let mut pixmap = tiny_skia::Pixmap::new(240, 120).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(210, 55, 65, 128));
    let mut image = PinnedImage::decode(&pixmap.encode_png().unwrap(), 1.2).unwrap();
    let original = image.png().unwrap();
    image.set_size((2400, 1200));
    assert_eq!(image.size(), (2400, 1200));
    assert_eq!(image.zoom, 12.0);
    assert_eq!(image.png().unwrap(), original);
    image.rotate(true).unwrap();
    // 1. 【贴图测试】【旋转倍率】原版旋转重新进入倍率计算，超过六倍的边缘拖动尺寸会恢复到六倍
    assert_eq!(image.size(), (600, 1200));
    image.rotate(false).unwrap();
    assert_eq!(image.png().unwrap(), original);
    image.scale(1.0);
    assert_eq!(image.size(), (200, 100));
}

/// 【贴图测试】【最小窗口旋转】旋转后重新计算宽度倍率，后续缩放从当前显示大小继续，不沿用旧宽轴
#[test]
fn minimum_extent_rotation_refreshes_the_zoom_basis() {
    let pixmap = tiny_skia::Pixmap::new(240, 120).unwrap();
    let mut image = PinnedImage::decode(&pixmap.encode_png().unwrap(), 1.0).unwrap();
    image.scale(0.1);
    assert_eq!(image.size(), (24, 24));
    image.rotate(true).unwrap();
    assert_eq!(image.size(), (24, 24));
    assert_eq!(image.zoom, 0.2);
    image.scale(image.zoom * 1.18);
    assert_eq!(image.size(), (28, 57));
}

/// 【贴图测试】【倍率下限】滚轮与菜单沿用 Qt 的 0.1–6 倍和二十四像素窗口下限，不应用初始屏幕适配
#[test]
fn wheel_and_menu_zoom_have_different_constraints_from_initial_fit() {
    let pixmap = tiny_skia::Pixmap::new(240, 120).unwrap();
    let mut image = PinnedImage::decode(&pixmap.encode_png().unwrap(), 1.0).unwrap();
    image.set_zoom(1.0, (120, 60));
    assert_eq!(image.size(), (120, 60));
    image.scale(100.0);
    assert_eq!(image.size(), (1440, 720));
    image.scale(0.001);
    assert_eq!(image.size(), (24, 24));
    let size = image.size();
    assert!(!image.scale(f32::NAN));
    assert_eq!(image.size(), size);
}
