//! 【滚动截图】【运行循环】真实捕获、按需重绘、暂停和结果操作共用一个期限循环

use super::{Outcome, PendingOutcome, ScrollRun, capture, completion::Completion, input, render};
use crate::backend::session::Session as Connection;
use crate::{
    backend::ScreenOverlay,
    editor::EditorState,
    keys::{Key, Mods},
    scroll::{
        panel::{self, PanelButton, preview::PreviewState},
        session::ScrollSession,
    },
    types::{CursorIcon, OverlayEvent},
};
use std::time::Instant;

/// 【滚动截图】【完整会话】按 conn、state 和 overlay 捕获当前选区，返回取消、标注原图或已交付结果
pub(super) async fn run(
    conn: &Connection,
    state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
) -> crate::AppResult<Outcome> {
    let zone = state
        .selection
        .zone
        .ok_or("scroll capture needs a selected region")?;
    let index = state
        .placements
        .iter()
        .position(|placement| {
            let (x, y) = (placement.position.0 as f32, placement.position.1 as f32);
            zone.left() >= x
                && zone.top() >= y
                && zone.right() <= x + placement.size.0 as f32
                && zone.bottom() <= y + placement.size.1 as f32
        })
        .ok_or("scroll capture needs a region inside one display")?;
    let placement = &state.placements[index];
    let region = crate::tools::selection::global_selection_to_local(&zone, placement)
        .ok_or("invalid scroll capture region")?;
    let bounds = (placement.size.0 as f32, placement.size.1 as f32);
    let mut run = ScrollRun {
        session: ScrollSession::new(region, index)?,
        layout: panel::layout(region, bounds, None),
        hovered: None,
        pressed: None,
        pointer: (0.0, 0.0),
        axis_drag: None,
        overview_drag: None,
        preview: PreviewState::new(),
        scale: placement.scale,
        dirty: true,
        needs_clean_frame: false,
        panel_limits: None,
    };
    overlay.set_popup(None)?;
    overlay.set_text_input(None)?;
    overlay.configure_scroll_panel(run.layout.rect, false)?;
    overlay.begin_scroll(index)?;
    super::resources::release(state, index);
    overlay.set_cursor(CursorIcon::Default);
    render::paint(state, &mut run, overlay.as_mut())?;
    let capture = crate::backend::initialize_capture(conn);
    let output = overlay
        .surface_output(index)
        .ok_or("the scroll capture output is gone")?
        .clone();
    let capture = capture
        .open_session(&[output])
        .await
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    log::info!("【滚动截图】【会话开始】区域={region:?}，目标表面={index}");
    let mut task = super::task::Task::new(capture, overlay.task_wake()?)?;
    let outcome = drive(state, overlay, &mut run, bounds, &mut task).await;
    let mut capture = task.finish()?;
    // 1. 【滚动截图】【统一关闭】完成、取消和错误都关闭持续授权与视频线程，不能留下后台捕获
    if let Err(error) = capture.close().await {
        log::error!("【滚动截图】【捕获关闭】{error}");
    }
    // 2. 【滚动截图】【交付前回收】先释放捕获帧和旧会话，逐块转换不能与完整捕获资源同时常驻
    drop(capture);
    drop(run);
    crate::utils::memory::release_unused();
    let outcome = match outcome? {
        PendingOutcome::Cancelled => Outcome::Cancelled,
        PendingOutcome::Annotate { image, monitor_idx } => Outcome::Annotate {
            image: crate::scroll::source::consume(image)
                .ok_or("cannot allocate the scroll image")?,
            monitor_idx,
        },
        PendingOutcome::Copy { image, scale } => Outcome::Copy {
            png: crate::scroll::png::consume(image)?,
            scale,
        },
        PendingOutcome::Saved(path) => Outcome::Saved(path),
    };
    // 3. 【滚动截图】【交付后回收】原块已消耗或完成编码，释放其空闲堆页后再打开结果编辑器或复制进程
    crate::utils::memory::release_unused();
    Ok(outcome)
}

