//! 【原生窗口】【主题刷新】同连接父窗口、模态窗口和菜单共享切换结果

use super::runtime::NativeWindow;

impl NativeWindow {
    /// 【原生窗口】【主题切换】按 chord 执行统一主题动作并刷新全部存活表面，返回是否消费快捷键
    pub(super) fn theme_key(&mut self, chord: Option<crate::keys::Chord>) -> bool {
        if !chord.is_some_and(crate::theme::is_toggle) {
            return false;
        }
        if let Err(error) = crate::theme::toggle() {
            log::error!("【原生窗口】【主题切换】无法保存主题: {error}");
            return true;
        }
        self.refresh_theme();
        true
    }

    /// 【原生窗口】【存活刷新】无参数，标记父窗口与模态表面重绘，菜单按当前主题更新缓存，保留原生抓取，无返回值
    pub(super) fn refresh_theme(&mut self) {
        self.main.dirty = true;
        if let Some(dialog) = &mut self.dialog {
            dialog.surface.dirty = true;
        }
    }
}
