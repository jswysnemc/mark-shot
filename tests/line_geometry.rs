//! 【线条测试】【平滑骨架】验证原版点序、二次曲线、多点路由、弧长和有限输入

use mark_shot::features::line::{
    LineSkeleton, LineStyle, MAX_CONTROLS, geometry,
    polyline::{Polyline, distance},
};

/// 【线条测试】【默认骨架】返回固定端点且包含 controls 的骨架，不连接桌面
fn skeleton(controls: &[(f32, f32)]) -> LineSkeleton {
    let mut line = LineSkeleton::new((30.0, 70.0), (420.0, 70.0)).unwrap();
    line.controls.extend_from_slice(controls);
    line
}

/// 【线条测试】【直线弧长】无控制点保持两个端点，中点与投影使用真实线段位置
#[test]
fn straight_line_preserves_reference_endpoint_order() {
    let line = skeleton(&[]);
    let sampled = geometry::sampled(&line).unwrap();
    assert_eq!(sampled.points, [(30.0, 70.0), (420.0, 70.0)]);
    assert_eq!(sampled.length, 390.0);
    assert_eq!(sampled.at(195.0), (225.0, 70.0));
    assert_eq!(sampled.tangent(0.0), (1.0, 0.0));
    assert_eq!(sampled.tangent(sampled.length), (1.0, 0.0));
    let insertion = geometry::insertion_at(&line, (225.0, 75.0), 5.0).unwrap();
    assert_eq!(insertion.index, 2);
    assert_eq!(insertion.point, (225.0, 70.0));
    assert_eq!(insertion.distance, 5.0);
}

/// 【线条测试】【单控制点】控制点是二次曲线控制参数，而不是必须经过的路径定点
#[test]
fn one_control_uses_the_reference_quadratic_curve() {
    let line = skeleton(&[(225.0, 180.0)]);
    let sampled = geometry::sampled(&line).unwrap();
    assert_eq!(sampled.points.len(), 65);
    assert_eq!(sampled.points[0], line.start);
    assert_eq!(*sampled.points.last().unwrap(), line.end);
    assert!(distance(sampled.at(sampled.length * 0.5), (225.0, 125.0)) < 0.1);
    assert!(geometry::insertion_at(&line, (225.0, 125.0), 1.0).is_some());
    assert!(geometry::insertion_at(&line, (225.0, 180.0), 1.0).is_none());
}

/// 【线条测试】【多控制点】连续二次曲线保留所有控制点影响，不把骨架误画为折线
#[test]
fn multiple_controls_form_smooth_routes_with_exact_ends() {
    let line = skeleton(&[(120.0, 180.0), (300.0, -20.0)]);
    let sampled = geometry::sampled(&line).unwrap();
    assert_eq!(sampled.points[0], line.start);
    assert_eq!(*sampled.points.last().unwrap(), line.end);
    assert!(sampled.points.iter().any(|point| point.1 > 100.0));
    assert!(sampled.points.iter().any(|point| point.1 < 55.0));
    for ratio in [0.2, 0.5, 0.8] {
        let point = sampled.at(sampled.length * ratio);
        assert!(geometry::insertion_at(&line, point, 0.1).is_some());
    }
}

/// 【线条测试】【插入索引】新骨架点使用零一端点之后的原版索引，插入位置跟随路径方向
#[test]
fn insertion_and_removal_keep_endpoints_and_route_order() {
    let mut line = skeleton(&[]);
    let insertion = geometry::insertion_at(&line, (225.0, 70.0), 0.0).unwrap();
    assert!(line.insert_control(insertion.index, insertion.point));
    assert_eq!(line.point(0), Some((30.0, 70.0)));
    assert_eq!(line.point(1), Some((420.0, 70.0)));
    assert_eq!(line.point(2), Some((225.0, 70.0)));
    assert_eq!(
        geometry::insertion_at(&line, (100.0, 70.0), 0.1)
            .unwrap()
            .index,
        2
    );
    assert_eq!(
        geometry::insertion_at(&line, (350.0, 70.0), 0.1)
            .unwrap()
            .index,
        3
    );
    assert!(!line.remove_control(0));
    assert!(!line.remove_control(1));
    assert!(line.remove_control(2));
    assert_eq!(geometry::sampled(&line).unwrap().points.len(), 2);
}

