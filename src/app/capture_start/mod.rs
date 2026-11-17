//! 【截图起始】【生命周期】选区形成后丢弃起始控件，不因清空选区重新出现

pub(crate) mod input;
mod placement;
#[cfg(test)]
mod tests;

use crate::{
    editor::EditorState, features::recording::Format, types::Mode, ui::capture_start::layout,
};

/// 【截图起始】【启动配置】按 state 和 launch 设置可见性及本次录制类型，不影响文件编辑和专用识别入口
pub(super) fn initialize(state: &mut EditorState, launch: &super::Launch) {
    if let Some(config) = &launch.record_config {
        state.recording_tool.config = config.clone();
    }
    let capture_cfg = &crate::config::preferences::current().capture;
    state.capture_start.recording_controls = launch.image.is_none()
        && !launch.code_scan
        && matches!(launch.mode, Mode::Editor | Mode::Region);
    state.capture_start.visible = state.capture_start.recording_controls
        && !launch.fullscreen
        && (capture_cfg.show_start_bar || launch.record_format.is_some() || launch.record_select);
    state.capture_start.show_hints = capture_cfg.show_hints;
    state.capture_start.recording = launch.record_format.or_else(|| {
        launch.record_select.then(|| {
            if state.recording_tool.config.format == "gif" {
                Format::Gif
            } else {
                Format::Video
            }
        })
    });
}

/// 【截图起始】【录屏参数】按 state 与 format 初始化命令行指定的格式，返回是否可进入录屏工具
pub(super) fn initialize_recording_tool(state: &mut EditorState, format: Format) -> bool {
    if !super::panels::recording::available(state) {
        return false;
    }
    state.recording_tool.config.format = format.resolve(&state.recording_tool.config.format);
    true
}

/// 【截图起始】【控件损坏】按 state 标记所有输出上入口和提示的边界，dirty 返回需要重绘的输出
fn damage(state: &mut EditorState, dirty: &mut u32) {
    for monitor in 0..state.placements.len() {
        let size = state.placements[monitor].size;
        for rect in [
            layout::top(size, true),
            layout::toolbar(size, &state.capture_start, monitor),
            layout::message(size),
        ] {
            state.damage_local(monitor, rect);
        }
        if state.capture_start.show_hints {
            state.damage_local(monitor, layout::hints(size));
        }
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
}

/// 【截图起始】【阶段交接】按 state 的正式选区永久关闭起始界面，dirty 返回旧控件损坏
pub(crate) fn sync(state: &mut EditorState, dirty: &mut u32) {
    if state.capture_start.visible
        && state.selection.zone.is_some()
        && !state.input.mouse_down
        && !state.tool_active
    {
        state.capture_start.visible = false;
        state.capture_start.hovered = None;
        state.capture_start.hint_hovered = None;
        state.capture_start.pressed = None;
        damage(state, dirty);
    }
}

/// 【截图起始】【后台状态】按 state 非阻塞领取录制状态，dirty 返回状态文字变化，不修改截图选区
pub(super) fn poll(state: &mut EditorState, dirty: &mut u32) {
    sync(state, dirty);
    if !state.capture_start.polls_recording() {
        return;
    }
    if let Some(result) = state.capture_start.query.poll() {
        match result {
            Ok(status) => {
                let checked = state.capture_start.checked;
                state.capture_start.checked = true;
                if !checked
                    || status != state.capture_start.status
                    || !state.capture_start.message.is_empty()
                {
                    if status.phase != state.capture_start.status.phase {
                        log::info!("【截图录制】【状态同步】录制阶段={}", status.phase);
                    }
                    state.capture_start.status = status;
                    if !state.capture_start.active() {
                        state.capture_start.stop_requested = false;
                    }
                    state.capture_start.message.clear();
                    // 1. 【截图起始】【布局切换】录制开始或结束会替换按钮，悬停归属按新布局重新计算
                    state.capture_start.hovered =
                        input::hit(state).map(|control| (state.input.pointer.monitor_idx, control));
                    damage(state, dirty);
                }
            }
            Err(error) => {
                log::error!("【截图起始】【录制状态】{error}");
                state.capture_start.message = if crate::ui::language::Language::current()
                    == crate::ui::language::Language::Chinese
                {
                    "录制状态暂不可用".into()
                } else {
                    "Recording status unavailable".into()
                };
                damage(state, dirty);
            }
        }
    }
    if state.selected_tool == crate::tools::Tool::Recording {
        super::panels::settings::update_settings_panel(state, dirty);
    }
    placement::sync(state, dirty);
}