/// 【滚动截图】【事件驱动】按 state、overlay、run 和 bounds 驱动已建立的 capture，返回用户操作，不重开授权
async fn drive(
    state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
    run: &mut ScrollRun,
    bounds: (f32, f32),
    task: &mut super::task::Task,
) -> crate::AppResult<PendingOutcome> {
    let index = run.session.monitor_idx;
    let mut completion = Completion::default();
    loop {
        let now = Instant::now();
        if let Some(result) = task.take() {
            match result
                .map_err(|error| error as Box<dyn std::error::Error>)
                .and_then(|result| capture::apply(run, overlay.as_mut(), result))
            {
                Ok(changed) => run.dirty |= changed,
                Err(error) => {
                    run.needs_clean_frame = false;
                    completion.cancel();
                    log::error!("【滚动截图】【帧捕获】{error}");
                    run.session.set_paused(true);
                    run.session.status = "Capture error".into();
                    run.dirty = true;
                }
            }
            render::layout(state, run);
        }
        if let Some(outcome) = completion.finish(
            task.busy() || run.needs_clean_frame,
            state,
            overlay.as_mut(),
            run,
        )? {
            return Ok(outcome);
        }
        // 1. 【滚动截图】【预览避让】没有安全放置空间时捕获期间隐藏预览，静止后暂停并显示操作
        if ((!run.layout.fits_without_overlap && !overlay.is_scroll_window())
            || overlay.scroll_controls_hidden())
            && run.session.should_idle(Instant::now())
            && !task.busy()
            && !run.needs_clean_frame
        {
            run.session.set_paused(true);
            run.session.status = "Capture paused".into();
            run.dirty = true;
        }
        if run.dirty && overlay.ready(index) {
            render::paint(state, run, overlay.as_mut())?;
        }
        // 2. 【滚动截图】【呈现顺序】先确认隐藏或移动后的浮层已呈现，再读取下一帧，不能依靠拼接器拒绝污染帧
        if !task.busy()
            && run.axis_drag.is_none()
            && overlay.ready(index)
            && (run.session.wants_capture(now) || run.needs_clean_frame)
        {
            task.request()?;
        }
        let timeout = if !overlay.ready(index) {
            Some(crate::backend::FRAME_FALLBACK)
        } else if run.axis_drag.is_some() || task.busy() {
            None
        } else {
            run.session.next_capture(Instant::now())
        };
        let timeout = if run.dirty {
            Some(timeout.map_or(crate::backend::FRAME_FALLBACK, |time| {
                time.min(crate::backend::FRAME_FALLBACK)
            }))
        } else {
            timeout
        };
        let event = overlay.next_event(timeout)?;
        let action = match event {
            OverlayEvent::ThemeChanged => {
                super::super::theme_switch::refresh(state, &mut 0);
                run.dirty = true;
                None
            }
            OverlayEvent::ScrollPanelConfigured { .. } => {
                run.panel_limits = overlay.scroll_panel_limits(index);
                render::layout(state, run);
                run.dirty = true;
                None
            }
            OverlayEvent::EscapePressed => Some(PanelButton::Cancel),
            OverlayEvent::Key {
                chord: Some(chord), ..
            } => {
                if chord.mods
                    == (Mods {
                        ctrl: true,
                        shift: true,
                        ..Mods::default()
                    })
                    && chord.key == Key::Char('l')
                {
                    super::super::theme_switch::toggle(state, &mut 0);
                    run.dirty = true;
                    None
                } else if chord.mods == Mods::default() {
                    match chord.key {
                        Key::Enter => Some(PanelButton::Annotate),
                        Key::Escape => Some(PanelButton::Cancel),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            OverlayEvent::PointerMove {
                monitor_idx, x, y, ..
            } if monitor_idx == index => {
                if overlay.is_scroll_window() {
                    input::window_motion(run, (x, y), bounds);
                } else {
                    input::motion(run, (x, y), bounds);
                }
                if run.axis_drag.is_some() {
                    completion.cancel();
                    task.cancel();
                }
                None
            }
            OverlayEvent::PointerButton {
                button, pressed, ..
            } => input::button(run, button, pressed),
            OverlayEvent::Scroll(event) => {
                input::wheel(run, event);
                None
            }
            _ => None,
        };
        match action {
            Some(PanelButton::Cancel) => return Ok(PendingOutcome::Cancelled),
            Some(PanelButton::TogglePause) => {
                // 3. 【滚动截图】【暂停收尾】停止新请求但接收已在途帧，不能丢弃暂停前已经捕获的末段
                completion.cancel();
                run.session.set_paused(!run.session.paused);
                run.dirty = true;
            }
            Some(PanelButton::ToggleAxis) => {
                completion.cancel();
                task.cancel();
                run.session.toggle_axis();
                run.dirty = true;
            }
            Some(button @ (PanelButton::Annotate | PanelButton::Copy | PanelButton::Save))
                if run.session.frame_count > 0 =>
            {
                completion.request(button, run);
            }
            _ => {}
        }
    }
}
