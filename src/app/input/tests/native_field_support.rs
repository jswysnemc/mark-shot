//! 【字段输入测试】【应用准备】复用真实截图编辑器、字段生命周期与协议路由，不连接桌面

use super::editor;
use crate::{
    app::{panels, text_input},
    editor::{EditorState, text_session},
    fonts::font_catalog::{Family, FontCatalog},
    types::{
        AnnotationShape, OverlayEvent,
        text_input::{Batch, Focus, Snapshot, Update},
    },
    ui::{
        color_popover::{ColorField, ColorTarget},
        font_panel::state::Field,
        text_field::{CursorInit, LineEditState},
    },
};

/// 【字段输入测试】【字段准备】按 role、text、cursor 和 anchor 构造编辑状态；返回正文为 body、字段独立聚焦的编辑器
pub(super) fn prepared(role: &str, text: &str, cursor: usize, anchor: usize) -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    state.tool_settings.color = tiny_skia::Color::from_rgba8(17, 34, 51, 255);
    state.selected_tool = crate::tools::Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    text_session::begin_new(&mut state, (100.25, 120.5), 1.0, &mut 0);
    crate::tools::text::session_input::text(&mut state, "body", &mut 0);
    super::super::refresh_panels(&mut state, &mut 0);
    state.font_panel.catalog = Some(FontCatalog {
        families: ["Inter", "Inter Bold", "Sample"]
            .map(|name| Family {
                name: name.into(),
                faces: Vec::new(),
            })
            .to_vec(),
    });
    if role == "hex" {
        panels::color::toggle(&mut state, ColorTarget::Main, &mut 0);
        state
            .color_popover
            .select_color(tiny_skia::Color::from_rgba8(17, 34, 51, 255));
        state
            .color_popover
            .fields
            .begin_edit(ColorField::Hex, text.into(), CursorInit::End);
    } else {
        panels::text::font_popup::toggle(&mut state, &mut 0);
        let key = if role == "size" {
            Field::Size
        } else {
            Field::Search
        };
        panels::text::font_popup::focus(&mut state, key, CursorInit::End);
        state.font_panel.fields.set_editing_text(key, text.into());
    }
    let field = field_mut(&mut state);
    field.cursor = cursor;
    field.selection_anchor = (cursor != anchor).then_some(anchor);
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9100,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    state
}

/// 【字段输入测试】【字段读取】返回 state 当前字体或颜色字段，不读取标注正文
pub(super) fn field(state: &EditorState) -> &LineEditState {
    if state.font_panel.open {
        &state.font_panel.fields.editing.as_ref().unwrap().field
    } else {
        &state.color_popover.fields.editing.as_ref().unwrap().field
    }
}

/// 【字段输入测试】【字段借用】返回 state 当前字段的可变状态，供原版初始选择设置使用
pub(super) fn field_mut(state: &mut EditorState) -> &mut LineEditState {
    if state.font_panel.open {
        &mut state.font_panel.fields.editing.as_mut().unwrap().field
    } else {
        &mut state.color_popover.fields.editing.as_mut().unwrap().field
    }
}

/// 【字段输入测试】【正文读取】返回 state 已确认截图标注文字，不含任何字段或候选
pub(super) fn content(state: &EditorState) -> &str {
    let shape = state
        .text_session
        .as_ref()
        .map(|session| &session.working.shape)
        .or_else(|| {
            state.annotations.iter().rev().find_map(|annotation| {
                matches!(annotation.shape, AnnotationShape::Text { .. })
                    .then_some(&annotation.shape)
            })
        })
        .unwrap();
    let AnnotationShape::Text { content, .. } = shape else {
        unreachable!()
    };
    content
}

/// 【字段输入测试】【协议采集】同步 state 并返回实际发送的快照，停用以空值记录
pub(super) fn sync(state: &mut EditorState) -> Vec<Option<Snapshot>> {
    let mut sent = Vec::new();
    text_input::sync_with(
        state,
        |snapshot| {
            sent.push(snapshot.cloned());
            Ok(())
        },
        &mut 0,
    )
    .unwrap();
    sent
}

/// 【字段输入测试】【原生交付】将 batch 按 state 当前目标与序号许可 matching 交给应用事件入口，无返回值
pub(super) fn deliver(state: &mut EditorState, batch: Batch, matching: bool) {
    text_input::event(
        state,
        OverlayEvent::TextInput(Update {
            target_id: state.input.text_input.id.unwrap(),
            serial: 1,
            sync_allowed: matching,
            batch,
        }),
        &mut 0,
    );
}
