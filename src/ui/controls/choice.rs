//! 【界面控件】【短选项菜单】小型选择列表拥有独立输入归属，取消不提交选项，支持鼠标和键盘

use crate::{
    backend::window::{LEFT_BUTTON, WindowEvent},
    keys::Key,
    renderer::{Painter, text::TextStyle},
    ui::{
        icons,
        menu_style::{self, ITEM_RADIUS, MenuStyle, POPUP_RADIUS},
    },
};
use cosmic_text::Weight;
use tiny_skia::Rect;

const ROW: f32 = 30.0;
const PAD: f32 = 6.0;
const ROW_INSET_Y: f32 = 2.0;

#[derive(Debug, PartialEq, Eq)]
pub enum ChoiceEvent {
    Keep,
    Redraw,
    Closed,
    Selected(usize),
}

pub struct Choice {
    anchor: Rect,
    labels: Vec<String>,
    selected: usize,
    focused: Option<usize>,
    hovered: Option<usize>,
    keyboard_active: bool,
    pressed: Option<usize>,
}

impl Choice {
    /// 【界面控件】【短菜单创建】按 anchor、labels 与 selected 创建当前值和键盘候选，返回拥有选项文字的菜单
    pub fn new(anchor: Rect, labels: Vec<String>, selected: usize) -> Self {
        let selected = selected.min(labels.len().saturating_sub(1));
        Self {
            anchor,
            labels,
            selected,
            focused: Some(selected),
            hovered: None,
            keyboard_active: false,
            pressed: None,
        }
    }

    /// 【界面控件】【菜单范围】按 size 计算窗口内的菜单位置，下面空间不足时放在选择框上方，返回范围
    pub(crate) fn bounds(&self, size: (u32, u32)) -> Rect {
        let width = self.anchor.width().min(size.0 as f32 - 16.0).max(1.0);
        let height = self.labels.len() as f32 * ROW + PAD * 2.0;
        let top = if self.anchor.bottom() + 4.0 + height <= size.1 as f32 - 2.0 {
            self.anchor.bottom() + 4.0
        } else {
            (self.anchor.top() - height - 4.0).max(8.0)
        };
        Rect::from_xywh(
            self.anchor.left().min(size.0 as f32 - width - 8.0).max(8.0),
            top,
            width,
            height,
        )
        .unwrap()
    }

    /// 【界面控件】【选项命中】按 point 与 size 返回菜单内的行号，边距与外部位置不属于选项
    fn hit(&self, point: (f64, f64), size: (u32, u32)) -> Option<usize> {
        let area = self.bounds(size);
        let (x, y) = (point.0 as f32, point.1 as f32);
        if x < area.left() + PAD
            || x >= area.right() - PAD
            || y < area.top() + PAD
            || y >= area.bottom() - PAD
        {
            return None;
        }
        Some(((y - area.top() - PAD) / ROW) as usize).filter(|index| *index < self.labels.len())
    }

    /// 【界面控件】【菜单绘制】用 painter 按 size 绘制带四像素垂直空隙的已选与悬停胶囊，无返回值
    pub fn draw(&self, painter: &mut Painter<'_>, size: (u32, u32)) {
        let area = self.bounds(size);
        let style = MenuStyle::current();
        menu_style::draw_popup_frame(painter, area, POPUP_RADIUS);
        for (index, label) in self.labels.iter().enumerate() {
            let row = Rect::from_xywh(
                area.left() + PAD,
                area.top() + PAD + index as f32 * ROW,
                area.width() - PAD * 2.0,
                ROW,
            )
            .unwrap();
            let selected = self.selected == index;
            let active_hover = self.hovered == Some(index)
                || (self.keyboard_active && self.focused == Some(index));
            let ink = menu_style::draw_row_pill(
                painter,
                row,
                ROW_INSET_Y,
                ITEM_RADIUS,
                selected,
                active_hover,
                false,
            );
            if selected {
                painter.icon(
                    icons::CHECK,
                    12.0,
                    row.left() + 8.0,
                    row.top() + (ROW - 12.0) * 0.5,
                    style.accent,
                );
            }
            let label = super::fit_label(painter, label, 12.0, row.width() - 34.0);
            painter.text(
                Rect::from_xywh(row.left() + 26.0, row.top(), row.width() - 30.0, ROW).unwrap(),
                &label,
                TextStyle::new(12.0, ink).weight(if selected {
                    Weight::MEDIUM
                } else {
                    Weight::NORMAL
                }),
            );
        }
    }

