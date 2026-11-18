//! 【原生贴图】【业务动作】菜单关闭后交付变换、识别及输出

use super::*;
use crate::config::pinned_preferences::Preference;

impl Pin {
    /// 【原生贴图】【菜单动作】按action变换原图或交付输出，返回重绘或关闭，保存和编辑使用共用业务入口
    pub(super) fn action(&mut self, action: PinAction) -> crate::AppResult<WindowAction> {
        self.action_with(action, crate::config::pinned_preferences::save)
    }

    /// 【原生贴图】【动作提交】按 action 执行业务，persist 保存菜单偏好，返回窗口动作或业务错误
    pub(super) fn action_with(
        &mut self,
        action: PinAction,
        persist: impl FnOnce(Preference, bool) -> std::io::Result<()>,
    ) -> crate::AppResult<WindowAction> {
        use crate::features::pinned_image::actions;
        if !self.enabled(action) {
            return Ok(WindowAction::None);
        }
        match action {
            PinAction::RotateLeft | PinAction::RotateRight => {
                let clockwise = action == PinAction::RotateRight;
                self.clear_text();
                self.image.rotate(clockwise)?;
                self.feedback.rotated(clockwise);
                if self.text_selection {
                    self.start_ocr()?;
                }
            }
            PinAction::ZoomIn | PinAction::ZoomOut | PinAction::ResetSize => {
                let zoom = if action == PinAction::ResetSize {
                    1.0
                } else {
                    self.image.zoom
                        * if action == PinAction::ZoomIn {
                            1.18
                        } else {
                            1.0 / 1.18
                        }
                };
                self.image.scale(zoom);
                self.feedback.zoom(self.image.zoom);
            }
            PinAction::AlwaysOnTop => {
                let enabled = !self.image.always_on_top;
                // 1. 【原生贴图】【偏好提交】保存成功后才修改当前开关，写入失败沿用原状态并交给统一反馈
                persist(Preference::AlwaysOnTop, enabled)?;
                self.image.always_on_top = enabled;
                self.feedback.topmost(self.image.always_on_top);
            }
            PinAction::Edit => actions::open(&self.image.png()?, false)?,
            PinAction::ScanCode => crate::features::code_scan::transport::spawn_from_pin(
                &self.image.png()?,
                self.image.always_on_top,
            )?,
            PinAction::Copy => {
                crate::backend::initialize_clipboard()
                    .copy_image_to_clipboard(&self.image.png()?)?;
                self.feedback.copied_image();
            }
            PinAction::CopySelectedText => {
                crate::utils::copy_to_clipboard(&self.text.line_text(true))?;
                self.feedback.copied_text();
            }
            PinAction::CopyImageText => {
                if self.text.is_active() {
                    let content = self.text.line_text(false);
                    if content.is_empty() {
                        self.feedback.no_text();
                    } else {
                        crate::utils::copy_to_clipboard(&content)?;
                        self.feedback.copied_text();
                    }
                } else {
                    self.copy_after_ocr = true;
                    if !self.start_ocr()? {
                        self.copy_after_ocr = false;
                    }
                }
            }
            PinAction::TextSelection => {
                let enabled = !self.text_selection;
                persist(Preference::AutoTextSelection, enabled)?;
                self.text_selection = enabled;
                if self.text_selection && !self.text.is_active() {
                    self.text_selection = self.start_ocr()?;
                } else {
                    self.feedback.text_selection(self.text_selection);
                }
                self.text.deselect();
                self.selecting = false;
            }
            PinAction::Translate => self.request_translation()?,
            PinAction::OpenTranslationWindow => self.open_translation_window()?,
            PinAction::ToggleTranslation => {
                self.translation.active = !self.translation.active;
                self.sync_translation_text();
                self.feedback.translation_view(self.translation.active);
            }
            PinAction::TargetLanguage => self.open_menu(true),
            PinAction::SaveAs => actions::save(self.image.png()?, self.image.density)?,
            PinAction::Close => return Ok(WindowAction::Close),
        }
        Ok(WindowAction::Redraw)
    }
    /// 【原生贴图】【可用动作】按action返回依赖当前文字、翻译任务及窗口管理能力的可用状态
    pub(super) fn enabled(&self, action: PinAction) -> bool {
        menu::enabled(
            action,
            self.top_supported,
            self.text.has_selection(),
            self.text.is_active(),
            self.translation_task.is_some() || self.translation.after_ocr,
            !self.translation.blocks.is_empty(),
        )
    }
}
