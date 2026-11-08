//! 【文字格式测试】【原版差分】字体局部撤销不改工具默认值和持久对象，恢复后输入沿用文档格式

use super::*;
use crate::{
    editor::{
        text_edits::{self, Property},
        text_session::PREVIEW_ID,
    },
    tools::Tool,
    types::AnnotationShape,
};
use cosmic_text::Edit;
use serde_json::{Value, json};

/// 【文字格式测试】【样本准备】按 existing 返回已开始的文字会话；默认普通二十点，已有正文使用原版样本
fn prepared(existing: bool) -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    state.tool_settings.text_family = "Inter".into();
    state.tool_settings.bold = false;
    state.tool_settings.italic = false;
    state.selected_tool = Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    if existing {
        super::text::add_text(&mut state);
        if let AnnotationShape::Text { content, bold, .. } = &mut state.annotations[0].shape {
            *content = "Mark Shot".into();
            *bold = false;
        }
        crate::editor::annotation_drag::rebuild_annotation(&mut state, 0);
        state.selected_tool = Tool::Pick;
        crate::editor::text_session::begin_existing(&mut state, 7, &mut 0);
    } else {
        crate::editor::text_session::begin_new(&mut state, (300.0, 200.0), 1.0, &mut 0);
    }
    state
}

/// 【文字格式测试】【字体读取】从 annotation 返回点字号和字体属性，不把工具默认值当作文档当前格式
fn font(annotation: &crate::types::Annotation) -> Value {
    let AnnotationShape::Text {
        font_size,
        bold,
        italic,
        appearance,
        ..
    } = &annotation.shape
    else {
        panic!("text required")
    };
    json!({"points":font_size,"family":appearance.family,"weight":if *bold {600} else {400},"italic":italic})
}

/// 【文字格式测试】【状态读取】读取 state 的实际正文、字体缓冲、局部历史、工具默认值及持久对象，返回与原版同义状态
fn observed(state: &mut EditorState) -> Value {
    let family = state.text_session.as_ref().map(|session| {
        let AnnotationShape::Text {
            appearance,
            bold,
            italic,
            ..
        } = &session.working.shape
        else {
            unreachable!()
        };
        crate::fonts::text_faces::family(
            &mut state.text.font_system,
            &appearance.family,
            *bold,
            *italic,
        )
    });
    let session = state.text_session.as_ref();
    let mut spans = Vec::new();
    let mut cursor = 0;
    let mut selection = [0, 0];
    if let Some(session) = session {
        let editor = &state.text.editors[&PREVIEW_ID];
        editor.with_buffer(|buffer| {
            let format = font(&session.working);
            let AnnotationShape::Text {
                font_size,
                appearance,
                ..
            } = &session.working.shape
            else {
                unreachable!()
            };
            assert_eq!(
                buffer.metrics().font_size,
                crate::features::text::geometry::pixels(*font_size, appearance.size_scale)
            );
            let mut offset = 0;
            for line in &buffer.lines {
                let attrs = line.attrs_list().defaults();
                assert_eq!(attrs.weight.0, format["weight"].as_u64().unwrap() as u16);
                assert_eq!(
                    attrs.style == cosmic_text::Style::Italic,
                    format["italic"].as_bool().unwrap()
                );
                assert_eq!(
                    attrs.family,
                    cosmic_text::Family::Name(family.as_deref().unwrap())
                );
                if !line.text().is_empty() {
                    let mut entry = format.clone();
                    entry["text"] = json!(line.text());
                    entry["position"] = json!(offset);
                    spans.push(entry);
                }
                offset += line.text().len() + 1;
            }
            let position = |cursor: cosmic_text::Cursor| {
                buffer.lines[..cursor.line]
                    .iter()
                    .map(|line| line.text().len() + 1)
                    .sum::<usize>()
                    + cursor.index
            };
            cursor = position(editor.cursor());
            selection = editor
                .selection_bounds()
                .map_or([cursor, cursor], |(a, b)| [position(a), position(b)]);
        });
    }
    let tools = &state.tool_settings;
    let annotations = state
        .annotations
        .iter()
        .map(|annotation| {
            let mut result = font(annotation);
            let AnnotationShape::Text { content, .. } = &annotation.shape else {
                unreachable!()
            };
            result["text"] = json!(content);
            result
        })
        .collect::<Vec<_>>();
    let mut result = json!({"editor_text":session.map_or("",|session|if let AnnotationShape::Text{content,..}=&session.working.shape {content} else {""}),
        "editor_cursor":cursor,"selection":selection,"editor_visible":session.is_some(),
        "editor_undo":session.is_some_and(|session|session.history.available(false)),
        "editor_redo":session.is_some_and(|session|session.history.available(true)),"spans":spans,"annotations":annotations,
        "defaults":{"points":tools.font_size,"family":tools.text_family,"weight":if tools.bold {600} else {400},"italic":tools.italic}});
    if let Some(session) = session {
        result["current_font"] = font(&session.working);
    }
    result
}