    /// 【界面控件】【菜单输入】按 event 与 size 返回重绘、取消或选项，移出菜单清除悬停但保留键盘候选
    pub fn event(&mut self, event: &WindowEvent, size: (u32, u32)) -> ChoiceEvent {
        if self.labels.is_empty() {
            return ChoiceEvent::Closed;
        }
        match event {
            WindowEvent::PointerMove { position } => {
                let next_hover = self.hit(*position, size);
                if let Some(hit) = next_hover {
                    if self.hovered == Some(hit)
                        && self.focused == Some(hit)
                        && !self.keyboard_active
                    {
                        return ChoiceEvent::Keep;
                    }
                    self.hovered = Some(hit);
                    self.focused = Some(hit);
                    self.keyboard_active = false;
                } else if self.hovered.take().is_none() {
                    return ChoiceEvent::Keep;
                }
            }
            WindowEvent::PointerLeave => {
                if self.hovered.take().is_none() {
                    return ChoiceEvent::Keep;
                }
            }
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed,
            } => {
                let hit = self.hit(*position, size);
                if *pressed {
                    self.pressed = hit;
                    if hit.is_none() {
                        return ChoiceEvent::Closed;
                    }
                } else if let Some(index) = self.pressed.take().filter(|index| Some(*index) == hit)
                {
                    return ChoiceEvent::Selected(index);
                }
            }
            WindowEvent::Key(input) => match input.key {
                Some(Key::Escape | Key::Tab) => return ChoiceEvent::Closed,
                Some(Key::Enter | Key::Space) => {
                    return ChoiceEvent::Selected(self.focused.unwrap_or(self.selected));
                }
                Some(Key::Up | Key::Down | Key::Home | Key::End) => {
                    let current = self.focused.unwrap_or(self.selected);
                    self.keyboard_active = true;
                    self.hovered = None;
                    self.focused = Some(match input.key {
                        Some(Key::Up) => current.saturating_sub(1),
                        Some(Key::Down) => (current + 1).min(self.labels.len().saturating_sub(1)),
                        Some(Key::Home) => 0,
                        _ => self.labels.len().saturating_sub(1),
                    });
                }
                _ => return ChoiceEvent::Keep,
            },
            WindowEvent::Focused(false)
            | WindowEvent::Resize { .. }
            | WindowEvent::Scroll { .. } => return ChoiceEvent::Closed,
            _ => return ChoiceEvent::Keep,
        }
        ChoiceEvent::Redraw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::window::KeyInput;

    /// 【选择控件测试】【键盘提交】键盘可直接选择最后一个真实选项，关闭操作不提交新值
    #[test]
    fn keyboard_selects_an_option_and_escape_cancels() {
        let mut choice = Choice::new(
            Rect::from_xywh(92.0, 76.0, 292.0, 26.0).unwrap(),
            vec!["MP4".into(), "MKV".into(), "GIF".into()],
            0,
        );
        let event = |key| {
            WindowEvent::Key(KeyInput {
                key: Some(key),
                text: String::new(),
                mods: Default::default(),
            })
        };
        assert_eq!(
            choice.event(&event(Key::End), (400, 300)),
            ChoiceEvent::Redraw
        );
        assert_eq!(
            choice.event(&event(Key::Enter), (400, 300)),
            ChoiceEvent::Selected(2)
        );
        assert_eq!(
            choice.event(&event(Key::Escape), (400, 300)),
            ChoiceEvent::Closed
        );
    }

