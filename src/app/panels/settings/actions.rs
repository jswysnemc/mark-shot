//! 【属性面板】【按钮动作】分派识别与绘图属性按钮

use super::update_settings_panel;
use crate::editor::EditorState;
use crate::ui::settings_panel::SettingsAction;

/// 【属性面板】【按钮动作】在 editor_state 中执行 action 并同步面板；dirty_mask 返回重绘显示器，无返回值
pub fn run_settings_action(
    editor_state: &mut EditorState,
    action: SettingsAction,
    dirty_mask: &mut u32,
) {
    match action {
        SettingsAction::FilterModes => super::super::filter::toggle(editor_state, dirty_mask),
        SettingsAction::OcrRescan => crate::tools::ocr::restart_ocr(editor_state, dirty_mask),
        SettingsAction::OcrCopyAll => crate::tools::ocr::copy_all(editor_state, dirty_mask),
        SettingsAction::OcrOpenResult => crate::tools::ocr::open_result(editor_state),
        SettingsAction::OcrLanguages => {
            super::super::model::toggle_model_popover(editor_state, dirty_mask)
        }
        SettingsAction::MarkerShapes => super::super::marker::toggle(editor_state, dirty_mask),
        SettingsAction::RulerModes => super::super::ruler::toggle(editor_state, dirty_mask),
        SettingsAction::NumberStyles => super::super::number::toggle(editor_state, dirty_mask),
        SettingsAction::ResetNumber => {
            crate::editor::number_edits::reset_sequence(editor_state, dirty_mask)
        }
        SettingsAction::ArrowStyles => super::super::line::toggle(
            editor_state,
            crate::ui::line_properties::LineChoice::Heads,
            dirty_mask,
        ),
        SettingsAction::LineStyles => super::super::line::toggle(
            editor_state,
            crate::ui::line_properties::LineChoice::Pattern,
            dirty_mask,
        ),
    }
    update_settings_panel(editor_state, dirty_mask);
}
