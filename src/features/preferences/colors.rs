//! 【颜色设置】【选择事务】所有颜色字段共用原生取色对话框，取消不保存，确认保留透明度

use super::{catalog::ENTRIES, window::Preferences};
use crate::{
    renderer::Painter,
    theme::Rgba,
    ui::color_popover::{
        self,
        window::{Completion, Selection},
    },
};
use serde_json::Value;
use tiny_skia::Rect;

pub(super) const DIALOG_ID: u64 = 0x434f4c4f52;
pub(super) struct Edit {
    index: usize,
    completion: Completion,
}

/// 【颜色设置】【实际颜色】按values和index返回生效颜色，未指定标注色时读取工具默认颜色
fn current(values: &Value, index: usize) -> Rgba {
    Rgba::parse_hex(&ENTRIES[index].display(values))
        .unwrap_or_else(crate::types::tool_settings::default_color)
}

/// 【颜色设置】【字段预览】按painter、bounds、values和index绘制真实色块与色值，focused控制边框
pub(super) fn draw(
    painter: &mut Painter<'_>,
    bounds: Rect,
    values: &Value,
    index: usize,
    focused: bool,
) {
    let color = current(values, index);
    super::widgets::field(painter, bounds, "", focused, false);
    color_popover::draw_swatch(
        painter,
        Rect::from_xywh(
            bounds.left() + 8.0,
            bounds.top() + 6.0,
            26.0,
            bounds.height() - 12.0,
        )
        .unwrap(),
        color,
    );
    let label = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.0, color.1, color.2, color.3
    );
    painter.text(
        Rect::from_xywh(
            bounds.left() + 43.0,
            bounds.top(),
            (bounds.width() - 48.0).max(1.0),
            bounds.height(),
        )
        .unwrap(),
        &label,
        crate::renderer::text::TextStyle::new(12.0, crate::theme::window::Palette::current().text),
    );
}

/// 【颜色设置】【打开组件】按state和index创建共享取色对话框，保持配置直到用户确认
pub(super) fn open(state: &mut Preferences, index: usize) {
    let (dialog, completion) =
        color_popover::window::dialog(DIALOG_ID, current(&state.values, index), state.chinese());
    state.color_edit = Some(Edit { index, completion });
    state.dialog = Some(dialog);
}

/// 【颜色设置】【提交结果】按state消费对话框结果，取消保持配置，恢复默认沿用现有继承规则
pub(super) fn finish(state: &mut Preferences) {
    let Some(edit) = state.color_edit.take() else {
        return;
    };
    let selected = edit.completion.lock().unwrap().take();
    match selected {
        Some(Selection::Color(color)) => {
            state.save(
                edit.index,
                format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    color.0, color.1, color.2, color.3
                )
                .into(),
            );
        }
        Some(Selection::Default) => {
            state.save(edit.index, Value::Null);
        }
        None => {}
    }
}
