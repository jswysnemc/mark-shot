//! 【设置窗口】【配置恢复】损坏文件仍能打开窗口，提供明确备份修复操作
use super::{
    layout::{self, rect},
    widgets,
    window::Preferences,
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
};

/// 【设置窗口】【修复按钮】按 state 返回修复操作范围，供绘制和命中共同使用
pub(super) fn button(state: &Preferences) -> tiny_skia::Rect {
    let viewport = layout::viewport(state);
    rect(viewport.left() + 16.0, viewport.top() + 142.0, 172.0, 32.0)
}

/// 【设置窗口】【错误展示】按 state、canvas、scale 显示读取错误与备份说明，无返回值
pub(super) fn draw(state: &mut Preferences, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let area = layout::viewport(state);
    let button = button(state);
    let chinese = state.chinese();
    let title = if chinese {
        "配置文件需要修复"
    } else {
        "Configuration needs repair"
    };
    let detail = if chinese {
        "先备份原文，再移除无效行。损坏的 JSON 将恢复默认。"
    } else {
        "Back up first, then remove invalid lines. Invalid JSON resets to defaults."
    };
    let action = if chinese {
        "备份并修复配置"
    } else {
        "Back up and repair"
    };
    let error = state.recovery.clone().unwrap_or_default();
    state
        .fonts
        .ensure_text(&format!("{title} {detail} {action} {error}"));
    let palette = Palette::current();
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    widgets::panel(&mut painter, area, title);
    for (index, text) in [detail, &error].into_iter().enumerate() {
        let text = crate::ui::controls::fit_label(&mut painter, text, 11.0, area.width() - 32.0);
        painter.text(
            rect(
                area.left() + 16.0,
                area.top() + 50.0 + index as f32 * 32.0,
                area.width() - 32.0,
                28.0,
            ),
            &text,
            TextStyle::new(11.0, palette.text),
        );
    }
    widgets::field(&mut painter, button, action, true, false);
}

impl Preferences {
    /// 【设置窗口】【修复提交】无参数，备份当前文件后恢复可编辑状态，失败保留错误详情
    pub(super) fn repair(&mut self) {
        match crate::config::recovery::repair(&self.path).and_then(|backup| {
            let values = crate::config::preferences::read(&self.path)?;
            Ok((backup, values))
        }) {
            Ok((backup, values)) => {
                self.values = values;
                self.toolbar = super::toolbar::Editor::new(&self.values);
                self.translation = super::translation::State::new(&self.values);
                self.recovery = None;
                self.status = format!(
                    "{}: {}",
                    if self.chinese() {
                        "已修复，备份位置"
                    } else {
                        "Repaired; backup"
                    },
                    backup.display()
                );
            }
            Err(error) => {
                self.recovery = Some(error.to_string());
            }
        }
    }
}