/// 【文字格式测试】【参考投影】从 value 保留实际语义字段；对象编号与探针窗口几何不作为跨实现格式断言
fn expected(value: &Value) -> Value {
    let mut value = value.clone();
    let keys = [
        "editor_text",
        "editor_cursor",
        "selection",
        "editor_visible",
        "editor_undo",
        "editor_redo",
        "spans",
        "annotations",
        "defaults",
        "current_font",
    ];
    value
        .as_object_mut()
        .unwrap()
        .retain(|key, _| keys.contains(&key.as_str()));
    for annotation in value["annotations"].as_array_mut().unwrap() {
        annotation.as_object_mut().unwrap().remove("id");
    }
    for key in ["color", "background"] {
        value["defaults"].as_object_mut().unwrap().remove(key);
    }
    value["defaults"]["points"] = json!(value["defaults"]["points"].as_f64().unwrap());
    for key in ["annotations", "spans"] {
        for entry in value[key].as_array_mut().unwrap() {
            entry["points"] = json!(entry["points"].as_f64().unwrap());
        }
    }
    if value["editor_visible"] == false {
        value.as_object_mut().unwrap().remove("current_font");
    } else {
        value["current_font"]["points"] = json!(value["current_font"]["points"].as_f64().unwrap());
    }
    value
}

/// 【文字格式测试】【正文发送】向 state 生产输入路由发送 text，dirty 返回重绘范围，无返回值
fn input(state: &mut EditorState, text: &str, dirty: &mut u32) {
    super::super::handle_key(state, None, Some(text.into()), dirty);
}

/// 【文字格式测试】【实际样式撤销】二十五组真实 Qt 格式、撤销、重做、继续输入和提交状态与生产入口逐项一致，无参数、无返回值
#[test]
fn production_format_history_matches_actual_qt_states() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-text-format-history.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 25);
    let mut results = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        // 1. 【文字属性】【确认规则】字号确认现在保留正文会话，采用字段导航的连续编辑参考记录
        let case = if name == "field-submit" {
            cases
                .iter()
                .find(|case| case["name"] == "field-navigation")
                .unwrap()
        } else {
            case
        };
        let existing = name.starts_with("existing-") || name == "initial-existing";
        let mut state = prepared(existing);
        let mut dirty = 0;
        if !existing && name != "empty" && !name.starts_with("initial-") {
            input(
                &mut state,
                if [
                    "sequential",
                    "resume-empty",
                    "confirm-again",
                    "middle",
                    "field-navigation",
                    "field-submit",
                ]
                .contains(&name)
                {
                    "alpha"
                } else {
                    "alpha beta"
                },
                &mut dirty,
            );
        }
        let mut states = vec![observed(&mut state)];
        if matches!(name, "field-navigation" | "field-submit") {
            super::super::refresh_panels(&mut state, &mut dirty);
            crate::app::panels::text::font_popup::toggle(&mut state, &mut dirty);
            key(&mut state, "Tab", &mut dirty);
            input(&mut state, "30", &mut dirty);
            key(
                &mut state,
                if name == "field-submit" {
                    "Return"
                } else {
                    "Left"
                },
                &mut dirty,
            );
            assert!(
                state.finish.is_none(),
                "{name} must not export the screenshot"
            );
            assert!(
                state.font_panel.open,
                "{name} must not close the font panel"
            );
            states.push(observed(&mut state));
            crate::app::panels::text::font_popup::close(&mut state, &mut dirty);
            for chord in ["Ctrl+Z", "Ctrl+Shift+Z"] {
                key(&mut state, chord, &mut dirty);
                states.push(observed(&mut state));
            }
        } else if name.starts_with("initial-") {
            for _ in 0..3 {
                key(&mut state, "Ctrl+Z", &mut dirty);
                states.push(observed(&mut state));
            }
        } else if ["empty", "resume-empty", "confirm-again", "middle"].contains(&name) {
            text_edits::set(&mut state, Property::Points(30.0), true, &mut dirty);
            super::super::refresh_panels(&mut state, &mut dirty);
            states.push(observed(&mut state));
            key(&mut state, "Ctrl+Z", &mut dirty);
            states.push(observed(&mut state));
            match name {
                "resume-empty" => key(&mut state, "Ctrl+Z", &mut dirty),
                "confirm-again" => {
                    text_edits::set(&mut state, Property::Points(30.0), true, &mut dirty)
                }
                "middle" => {
                    key(&mut state, "Home", &mut dirty);
                    key(&mut state, "Right", &mut dirty);
                }
                _ => {}
            }
            super::super::refresh_panels(&mut state, &mut dirty);
            states.push(observed(&mut state));
            input(&mut state, "x", &mut dirty);
            states.push(observed(&mut state));
            for chord in ["Ctrl+Z", "Ctrl+Shift+Z"] {
                key(&mut state, chord, &mut dirty);
                states.push(observed(&mut state));
            }
        } else if name == "sequential" {
            for property in [Property::Bold(true), Property::Italic(true)] {
                text_edits::set(&mut state, property, true, &mut dirty);
                super::super::refresh_panels(&mut state, &mut dirty);
                states.push(observed(&mut state));
            }
            for chord in [
                "Ctrl+Z",
                "Ctrl+Z",
                "Ctrl+Z",
                "Ctrl+Z",
                "Ctrl+Shift+Z",
                "Ctrl+Shift+Z",
                "Ctrl+Shift+Z",
                "Ctrl+Shift+Z",
            ] {
                key(&mut state, chord, &mut dirty);
                states.push(observed(&mut state));
            }
        } else {
            key(&mut state, "Ctrl+A", &mut dirty);
            states.push(observed(&mut state));
            match name.split_once('-').unwrap().1 {
                "bold" => text_edits::set(&mut state, Property::Bold(true), true, &mut dirty),
                "italic" => text_edits::set(&mut state, Property::Italic(true), true, &mut dirty),
                "family" => text_edits::set(
                    &mut state,
                    Property::Family("Liberation Sans".into()),
                    true,
                    &mut dirty,
                ),
                "size" => text_edits::set(&mut state, Property::Points(30.0), true, &mut dirty),
                "same-size" => {
                    text_edits::set(&mut state, Property::Points(20.0), true, &mut dirty)
                }
                "color" => crate::editor::color_selection::apply_color_selection(
                    &mut state,
                    Color::from_rgba8(255, 77, 77, 128),
                    true,
                    &mut dirty,
                ),
                "background" => {
                    state.color_popover.target =
                        crate::ui::color_popover::ColorTarget::TextBackground;
                    crate::editor::color_selection::apply_color_selection(
                        &mut state,
                        Color::from_rgba8(255, 77, 77, 128),
                        true,
                        &mut dirty,
                    );
                }
                "opacity" => {
                    let mut color = if existing {
                        state.annotations[0].color
                    } else {
                        state.tool_settings.color
                    };
                    color.set_alpha(230.0 / 255.0);
                    crate::editor::color_selection::apply_color_opacity(
                        &mut state, color, true, &mut dirty,
                    );
                }
                _ => unreachable!(),
            }
            super::super::refresh_panels(&mut state, &mut dirty);
            states.push(observed(&mut state));
            for chord in ["Ctrl+Z", "Ctrl+Shift+Z"] {
                key(&mut state, chord, &mut dirty);
                states.push(observed(&mut state));
            }
            key(&mut state, "Ctrl+Z", &mut dirty);
            input(&mut state, "x", &mut dirty);
            states.push(observed(&mut state));
            key(&mut state, "Ctrl+Z", &mut dirty);
            states.push(observed(&mut state));
            key(&mut state, "Ctrl+Return", &mut dirty);
            states.push(observed(&mut state));
            state.undo(&mut dirty);
            states.push(observed(&mut state));
        }
        assert_eq!(
            states.len(),
            case["states"].as_array().unwrap().len(),
            "{name} state count"
        );
        for (index, (actual, reference)) in states
            .iter()
            .zip(case["states"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(*actual, expected(reference), "{name} state {index}");
        }
        results.push(json!({"name":name,"states":states}));
    }
    if let Ok(directory) = std::env::var("MARK_SHOT_FORMAT_ARTIFACTS") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("actual-formats.json"),
            serde_json::to_vec_pretty(&results).unwrap(),
        )
        .unwrap();
    }
}

