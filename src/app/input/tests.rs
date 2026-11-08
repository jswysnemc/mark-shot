//! 【输入测试】【环境构造】提供不连接显示服务器、不加载识别模型的编辑器与输入辅助方法

mod annotation_collection;
mod annotation_collection_lifecycle;
mod annotation_collection_magnifier;
mod annotation_collection_reference;
mod annotation_collection_render;
mod annotation_rotation;
mod annotation_selection;
mod annotation_style;
mod canvas_navigation;
mod color;
mod color_sample;
mod cursor_magnifier;
mod external_actions;
mod filter;
mod filter_output;
mod filter_properties;
mod filter_support;
mod history;
mod interaction_parity;
mod line;
mod line_properties;
mod line_support;
mod magnifier;
mod marker;
mod native_field_input;
mod native_field_reference;
mod native_field_support;
mod native_text_input;
mod number;
mod pen_cache;
mod pointer_buttons;
mod ruler;
mod selection_handles;
mod selection_size;
mod selection_size_clicks;
mod selection_size_navigation;
mod selection_size_pointer;
mod selection_translation;
mod text;
mod text_caret;
mod text_composition;
mod text_context_menu;
mod text_format_history;
mod text_properties;
mod text_session;
mod text_size_policy;
mod text_storage;
mod upload;
mod wheel_preview;

use std::collections::HashMap;

use crate::editor::{EditorState, Layers, OcrState, TextState};
use crate::types::{Capture, Placement};
use crate::ui::panel::{PanelItem, UiPanel};
use tiny_skia::{Color, Pixmap};

/// 【输入测试】【编辑器构造】返回单显示器编辑器；只加载内置字体，识别运行时保持未初始化
pub(in crate::app) fn editor() -> EditorState {
    let placement = Placement {
        position: (0, 0),
        size: (640, 480),
        scale: 1.0,
    };
    let mut image = Pixmap::new(640, 480).unwrap();
    image.fill(Color::from_rgba8(30, 34, 38, 255));
    let mut editor = EditorState::new(
        Layers {
            captures: vec![Capture {
                pixmap: image.clone().into(),
                scale: 1.0,
            }],
            canvas: vec![image.clone()],
            dimmed: vec![image],
            annotations: vec![Pixmap::new(640, 480).unwrap()],
        },
        vec![placement],
        crate::app::init::load_icons_cache(),
        TextState {
            font_system: crate::fonts::bundled(),
            system_fonts: None,
            swash_cache: cosmic_text::SwashCache::new(),
            editors: HashMap::new(),
            editing: None,
        },
        OcrState {
            runtime: crate::ocr::OcrRuntime::new(crate::ocr::settings::Mode::OnDemand),
            models: crate::ocr::models::OcrModels::load(),
            view: crate::ocr::OcrView::default(),
            redrag: false,
            redrag_from: None,
            await_region: false,
            scan_started: None,
            result_scale: 1.0,
            open_result_on_completion: false,
            copy_on_completion: false,
        },
    );
    editor.magnifier.hidden = true;
    editor
}

/// 【输入测试】【指针输入】在 state 的单显示器内移动到 x、y；dirty 返回重绘显示器，无返回值
fn pointer(state: &mut EditorState, x: f64, y: f64, dirty: &mut u32) {
    super::handle_pointer_move(state, 0, x, y, dirty);
}

/// 【输入测试】【按键输入】解析 value 并发送给 state；dirty 返回重绘显示器，无返回值
fn key(state: &mut EditorState, value: &str, dirty: &mut u32) {
    super::handle_key(state, Some(crate::keys::parse(value).unwrap()), None, dirty);
}

/// 【输入测试】【控件中心】返回 state 属性面板 index 对应控件的本地中心，供真实输入分派测试使用
fn widget_center(state: &EditorState, index: usize) -> (f64, f64) {
    let panel = &state.settings_panel;
    let offset: f32 = panel.widgets[..index]
        .iter()
        .map(|widget| widget.size() + widget.trailing_padding())
        .sum();
    (
        (panel.render_pos.0 + panel.padding() + offset + panel.widgets[index].size() / 2.0) as f64,
        (panel.render_pos.1 + panel.size.1 / 2.0) as f64,
    )
}
