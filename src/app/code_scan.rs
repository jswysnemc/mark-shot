//! 【扫码】【选区动作】提交标注后把当前成品区域交给独立识别窗口

use crate::{editor::EditorState, ui::toast::ToastKind};

/// 【扫码】【编辑器交接】按 state 导出当前选区，成功后退出覆盖层，失败通过 dirty 更新错误提示
pub(crate) fn open(state: &mut EditorState, dirty: &mut u32) {
    if state.selection.zone.is_none()
        || state.tool_active
        || state.input.mouse_down
        || state.pending.is_some()
        || state.ann_drag.is_some()
    {
        return;
    }
    let result = super::render_final(state)
        .ok_or_else(|| "无法生成扫码图片 / Cannot render scan image".into())
        .and_then(|(png, _, _)| crate::features::code_scan::transport::spawn(&png));
    match result {
        Ok(()) => state.cancel = true,
        Err(error) => {
            log::error!("【扫码】【选区交接】{error}");
            state
                .toasts
                .show(ToastKind::ScanFailed, &mut state.text.font_system);
            crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
        }
    }
}