/// 【文字格式测试】【字段导航】字号字段导航与刷新不追加同值格式记录，关闭面板后一次撤销恢复旧字体但保留默认值，无参数、无返回值
#[test]
fn size_field_navigation_does_not_add_duplicate_format_history() {
    let mut state = prepared(false);
    let mut dirty = 0;
    input(&mut state, "alpha", &mut dirty);
    super::super::refresh_panels(&mut state, &mut dirty);
    crate::app::panels::text::font_popup::toggle(&mut state, &mut dirty);
    key(&mut state, "Tab", &mut dirty);
    input(&mut state, "30", &mut dirty);
    key(&mut state, "Left", &mut dirty);
    key(&mut state, "Escape", &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(
        font(&state.text_session.as_ref().unwrap().working)["points"],
        json!(20.0)
    );
    assert_eq!(state.tool_settings.font_size, 30.0);
    assert!(state.annotations.is_empty());
    assert!(state.undo_stack.is_empty());
}

/// 【文字格式测试】【透明度路由】透明度滑条只改变属性颜色，不重设文档字体或追加格式撤销；一次正文撤销仍清空输入，无参数、无返回值
#[test]
fn opacity_slider_does_not_insert_a_document_format_undo() {
    let mut state = prepared(false);
    let mut dirty = 0;
    input(&mut state, "alpha", &mut dirty);
    super::super::refresh_panels(&mut state, &mut dirty);
    let cell = state
        .settings_panel
        .widget_rect(crate::ui::text_properties::layout::Control::Opacity.index())
        .unwrap();
    pointer(
        &mut state,
        (cell.left() + 1.0) as f64,
        (cell.top() + 7.0) as f64,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(state.tool_settings.color.alpha() < 1.0);
    key(&mut state, "Ctrl+Z", &mut dirty);
    let AnnotationShape::Text { content, .. } = &state.text_session.as_ref().unwrap().working.shape
    else {
        unreachable!()
    };
    assert_eq!(content, "");
    assert!(state.tool_settings.color.alpha() < 1.0);
}
