//! 【文字识别测试】【快速入口】验证快捷键、取消边界和空结果不会误交付截图

use super::*;

/// 【文字识别测试】【快捷请求】无参数，Ctrl+Shift+C 进入快速识别且不触发截图复制，无返回值
#[test]
fn shortcut_requests_recognition_without_finishing_image_copy() {
    let mut state = editor();
    let mut dirty = 0;
    key(&mut state, "Ctrl+Shift+C", &mut dirty);
    assert_eq!(state.selected_tool, crate::tools::Tool::Ocr);
    assert!(state.ocr.quick_copy);
    assert!(state.finish.is_none());
    crate::app::input::select_tool(&mut state, crate::tools::Tool::Selection, &mut dirty);
    assert!(!state.ocr.quick_copy);
}

/// 【文字识别测试】【空结果】无参数，空识别结果清理快速请求并保留截图编辑器，无返回值
#[test]
fn empty_recognition_keeps_editor_open() {
    let mut state = editor();
    state.ocr.quick_copy = true;
    crate::tools::ocr::finish_ocr(&mut state, Ok(Default::default()), &mut 0);
    assert!(!state.ocr.quick_copy);
    assert!(!state.cancel);
    assert!(state.finish.is_none());
}

/// 【文字识别测试】【识别结果】无参数，返回两行可复制的识别内容，边界与字符坐标保持一致
#[cfg(target_os = "linux")]
fn recognized() -> crate::ocr::OcrText {
    crate::ocr::OcrText {
        lines: vec![
            crate::ocr::OcrLine {
                text: "First".into(),
                bounds: tiny_skia::Rect::from_xywh(10.0, 10.0, 50.0, 20.0).unwrap(),
                char_x: vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0],
            },
            crate::ocr::OcrLine {
                text: "Second".into(),
                bounds: tiny_skia::Rect::from_xywh(10.0, 35.0, 60.0, 20.0).unwrap(),
                char_x: vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0],
            },
        ],
    }
}

/// 【文字识别测试】【成功复制】无参数，真实完成入口交付全文并结束截图，不设置图像复制请求，无返回值
#[cfg(target_os = "linux")]
#[test]
fn successful_quick_copy_delivers_all_lines_and_closes_editor() {
    use crate::backend::wayland::clipboard::context::{self, Content};
    let delivered = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let output = delivered.clone();
    let _scope = context::enter(move |content| {
        let Content::Text(text) = content else {
            panic!("expected OCR text");
        };
        output.replace(text);
        Ok(())
    });
    let mut state = editor();
    state.ocr.quick_copy = true;
    crate::tools::ocr::finish_ocr(&mut state, Ok(recognized()), &mut 0);
    assert_eq!(*delivered.borrow(), "First\nSecond");
    assert!(state.cancel);
    assert!(!state.ocr.quick_copy);
    assert!(state.finish.is_none());
}

/// 【文字识别测试】【复制失败】无参数，复制错误保留识别结果和编辑器，允许用户重试，无返回值
#[cfg(target_os = "linux")]
#[test]
fn failed_quick_copy_preserves_editor_and_recognized_text() {
    let _scope =
        crate::backend::wayland::clipboard::context::enter(|_| Err("clipboard unavailable".into()));
    let mut state = editor();
    state.ocr.quick_copy = true;
    crate::tools::ocr::finish_ocr(&mut state, Ok(recognized()), &mut 0);
    assert!(!state.cancel);
    assert!(!state.ocr.quick_copy);
    assert!(state.ocr.view.is_active());
    assert!(state.finish.is_none());
}