/// 【线条测试】【导出比例】原图缩放同时改变细分误差和绝对样式比例，中心线路由不发生漂移
#[test]
fn original_resolution_scaling_preserves_curve_sampling() {
    let line = skeleton(&[(125.25, 175.75), (306.5, 22.25)]);
    let before = geometry::sampled(&line).unwrap();
    let scaled = line.scaled(3.25);
    let after = geometry::sampled(&scaled).unwrap();
    assert_eq!(after.points.len(), before.points.len());
    assert_eq!(scaled.style_scale, 3.25);
    for (first, second) in before.points.iter().zip(&after.points) {
        assert!(distance((first.0 * 3.25, first.1 * 3.25), *second) < 0.002);
    }
}

/// 【线条测试】【中心线截取】按弧长截取时保留精确基点，退化重复点不影响切线
#[test]
fn sections_and_tangents_ignore_consecutive_duplicate_points() {
    let line = Polyline::new(vec![(0.0, 0.0), (0.0, 0.0), (30.0, 0.0), (30.0, 40.0)]).unwrap();
    assert_eq!(line.length, 70.0);
    assert_eq!(
        line.section(20.0, 50.0),
        [(20.0, 0.0), (30.0, 0.0), (30.0, 20.0)]
    );
    assert_eq!(line.tangent(60.0), (0.0, 1.0));
    assert!(Polyline::new(vec![(0.0, 0.0), (0.0, 0.0)]).is_none());
}

/// 【线条测试】【容量约束】达到控制点容量后拒绝继续插入，中心线重采样仍保持有限数量
#[test]
fn skeleton_capacity_and_sample_count_remain_bounded() {
    let mut line = skeleton(&[]);
    for index in 0..MAX_CONTROLS {
        assert!(line.insert_control(
            index + 2,
            (40.0 + index as f32 * 2.5, 70.0 + (index % 2) as f32 * 30.0)
        ));
    }
    assert!(!line.insert_control(MAX_CONTROLS + 2, (200.0, 120.0)));
    assert!(line.valid());
    assert!(geometry::sampled(&line).unwrap().points.len() <= 513);
    assert!(line.remove_control(2));
    assert!(line.insert_control(2, (80.0, 90.0)));
    line.controls.push((0.0, 0.0));
    assert!(!line.valid());
    assert!(geometry::path(&line).is_none());
}

/// 【线条测试】【非法输入】非有限坐标、负容差和越界定点不产生路径或破坏原骨架
#[test]
fn invalid_geometry_is_rejected_without_panicking() {
    assert!(LineSkeleton::new((f32::NAN, 0.0), (10.0, 0.0)).is_none());
    let mut line = skeleton(&[]);
    let before = line.clone();
    assert!(!line.set_point(usize::MAX, (1.0, 2.0)));
    assert!(!line.insert_control(1, (1.0, 2.0)));
    assert!(!line.insert_control(2, (f32::INFINITY, 2.0)));
    assert_eq!(line, before);
    assert!(geometry::insertion_at(&line, (200.0, 70.0), -1.0).is_none());
    assert!(geometry::insertion_at(&line, (f32::NAN, 70.0), 1.0).is_none());
    line.style_scale = 0.0;
    assert!(geometry::sampled(&line).is_none());
}

/// 【线条测试】【线型兼容】线型标识、原版编号和描边间隔互相独立
#[test]
fn line_styles_preserve_reference_ids_and_patterns() {
    for (index, style) in LineStyle::ALL.into_iter().enumerate() {
        assert_eq!(style as usize, index);
        assert_eq!(LineStyle::parse(style.key()), Some(style));
        assert_eq!(LineStyle::parse(&index.to_string()), Some(style));
        assert_eq!(style.stroke(4.0).dash.is_some(), style != LineStyle::Solid);
    }
    assert_eq!(LineStyle::parse("missing"), None);
}
