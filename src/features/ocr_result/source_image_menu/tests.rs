//! 【原图菜单测试】【原版与隔离】校验实际 Qt 行和独立原生帧，旧身份与菜单输入不改正文

use crate::{
    backend::window::{RIGHT_BUTTON, WindowAction, WindowContent, WindowEvent},
    features::ocr_result::{
        document::OcrDocument,
        layout::Control,
        window::{ResultWindow, View},
    },
};

/// 【原图菜单测试】【固定窗口】创建原文 text 的离线结果，不调用模型或桌面，返回窗口状态
fn window(text: &str) -> ResultWindow {
    ResultWindow::new(OcrDocument {
        text: text.into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(321, 153).unwrap()),
        lines: vec![],
    })
    .unwrap()
}

/// 【原图菜单测试】【真实菜单】按冻结实际 Qt 几何与两行名称比较原生菜单，旧编号取消不能回收新菜单，无参数、无返回值
#[test]
fn original_image_menu_matches_rows_geometry_and_stale_delivery_isolated() {
    let mut state = window("alpha");
    state.control(Control::View(View::Image));
    let action = state.event(WindowEvent::PointerButton {
        position: (100.0, 150.0),
        button: RIGHT_BUTTON,
        pressed: true,
    });
    assert_eq!(action, WindowAction::Redraw);
    let old = state.popup_id().unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-source-preview.json"
    ))
    .unwrap();
    let menu = state.source_menu.as_ref().unwrap();
    assert_eq!(
        menu.layout.size,
        (
            expected["menu"]["size"][0].as_f64().unwrap() as f32,
            expected["menu"]["size"][1].as_f64().unwrap() as f32
        )
    );
    for (index, row) in menu.rows.iter().enumerate() {
        assert_eq!(
            row.label,
            expected["menu"]["rows"][index]["label"].as_str().unwrap()
        );
        assert!(row.enabled);
        let rect = menu.layout.rows[index];
        let actual = [rect.x(), rect.y(), rect.width(), rect.height()];
        let expected: Vec<_> = expected["menu"]["rows"][index]["rect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() as f32)
            .collect();
        assert_eq!(actual.as_slice(), expected);
    }
    let frame = state.popup_frame(1.2).unwrap();
    assert_eq!(frame.size, (168, 66));
    crate::types::popup::validate(&frame).unwrap();
    assert!(state.popup_frame(1.2).is_none());
    super::open(&mut state, (120.0, 140.0));
    let current = state.popup_id().unwrap();
    assert_ne!(old, current);
    assert_eq!(state.popup_closed(old, Some(0)), WindowAction::None);
    assert_eq!(state.popup_id(), Some(current));
    state.popup_closed(current, None);
    assert_eq!(state.source.text(), "alpha");
}

/// 【原图菜单测试】【标题复制资格】所有页面的标题复制只检查原文，不以原图存在或表格数据代替资格，无参数、无返回值
#[test]
fn title_copy_availability_always_belongs_to_source_text() {
    for text in ["", " \n\t", "alpha"] {
        let mut state = window(text);
        for view in [View::Text, View::Image, View::Table] {
            state.view = view;
            assert_eq!(
                state.control_enabled(Control::Copy),
                !text.trim().is_empty()
            );
        }
    }
}

/// 【原图菜单测试】【非编辑输入】原图焦点不能接收正文或触发系统字库扫描，无参数、无返回值
#[test]
fn image_page_character_input_does_not_load_editor_fonts() {
    let mut state = window("alpha");
    state.view = View::Image;
    let faces = state.fonts.db().len();
    assert_eq!(
        state.event(WindowEvent::Key(crate::backend::window::KeyInput {
            key: Some(crate::keys::Key::Char('你')),
            text: "你".into(),
            mods: crate::keys::Mods::default(),
        })),
        WindowAction::None
    );
    assert_eq!(state.fonts.db().len(), faces);
    assert_eq!(state.source.text(), "alpha");
}

/// 【原图菜单测试】【原图事务】左键抓取、失焦取消和页面恢复不改原图，右键菜单不接收正文输入，无参数、无返回值
#[test]
fn image_drag_focus_loss_and_view_switch_preserve_source_snapshot() {
    let mut state = window("alpha");
    let png = state.png.clone();
    state.view = View::Image;
    state.event(WindowEvent::Scroll {
        position: (100.0, 150.0),
        delta: (0.0, -15.0),
    });
    state.event(WindowEvent::PointerButton {
        position: (100.0, 150.0),
        button: crate::backend::window::LEFT_BUTTON,
        pressed: true,
    });
    assert!(state.preview.dragging);
    state.event(WindowEvent::Focused(false));
    assert!(!state.preview.dragging);
    let zoom = state.preview.zoom;
    state.control(Control::View(View::Text));
    state.control(Control::View(View::Image));
    assert_eq!(state.preview.zoom, zoom);
    assert_eq!(state.png, png);
    assert_eq!(state.source.text(), "alpha");
}
