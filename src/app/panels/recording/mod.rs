//! 【录屏工具】【属性交互】管理本次参数菜单、工具激活与明确开始的区域交接

pub(crate) mod input;

use crate::{
    editor::EditorState,
    features::recording::{Format, tool_options::Parameter, tool_state::Menu},
    tools::Tool,
    types::Mode,
    ui::{
        controls::Choice,
        panel::{UiPanel, emit_panel_damage},
        settings_panel::{SettingsAction, SettingsWidget},
    },
};

/// 【录屏工具】【场景可用】按 state 返回是否为允许屏幕录制的截图会话
pub(crate) fn available(state: &EditorState) -> bool {
    state.capture_start.recording_controls && state.file_canvas.is_none()
}

/// 【录屏工具】【激活】按 state 保留截图选区并显示编辑界面，dirty 返回面板变化；不提交开始请求
pub(crate) fn activate(state: &mut EditorState, dirty: &mut u32) {
    state.select_annotation_ids([]);
    state.mode = Mode::Editor;
    state.ui_hidden = false;
    state.finish = None;
    state.toolbar.dirty = true;
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}

/// 【录屏工具】【格式入口】按 format 为 state 设置本次格式并选择工具，dirty 返回重绘，不保存全局偏好
pub(crate) fn select(state: &mut EditorState, format: Format, dirty: &mut u32) {
    if !available(state) || !state.capture_start.can_record() {
        return;
    }
    state.recording_tool.config.format = format.resolve(&state.recording_tool.config.format);
    state.capture_start.recording = Some(format);
    crate::app::select_tool(state, Tool::Recording, dirty);
}

/// 【录屏工具】【属性同步】按 state 同步小工具栏参数与开始状态；dirty 返回旧菜单损坏，无返回值
pub(crate) fn sync(state: &mut EditorState, dirty: &mut u32) {
    if state.selected_tool != Tool::Recording || state.ui_hidden || state.tool_active {
        close(state, dirty);
    }
    if state.selected_tool != Tool::Recording {
        return;
    }
    let ready = available(state)
        && state.capture_start.can_record()
        && crate::features::recording::selection::validate_region(state).is_ok();
    let changed = state.settings_panel.recording != state.recording_tool.config;
    if changed || state.settings_panel.recording_ready != ready {
        state.settings_panel.recording = state.recording_tool.config.clone();
        state.settings_panel.recording_ready = ready;
        state.settings_panel.dirty = true;
        if let Some(rect) = state.settings_panel.rect() {
            emit_panel_damage(
                rect,
                state.settings_panel.monitor_idx,
                &mut state.damage_rects,
                dirty,
            );
        }
    }
    if changed
        || state.settings_panel.active_source
            != Some(crate::ui::settings_panel::SettingsSource::Tool(
                Tool::Recording,
            ))
    {
        let cn = crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
        let text = [
            Parameter::Format,
            Parameter::Fps,
            Parameter::Countdown,
            Parameter::Audio,
            Parameter::Quality,
            Parameter::Encoder,
            Parameter::Cursor,
        ]
        .iter()
        .map(|parameter| {
            format!(
                "{}{}",
                parameter.label(cn),
                parameter.choices(&state.recording_tool.config, cn).join("")
            )
        })
        .collect::<String>()
            + if cn {
                "开始录制"
            } else {
                "Start recording"
            };
        crate::fonts::ui_fallback::ensure_text(&mut state.text.font_system, &text);
    }
}

