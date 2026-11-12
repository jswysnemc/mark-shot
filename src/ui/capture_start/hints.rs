//! 【截图起始】【快捷键提示】按当前有效绑定生成分组提示，只读展示且不拦截输入

use super::layout::{self, Grid, rect};
use crate::{
    editor::EditorState,
    keys::{self, Action},
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};

/// 键位列宽上限，说明文字在同一列对齐
const KEY_COLUMN: f32 = 92.0;

/// 一条提示：按键显示文本与动作说明
pub(crate) struct Hint {
    pub key: String,
    pub title: &'static str,
}

/// 【截图起始】【提示内容】按 state 和 cn 返回选择组与录制组提示，未绑定或当前无效的动作不出现
pub(crate) fn groups(state: &EditorState, cn: bool) -> (Vec<Hint>, Vec<Hint>) {
    let map = keys::map();
    let label = |a: &'static str, b: &'static str| if cn { a } else { b };
    // 1. 【截图起始】【选择组】指针手势在前，其后是选区相关快捷键
    let mut selection = vec![Hint {
        key: label("拖动 / 单击", "Drag / click").into(),
        title: label("选择范围", "Select region"),
    }];
    if state.region_detection.layered() {
        selection.push(Hint {
            key: label("滚轮", "Wheel").into(),
            title: label("切换识别区域", "Cycle detected area"),
        });
    }
    push(
        &mut selection,
        &map,
        Action::SelectAll,
        label("选择全屏", "Select screen"),
    );
    // 2. 【截图起始】【历史选区】逗号与句点在未被用户改绑时仍是历史切换入口
    let free = [",", "."]
        .iter()
        .all(|key| keys::parse(key).is_ok_and(|chord| map.lookup(chord).is_none()));
    if free {
        selection.push(Hint {
            key: ", / .".into(),
            title: label("切换历史选区", "Recent regions"),
        });
    }
    for (action, title) in [
        (Action::Accept, label("完成截图", "Finish capture")),
        (Action::Cancel, label("退出截图", "Exit capture")),
    ] {
        push(&mut selection, &map, action, title);
    }
    // 3. 【截图起始】【录制组】已有会话时提示暂停与停止，否则提示两种录制入口
    let session = if state.capture_start.active() {
        [
            (Action::RecordPause, label("暂停 / 继续", "Pause / resume")),
            (Action::RecordStop, label("停止并保存", "Stop and save")),
        ]
    } else {
        [
            (Action::RecordVideo, label("录屏", "Record video")),
            (Action::RecordGif, label("录 GIF", "Record GIF")),
        ]
    };
    let mut recording = Vec::new();
    for (action, title) in session.into_iter().chain([
        (Action::ToggleMagnifier, label("放大预览", "Magnifier")),
        (Action::ToggleTheme, label("切换主题", "Toggle theme")),
        (Action::History, label("截图历史", "History")),
        (Action::Settings, label("设置", "Settings")),
    ]) {
        push(&mut recording, &map, action, title);
    }
    (selection, recording)
}

/// 【截图起始】【提示追加】按 map 把 action 的最短绑定加入 rows，未绑定时跳过
fn push(rows: &mut Vec<Hint>, map: &keys::Keymap, action: Action, title: &'static str) {
    if let Some(key) = map.label(action) {
        rows.push(Hint { key, title });
    }
}

/// 【截图起始】【面板绘制】用 painter 在 grid 内按 groups 绘制键位与说明，无返回值
///
/// 参数：painter 目标画布、grid 由 layout::hint_grid 算出的网格、
/// groups 选择组与录制组提示、palette 当前窗口配色
pub(crate) fn draw(
    painter: &mut Painter<'_>,
    grid: &Grid,
    groups: &(Vec<Hint>, Vec<Hint>),
    palette: Palette,
) {
    // 1. 【截图起始】【列顺序】两列时选择组在左，单列时选择组在上，超出高度的条目已由网格裁剪
    let mut rows: Vec<(usize, usize, &Hint)> = Vec::new();
    if grid.columns == 2 {
        for (column, group) in [&groups.0, &groups.1].into_iter().enumerate() {
            for (row, hint) in group.iter().enumerate() {
                if row < grid.rows {
                    rows.push((column, row, hint));
                }
            }
        }
    } else {
        for (row, hint) in groups.0.iter().chain(groups.1.iter()).enumerate() {
            if row >= grid.rows {
                break;
            }
            rows.push((0, row, hint));
        }
    }
    for (column, row, hint) in rows {
        let cell = layout::hint_cell(grid, column, row);
        let key_column = KEY_COLUMN.min(cell.width() * 0.55);
        // 2. 【截图起始】【键位键帽】按键文本放在浅色圆角块内，与说明文字区分开
        let key = controls::fit_label(painter, &hint.key, 10.0, key_column - 12.0);
        let key_width = (painter.measure(&key, 10.0) + 10.0).min(key_column - 4.0);
        let cap = rect(cell.left(), cell.top() + 2.0, key_width, 16.0);
        painter.fill_rounded(cap, 4.0, palette.alternate);
        painter.text(cap, &key, TextStyle::new(10.0, palette.text).center());
        let title = controls::fit_label(painter, hint.title, 11.0, cell.width() - key_column - 4.0);
        painter.text(
            rect(
                cell.left() + key_column,
                cell.top(),
                cell.width() - key_column,
                cell.height(),
            ),
            &title,
            TextStyle::new(11.0, palette.muted),
        );
    }
}
