//! 【Windows独立窗口】【主题输入】主窗口与存活菜单使用同一快捷键入口，不让主题键触发菜单动作

use super::runtime::Surface;
use crate::{backend::window::WindowAction, keys::Chord, types::popup::PopupEvent};

/// 【Windows独立窗口】【主题快捷键】按surface与chord识别切换请求，返回是否消费；保存失败向调用者返回错误，不关闭当前菜单
pub(super) fn handle(surface: &mut Surface, chord: Option<Chord>) -> crate::AppResult<bool> {
    if !chord.is_some_and(crate::theme::is_toggle) {
        return Ok(false);
    }
    crate::theme::toggle()?;
    surface.apply(WindowAction::Redraw);
    Ok(true)
}

/// 【Windows独立窗口】【菜单快捷键】按event提取实际按键并交给surface处理，返回是否消费，指针和纯文本事件不改变主题
pub(super) fn popup(surface: &mut Surface, event: &PopupEvent) -> crate::AppResult<bool> {
    let PopupEvent::Key { chord, .. } = event else {
        return Ok(false);
    };
    handle(surface, *chord)
}