/// 【录屏工具】【菜单切换】按 parameter 在 state 对应属性按钮展开统一短菜单，dirty 返回新旧损坏，无返回值
pub(crate) fn toggle(state: &mut EditorState, parameter: Parameter, dirty: &mut u32) {
    let same = state
        .recording_tool
        .menu
        .as_ref()
        .is_some_and(|menu| menu.parameter == parameter);
    close(state, dirty);
    if same
        || state.selected_tool != Tool::Recording
        || !parameter.enabled(&state.recording_tool.config)
    {
        return;
    }
    let Some(index) = state.settings_panel.widgets.iter().position(|widget| matches!(widget,
        SettingsWidget::Action { action: SettingsAction::Recording(candidate), .. } if *candidate == parameter
    )) else { return; };
    let Some(anchor) = state.settings_panel.widget_rect(index) else {
        return;
    };
    let anchor = tiny_skia::Rect::from_xywh(
        anchor.left(),
        anchor.top(),
        anchor.width().max(172.0),
        anchor.height(),
    )
    .unwrap();
    let cn = crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    state.recording_tool.menu = Some(Menu {
        parameter,
        monitor: state.settings_panel.monitor_idx,
        choice: Choice::new(
            anchor,
            parameter.choices(&state.recording_tool.config, cn),
            parameter.selected(&state.recording_tool.config),
        ),
    });
    damage(state, dirty);
}

/// 【录屏工具】【菜单关闭】按 state 清除菜单，dirty 返回原菜单区域；关闭后不改变参数
pub(crate) fn close(state: &mut EditorState, dirty: &mut u32) {
    damage(state, dirty);
    state.recording_tool.menu = None;
}

/// 【录屏工具】【菜单损坏】按 state 标记当前菜单范围，dirty 返回所属显示器，无返回值
pub(super) fn damage(state: &mut EditorState, dirty: &mut u32) {
    if let Some(menu) = &state.recording_tool.menu {
        let monitor = menu.monitor;
        let size = state.placements[monitor].size;
        let rect = menu.choice.bounds((size.0 as u32, size.1 as u32));
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
}

/// 【录屏工具】【开始确认】校验 state 单屏区域与当前参数后提交请求，dirty 返回反馈；失败保留编辑界面
pub(crate) fn start(state: &mut EditorState, dirty: &mut u32) {
    if state.selected_tool != Tool::Recording
        || !available(state)
        || state.tool_active
        || state.input.mouse_down
        || state.upload.busy()
        || state.selection_translation.busy()
    {
        return;
    }
    let result = (|| -> crate::AppResult {
        if !state.capture_start.can_record() {
            return Err("录制尚未就绪或已有会话 / Recording unavailable or already active".into());
        }
        crate::features::recording::selection::validate_region(state)?;
        state.recording_tool.config.validate()?;
        Ok(())
    })();
    if let Err(error) = result {
        state.toasts.show_text(
            crate::ui::toast::ToastKind::RecordingFeedback,
            error.to_string(),
            &mut state.text.font_system,
        );
        crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
        return;
    }
    // 1. 【录屏工具】【参数快照】仅明确开始才提交配置，截图保存或关闭不启动录制
    log::info!(
        "【录屏工具】【开始录制】格式={} 帧率={} 倒计时={} 声音={} 画质={} 编码器={} 光标={}",
        state.recording_tool.config.format,
        state.recording_tool.config.frame_rate(),
        state.recording_tool.config.countdown,
        state.recording_tool.config.audio,
        state.recording_tool.config.quality,
        state.recording_tool.config.encoder,
        state.recording_tool.config.cursor
    );
    state.recording_tool.request = Some(state.recording_tool.config.clone());
    state.finish = Some(state.to);
    close(state, dirty);
}

/// 【录屏工具】【菜单绘制】按 state 在 monitor 绘制菜单，无返回值，使用屏幕逻辑坐标
pub(crate) fn draw(state: &mut EditorState, monitor: usize) {
    let Some(menu) = state
        .recording_tool
        .menu
        .as_ref()
        .filter(|menu| menu.monitor == monitor)
    else {
        return;
    };
    let placement = &state.placements[monitor];
    let mut painter = crate::renderer::Painter::new(&mut state.canvas[monitor])
        .with_scale(placement.scale)
        .with_icons(&state.icons_cache)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
    menu.choice.draw(
        &mut painter,
        (placement.size.0 as u32, placement.size.1 as u32),
    );
}
