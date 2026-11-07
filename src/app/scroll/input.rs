//! 【滚动截图】【输入事务】非左键不操作按钮，左键释放交付，方向握柄按轴移动选区

use super::ScrollRun;
use crate::{
    scroll::panel::{self, PanelButton, preview},
    types::{MouseButton, ScrollEvent},
};
use shot_stitch::Axis;
use tiny_skia::Rect;

pub(super) struct AxisDrag {
    pub start: (f64, f64),
    pub panel_origin: (f64, f64),
    pub region: Rect,
    pub moved: bool,
}

/// 【滚动截图】【普通窗口移动】按 position 的当前面板坐标处理 run，bounds 保持原输出；拖动期间使用起始原点，避免布局变化重复累计位移
pub(super) fn window_motion(run: &mut ScrollRun, position: (f64, f64), bounds: (f32, f32)) {
    let adjusted = run.axis_drag.as_ref().map_or(position, |drag| {
        (
            position.0 - run.layout.rect.left() as f64 + drag.panel_origin.0,
            position.1 - run.layout.rect.top() as f64 + drag.panel_origin.1,
        )
    });
    motion(run, adjusted, bounds);
    run.pointer = position;
}

/// 【滚动截图】【鼠标移动】按 position 和 bounds 更新 run 的悬停、方向拖动或概览位置，无返回值
pub(super) fn motion(run: &mut ScrollRun, position: (f64, f64), bounds: (f32, f32)) {
    run.pointer = position;
    if let Some(drag) = &mut run.axis_drag {
        let delta = (position.0 - drag.start.0, position.1 - drag.start.1);
        drag.moved |= delta.0.abs() >= 5.0 || delta.1.abs() >= 5.0;
        if drag.moved {
            let (x, y) = if run.session.axis() == Axis::Vertical {
                (drag.region.left(), drag.region.top() + delta.1 as f32)
            } else {
                (drag.region.left() + delta.0 as f32, drag.region.top())
            };
            if let Some(region) = Rect::from_xywh(
                x.clamp(0.0, (bounds.0 - drag.region.width()).max(0.0)),
                y.clamp(0.0, (bounds.1 - drag.region.height()).max(0.0)),
                drag.region.width(),
                drag.region.height(),
            ) {
                run.session.move_region(region);
                run.dirty = true;
            }
        }
        return;
    }
    if let Some(offset) = run.overview_drag {
        scrub(run, position, offset);
        return;
    }
    let hovered = panel::hit_test(&run.layout, position);
    if hovered != run.hovered {
        run.hovered = hovered;
        run.dirty = true;
    }
}

/// 【滚动截图】【鼠标按钮】按 button 和 pressed 更新 run 的左键事务，返回释放后真正激活的操作
pub(super) fn button(
    run: &mut ScrollRun,
    button: MouseButton,
    pressed: bool,
) -> Option<PanelButton> {
    if button != MouseButton::Left {
        return None;
    }
    if pressed {
        run.pressed = run
            .hovered
            .filter(|button| panel::is_button_enabled(*button, &run.session));
        if run.pressed == Some(PanelButton::ToggleAxis) {
            run.axis_drag = Some(AxisDrag {
                start: run.pointer,
                panel_origin: (run.layout.rect.left() as f64, run.layout.rect.top() as f64),
                region: run.session.region,
                moved: false,
            });
        } else if let Some(layout) = view(run)
            && let Some(rect) = preview::overview_rect(&layout, run.session.stitcher.dimensions())
            && panel::contains(rect, run.pointer)
        {
            run.preview.sync(&layout);
            let point = preview::scrub_at(run.pointer, rect, &layout, 0.0);
            let offset =
                if point >= run.preview.scrub && point <= run.preview.scrub + layout.viewport {
                    point - run.preview.scrub
                } else {
                    layout.viewport / 2.0
                };
            run.overview_drag = Some(offset);
            scrub(run, run.pointer, offset);
        }
        run.dirty = true;
        return None;
    }
    if let Some(drag) = run.axis_drag.take()
        && drag.moved
    {
        log::info!("【滚动截图】【区域调整】区域={:?}", run.session.region);
        run.pressed = None;
        run.dirty = true;
        return None;
    }
    run.overview_drag = None;
    let activated = run.pressed.take().filter(|button| {
        run.hovered == Some(*button) && panel::is_button_enabled(*button, &run.session)
    });
    run.dirty = true;
    activated
}

/// 【滚动截图】【预览滚轮】按 event 的轴来源更新 run 的原图浏览位置，纯像素保持位移，离散刻度按视口八分之一换算
pub(super) fn wheel(run: &mut ScrollRun, event: ScrollEvent) {
    let Some(layout) = view(run) else {
        return;
    };
    let axis = if layout.horizontal
        && (event.horizontal.pixels != 0.0
            || event.horizontal.value120 != 0
            || event.horizontal.discrete != 0)
    {
        event.horizontal
    } else {
        event.vertical
    };
    let steps = if axis.value120 != 0 {
        axis.value120 as f32 / 120.0
    } else {
        axis.discrete as f32
    };
    let delta = if steps != 0.0 {
        steps * (layout.viewport / 8.0).max(24.0)
    } else {
        axis.pixels as f32
    };
    // 1. 【滚动截图】【横轴方向】覆盖层横轴保留协议方向，预览统一使用原版向左为正的浏览刻度
    let delta = if layout.horizontal
        && (event.horizontal.pixels != 0.0
            || event.horizontal.value120 != 0
            || event.horizontal.discrete != 0)
    {
        -delta
    } else {
        delta
    };
    run.preview.sync(&layout);
    run.dirty |= run.preview.set(run.preview.scrub - delta, &layout);
}

/// 【滚动截图】【浏览布局】按 run 的图像和面板返回当前细节映射，没有原图时返回空值
fn view(run: &ScrollRun) -> Option<preview::PreviewLayout> {
    preview::layout(
        run.layout.preview?,
        run.session.stitcher.dimensions(),
        run.session.axis(),
    )
}

/// 【滚动截图】【概览拖动】按 position 与 offset 更新 run 的原图浏览位置，无返回值
fn scrub(run: &mut ScrollRun, position: (f64, f64), offset: f32) {
    let Some(layout) = view(run) else {
        return;
    };
    let Some(rect) = preview::overview_rect(&layout, run.session.stitcher.dimensions()) else {
        return;
    };
    run.dirty |= run
        .preview
        .set(preview::scrub_at(position, rect, &layout, offset), &layout);
}