    /// 【选择控件测试】【外部点击】按下底部开始按钮只取消菜单，不提交选项，跨行松开也不执行旧行
    #[test]
    fn outside_press_and_cross_row_release_do_not_commit() {
        let mut choice = Choice::new(
            Rect::from_xywh(92.0, 76.0, 292.0, 26.0).unwrap(),
            vec!["MP4".into(), "GIF".into()],
            0,
        );
        let button = |position, pressed| WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        };
        assert_eq!(
            choice.event(&button((320.0, 278.0), true), (400, 300)),
            ChoiceEvent::Closed
        );
        assert_ne!(
            choice.event(&button((120.0, 126.0), false), (400, 300)),
            ChoiceEvent::Selected(0)
        );
        choice.event(&button((120.0, 126.0), true), (400, 300));
        assert_ne!(
            choice.event(&button((120.0, 154.0), false), (400, 300)),
            ChoiceEvent::Selected(0)
        );
    }

    /// 【选择控件测试】【候选保留】键盘选中视频后，选择框上的指针移动不能把候选恢复成原来的 GIF
    #[test]
    fn keyboard_candidate_survives_pointer_motion_over_the_field() {
        let mut choice = Choice::new(
            Rect::from_xywh(92.0, 76.0, 292.0, 26.0).unwrap(),
            vec!["MP4".into(), "MKV".into(), "GIF".into()],
            2,
        );
        let key = |key| {
            WindowEvent::Key(KeyInput {
                key: Some(key),
                text: String::new(),
                mods: Default::default(),
            })
        };
        choice.event(&key(Key::Home), (400, 300));
        choice.event(
            &WindowEvent::PointerMove {
                position: (136.0, 92.0),
            },
            (400, 300),
        );
        assert_eq!(
            choice.event(&key(Key::Enter), (400, 300)),
            ChoiceEvent::Selected(0)
        );
    }

    /// 【选择控件测试】【悬停与间隙】相邻胶囊保持四像素垂直空隙，指针移入移出独立更新悬停态且与已选项配色分离
    #[test]
    fn row_pills_keep_vertical_gap_and_hover_clears_on_leave() {
        let mut choice = Choice::new(
            Rect::from_xywh(92.0, 76.0, 292.0, 26.0).unwrap(),
            vec!["关闭".into(), "3 秒".into(), "5 秒".into()],
            0,
        );
        let area = choice.bounds((400, 300));
        let row0 = Rect::from_xywh(
            area.left() + PAD,
            area.top() + PAD,
            area.width() - PAD * 2.0,
            ROW,
        )
        .unwrap();
        let row1 = Rect::from_xywh(
            area.left() + PAD,
            area.top() + PAD + ROW,
            area.width() - PAD * 2.0,
            ROW,
        )
        .unwrap();
        let pill0 = menu_style::pill_rect(row0, ROW_INSET_Y);
        let pill1 = menu_style::pill_rect(row1, ROW_INSET_Y);
        assert!((pill1.top() - pill0.bottom() - 4.0).abs() < f32::EPSILON);

        let style = MenuStyle::current();
        assert_ne!(style.selected_fill, style.hover_fill);

        assert_eq!(choice.hovered, None);
        assert_eq!(
            choice.event(
                &WindowEvent::PointerMove {
                    position: (120.0, 154.0),
                },
                (400, 300),
            ),
            ChoiceEvent::Redraw
        );
        assert_eq!(choice.hovered, Some(1));
        assert_eq!(
            choice.event(
                &WindowEvent::PointerMove {
                    position: (20.0, 20.0),
                },
                (400, 300),
            ),
            ChoiceEvent::Redraw
        );
        assert_eq!(choice.hovered, None);
    }
}
