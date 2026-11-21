//! 【文字识别】【快速复制】复用选区识别结果，成功复制全文后结束截图

use crate::{editor::EditorState, tools::Tool, ui::toast::ToastKind};

/// 【文字识别】【快速请求】按 state 当前选区开始识别或复用缓存，dirty 接收重绘范围，无返回值；失败保留编辑器
pub(crate) fn request(state: &mut EditorState, dirty: &mut u32) {
    // 1. 【文字识别】【复用入口】已有结果必须与当前选区一致，工具切换继续使用模型选择和选区提示
    crate::app::input::select_tool(state, Tool::Ocr, dirty);
    if state.ocr.view.is_active() && state.ocr.view.region() == state.selection.zone {
        complete(state, dirty);
    } else {
        state.ocr.quick_copy = true;
    }
}

/// 【文字识别】【全文交付】复制 state 的全部识别文字，dirty 接收失败提示范围，无返回值；空文字不覆盖剪贴板
pub(crate) fn complete(state: &mut EditorState, dirty: &mut u32) {
    // 1. 【文字识别】【结果选择】结束快速请求并取得全部文字，空结果保留编辑器
    state.ocr.quick_copy = false;
    let _ = state.ocr.view.select_all();
    let text = state.ocr.view.text_to_copy();
    if text.is_empty() {
        return;
    }
    // 2. 【文字识别】【结果复制】复制成功后结束截图，失败在当前编辑器显示提示
    match crate::utils::copy_to_clipboard(&text) {
        Ok(()) => {
            log::info!(
                "【文字识别】【快速复制】已复制 {} 行文字",
                text.lines().count()
            );
            state.cancel = true;
        }
        Err(error) => {
            log::error!("【文字识别】【快速复制】{error}");
            state
                .toasts
                .show(ToastKind::CopyFailed, &mut state.text.font_system);
        }
    }
    // 3. 【文字识别】【界面更新】结果或提示变化后重新绘制所有截图表面
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}
