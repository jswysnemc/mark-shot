use std::time::{Duration, Instant};

use mark_shot::features::laser_trace::{
    LIFETIME, LaserTrace, MAX_POINTS_PER_STROKE, MAX_STROKES, MAX_TOTAL_POINTS, draw,
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【激光测试】【画布边界】返回包含负坐标的固定逻辑画布
fn bounds() -> Rect {
    Rect::from_xywh(-100.0, -100.0, 1000.0, 800.0).unwrap()
}

/// 【激光测试】【笔迹构造】以 now 提交固定红色笔迹；返回可绘制状态
fn trace(now: Instant) -> LaserTrace {
    let mut trace = LaserTrace::default();
    trace.begin(
        (40.0, 60.0),
        Color::from_rgba8(255, 40, 20, 255),
        10.0,
        bounds(),
    );
    trace.extend((180.0, 60.0));
    trace.commit(now);
    trace
}

/// 【激光测试】【单击过滤】单点草稿不成为完成笔迹，也不启动消散计时
#[test]
fn clicking_does_not_create_an_animation() {
    let mut trace = LaserTrace::default();
    trace.begin((20.0, 20.0), Color::WHITE, 10.0, bounds());
    assert!(!trace.is_animating());
    trace.commit(Instant::now());
    assert!(trace.is_empty());
    assert_eq!(trace.point_count(), 0);
}

/// 【激光测试】【生命周期】草稿始终可见，松开后才开始完整的 1.8 秒消散
#[test]
fn lifetime_starts_on_release_and_fades_linearly() {
    let start = Instant::now();
    let release = start + Duration::from_secs(30);
    let mut trace = LaserTrace::default();
    trace.begin((20.0, 20.0), Color::WHITE, 10.0, bounds());
    trace.extend((80.0, 20.0));
    assert_eq!(trace.strokes().next().unwrap().opacity(release), 1.0);
    assert!(!trace.is_animating());
    trace.commit(release);
    assert!(trace.is_animating());
    let stroke = trace.strokes().next().unwrap();
    assert_eq!(stroke.opacity(release), 1.0);
    assert!((stroke.opacity(release + LIFETIME / 2) - 0.5).abs() < 0.0001);
    assert_eq!(stroke.opacity(release + LIFETIME), 0.0);
}

/// 【激光测试】【过期清理】最后一次消散返回旧区域，随后释放点数组并恢复空闲
#[test]
fn expiration_damages_the_last_frame_then_stops() {
    let now = Instant::now();
    let mut trace = trace(now);
    let old = trace.strokes().next().unwrap().bounds();
    assert_eq!(trace.tick(now + LIFETIME / 2), Some(old));
    assert_eq!(trace.tick(now + LIFETIME), Some(old));
    assert!(trace.is_empty());
    assert!(!trace.is_animating());
    assert_eq!(trace.point_count(), 0);
    assert_eq!(trace.tick(now + LIFETIME * 2), None);
}

/// 【激光测试】【距离插值】小幅抖动不追加点，远距离移动最多插入 24 个中间点
#[test]
fn pointer_sampling_limits_jitter_and_intermediate_points() {
    let mut trace = LaserTrace::default();
    trace.begin((20.0, 20.0), Color::WHITE, 10.0, bounds());
    assert_eq!(trace.extend((20.5, 20.0)), None);
    assert_eq!(trace.point_count(), 1);
    assert!(trace.extend((800.0, 600.0)).is_some());
    assert_eq!(trace.point_count(), 26);
    assert_eq!(
        trace.strokes().next().unwrap().points().last(),
        Some(&(800.0, 600.0))
    );
}

/// 【激光测试】【坐标约束】所有采样点限制在源图范围，无效坐标不破坏现有草稿
#[test]
fn source_bounds_and_nonfinite_input_are_respected() {
    let mut trace = LaserTrace::default();
    trace.begin((-500.0, 2000.0), Color::WHITE, 10.0, bounds());
    assert_eq!(trace.strokes().next().unwrap().points(), &[(-100.0, 700.0)]);
    trace.extend((2000.0, -500.0));
    assert_eq!(
        trace.strokes().next().unwrap().points().last(),
        Some(&(900.0, -100.0))
    );
    let count = trace.point_count();
    assert_eq!(trace.extend((f32::NAN, 30.0)), None);
    assert_eq!(
        trace.begin((20.0, 30.0), Color::WHITE, f32::INFINITY, bounds()),
        None
    );
    assert_eq!(trace.point_count(), count);
}

/// 【激光测试】【辉光边界】损坏区域覆盖三层描边和抗锯齿，不留下外侧残影
#[test]
fn damage_includes_the_outer_glow() {
    let now = Instant::now();
    let trace = trace(now);
    let area = trace.strokes().next().unwrap().bounds();
    assert!(area.left() <= 40.0 - 13.0);
    assert!(area.right() >= 180.0 + 13.0);
    assert!(area.top() <= 60.0 - 13.0);
    assert!(area.bottom() >= 60.0 + 13.0);
}

/// 【激光测试】【草稿取消】切换工具只删除未提交笔迹，不提前删除完成笔迹
#[test]
fn canceling_the_draft_keeps_committed_traces() {
    let now = Instant::now();
    let mut trace = trace(now);
    trace.begin((400.0, 200.0), Color::WHITE, 10.0, bounds());
    trace.extend((600.0, 200.0));
    let draft = trace.strokes().last().unwrap().bounds();
    assert_eq!(trace.cancel_draft(), Some(draft));
    assert_eq!(trace.strokes().count(), 1);
    assert!(trace.is_animating());
    assert_eq!(trace.cancel_draft(), None);
}

/// 【激光测试】【笔迹数量】快速重复手势受到数量限制，淘汰区域参与重绘
#[test]
fn completed_strokes_have_a_bounded_count() {
    let now = Instant::now();
    let mut trace = LaserTrace::default();
    for index in 0..MAX_STROKES + 1 {
        let x = index as f32 * 10.0;
        trace.begin((x, 20.0), Color::WHITE, 3.0, bounds());
        trace.extend((x + 5.0, 20.0));
        let damage = trace.commit(now).unwrap();
        if index == MAX_STROKES {
            assert!(damage.left() < 0.0);
        }
    }
    assert_eq!(trace.strokes().count(), MAX_STROKES);
    assert!(trace.point_count() <= MAX_TOTAL_POINTS);
}

/// 【激光测试】【内存限制】长时间手势保留起点和终点，同时限制单笔迹与总采样量
#[test]
fn long_gestures_and_total_geometry_are_bounded() {
    let now = Instant::now();
    let mut trace = LaserTrace::default();
    for index in 0..10 {
        trace.begin((20.0, 20.0), Color::WHITE, 3.0, bounds());
        for step in 0..180 {
            let x = if step % 2 == 0 { 800.0 } else { 40.0 };
            trace.extend((x, 40.0 + index as f32));
            assert!(trace.strokes().last().unwrap().points().len() <= MAX_POINTS_PER_STROKE);
            assert!(trace.point_count() <= MAX_TOTAL_POINTS);
        }
        let points = trace.strokes().last().unwrap().points();
        assert_eq!(points.first(), Some(&(20.0, 20.0)));
        assert_eq!(points.last(), Some(&(40.0, 40.0 + index as f32)));
        trace.commit(now);
        assert!(trace.point_count() <= MAX_TOTAL_POINTS);
    }
}

/// 【激光测试】【分层渲染】主线包含白色细芯，外侧辉光保持原始颜色且透明度更低
#[test]
fn rendering_has_a_hot_core_and_colored_glow() {
    let now = Instant::now();
    let trace = trace(now);
    let mut pixels = Pixmap::new(220, 120).unwrap();
    draw(&mut pixels, &trace, (0.0, 0.0), None, 1.0, now);
    let core = pixels.pixel(100, 60).unwrap();
    let glow = pixels.pixel(100, 69).unwrap();
    assert!(core.alpha() > glow.alpha());
    assert!(core.green() > glow.green());
    assert!(glow.red() > glow.green());
    assert!(glow.alpha() > 0);
    assert_eq!(pixels.pixel(100, 78).unwrap().alpha(), 0);
}

/// 【激光测试】【颜色透明度】配置透明度与消散比例同时影响三层笔迹
#[test]
fn opacity_and_fade_both_reduce_the_rendered_alpha() {
    let now = Instant::now();
    let trace = trace(now);
    let mut full = Pixmap::new(220, 120).unwrap();
    let mut faded = full.clone();
    draw(&mut full, &trace, (0.0, 0.0), None, 1.0, now);
    draw(
        &mut faded,
        &trace,
        (0.0, 0.0),
        None,
        1.0,
        now + LIFETIME / 2,
    );
    assert!(full.pixel(100, 60).unwrap().alpha() > faded.pixel(100, 60).unwrap().alpha());
    let mut transparent = LaserTrace::default();
    transparent.begin((40.0, 60.0), Color::TRANSPARENT, 10.0, bounds());
    transparent.extend((180.0, 60.0));
    let mut empty = Pixmap::new(220, 120).unwrap();
    draw(&mut empty, &transparent, (0.0, 0.0), None, 1.0, now);
    assert!(empty.data().iter().all(|byte| *byte == 0));
}

/// 【激光测试】【局部重绘】分数缩放和全局偏移下，裁切渲染与完整重绘一致且不叠加外部透明度
#[test]
fn dirty_rendering_matches_a_full_frame_without_touching_other_pixels() {
    let now = Instant::now();
    let trace = trace(now);
    let origin = (-10.0, -5.0);
    let scale = 1.2;
    let dirty = Rect::from_xywh(80.3, 40.2, 50.4, 45.6).unwrap();
    let mut full = Pixmap::new(280, 160).unwrap();
    full.fill(Color::BLACK);
    let mut partial = full.clone();
    draw(&mut full, &trace, origin, None, scale, now);
    draw(&mut partial, &trace, origin, Some(&dirty), scale, now);
    let area = mark_shot::renderer::scale_rect(&dirty, scale).unwrap();
    let (left, top, width, height) = mark_shot::renderer::rect_bounds(&area, 280, 160).unwrap();
    for y in 0..160 {
        for x in 0..280 {
            let inside = x >= left && x < left + width && y >= top && y < top + height;
            let expected = if inside {
                full.pixel(x, y).unwrap()
            } else {
                Color::BLACK.to_color_u8().premultiply()
            };
            assert_eq!(partial.pixel(x, y).unwrap(), expected, "pixel ({x}, {y})");
        }
    }
}

/// 【激光测试】【独立笔宽】激光设置不修改持久画笔或其他工具的笔宽
#[test]
fn laser_stroke_width_is_independent_of_persistent_tools() {
    use mark_shot::types::{StrokeGroup, ToolSettings};
    let mut settings = ToolSettings::default();
    let old = settings.clone();
    settings.set_stroke(StrokeGroup::Laser, 17.0);
    assert_eq!(settings.stroke(StrokeGroup::Laser), 17.0);
    for group in StrokeGroup::ALL
        .into_iter()
        .filter(|group| *group != StrokeGroup::Laser)
    {
        assert_eq!(settings.stroke(group), old.stroke(group));
    }
    assert_eq!(
        StrokeGroup::of_tool(mark_shot::tools::Tool::Laser),
        StrokeGroup::Laser
    );
    assert_eq!(StrokeGroup::Laser.key(), "laser");
}

/// 【激光测试】【工具接入】工具栏与共用设置面板均包含激光，不仅支持命令行解析
#[test]
fn laser_has_a_toolbar_entry_and_stroke_settings() {
    use mark_shot::tools::Tool;
    use mark_shot::ui::settings_panel::{SettingsWidget, StepperRange, widgets_for_tool};
    use mark_shot::ui::toolbar::{ToolbarButton, ToolbarItem};
    assert!(
        mark_shot::ui::toolbar::ITEMS
            .iter()
            .any(|item| matches!(item, ToolbarItem::Button(ToolbarButton::Tool(Tool::Laser))))
    );
    assert!(widgets_for_tool(Tool::Laser).iter().any(|widget| matches!(
        widget,
        SettingsWidget::Stepper {
            range: StepperRange::Stroke,
            ..
        }
    )));
}
