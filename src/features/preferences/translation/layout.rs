//! 【翻译设置】【表单布局】服务卡片与分组参数共用绘制、滚动和命中坐标

use super::super::{
    layout::viewport,
    window::{Preferences, contains},
};
use super::fields::Field;
use crate::ui::scrollbar;
use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::features::preferences) enum Hit {
    Enable,
    Auto,
    Field(Field),
    Apply,
    Reveal,
    Query,
    Models,
}

pub(super) struct Row {
    pub field: Field,
    pub label: Rect,
    pub help: Rect,
    pub bounds: Rect,
}

pub(in crate::features::preferences) struct Layout {
    pub(super) viewport: Rect,
    pub(super) rows: Vec<Row>,
    pub(super) panels: Vec<(Rect, &'static str)>,
    pub(in crate::features::preferences) controls: Vec<(Hit, Rect)>,
    pub(super) scrollbar: Option<scrollbar::Geometry>,
    pub(super) capacity: usize,
    pub(super) maximum: usize,
}

pub(super) use super::super::layout::rect;

impl Layout {
    /// 【翻译设置】【自适应布局】按 state 返回卡片、参数分组和可见菜单；表单按像素滚动，目录按条目滚动
    pub(in crate::features::preferences) fn new(state: &Preferences) -> Self {
        let viewport = viewport(state);
        let mut layout = Self {
            viewport,
            rows: Vec::new(),
            panels: Vec::new(),
            controls: Vec::new(),
            scrollbar: None,
            capacity: viewport.height() as usize,
            maximum: 0,
        };
        let total = layout.form(state);
        layout.maximum = total.saturating_sub(layout.capacity);
        layout.scrollbar =
            scrollbar::geometry(viewport, total, layout.capacity, state.translation.scroll);
        layout
    }

    /// 【翻译设置】【表单分组】按 state 在当前布局添加服务卡片和字段，返回完整表单高度
    fn form(&mut self, state: &Preferences) -> usize {
        let chinese = state.chinese();
        let left = self.viewport.left();
        let width = self.viewport.width() - 14.0;
        let origin = self.viewport.top() - state.translation.scroll as f32;
        let mut top = origin;
        let provider_height = 124.0;
        self.panels.push((
            rect(left, top, width, provider_height),
            if chinese {
                "查看与启用服务"
            } else {
                "Configure and enable service"
            },
        ));
        self.controls.push((
            Hit::Field(Field::Provider),
            rect(left + 14.0, top + 40.0, width - 28.0, 32.0),
        ));
        self.controls.push((
            Hit::Enable,
            rect(left + 14.0, top + 80.0, (width - 36.0) / 2.0, 30.0),
        ));
        self.controls.push((
            Hit::Auto,
            rect(
                left + 22.0 + (width - 36.0) / 2.0,
                top + 80.0,
                (width - 36.0) / 2.0,
                30.0,
            ),
        ));
        top += provider_height + 16.0;
        let mut fields = state.translation.fields();
        fields.retain(|field| *field != Field::Provider);
        fields.sort_by_key(|field| field.group(chinese).0);
        let mut group = None;
        for field in fields {
            let (number, title) = field.group(chinese);
            if group != Some(number) {
                if group.is_some() {
                    top += 16.0;
                }
                self.panels.push((rect(left, top, width, 34.0), title));
                top += 34.0;
                group = Some(number);
            }
            let toggle = matches!(field, Field::Filter | Field::Disable);
            let stacked = width < 520.0 && !toggle;
            let height = if stacked { 84.0 } else { 60.0 };
            let control_width = if toggle {
                38.0
            } else if stacked {
                width - 28.0
            } else {
                (width * 0.58).min(430.0)
            };
            let bounds = rect(
                left + width - control_width - 14.0,
                top + if stacked { 44.0 } else { 14.0 },
                control_width,
                32.0,
            );
            let label_width = if stacked {
                width - 28.0
            } else {
                width - control_width - 42.0
            };
            self.rows.push(Row {
                field,
                label: rect(left + 14.0, top + 10.0, label_width, 22.0),
                help: rect(left + 14.0, top + 33.0, label_width, 17.0),
                bounds,
            });
            self.field(state, field, bounds);
            top += height;
            if let Some((panel, _)) = self.panels.last_mut() {
                *panel = rect(panel.left(), panel.top(), width, top - panel.top());
            }
        }
        (top - origin).ceil() as usize
    }

    /// 【翻译设置】【字段控件】按 state、field 和 bounds 添加真实编辑动作，返回空值
    fn field(&mut self, state: &Preferences, field: Field, bounds: Rect) {
        if field == Field::Models {
            let width = (bounds.width() - 8.0) / 2.0;
            self.controls
                .push((Hit::Query, rect(bounds.left(), bounds.top(), width, 32.0)));
            self.controls.push((
                Hit::Models,
                rect(bounds.left() + width + 8.0, bounds.top(), width, 32.0),
            ));
        } else if state
            .translation
            .editing
            .as_ref()
            .is_some_and(|(active, _)| *active == field)
        {
            self.controls.push((
                Hit::Apply,
                rect(bounds.right() - 48.0, bounds.top(), 48.0, 32.0),
            ));
            if field.secret() {
                self.controls.push((
                    Hit::Reveal,
                    rect(bounds.right() - 102.0, bounds.top(), 48.0, 32.0),
                ));
            }
            self.controls.push((
                Hit::Field(field),
                rect(
                    bounds.left(),
                    bounds.top(),
                    bounds.width() - if field.secret() { 108.0 } else { 54.0 },
                    32.0,
                ),
            ));
        } else {
            self.controls.push((Hit::Field(field), bounds));
        }
    }

    /// 【翻译设置】【控件命中】按 point 返回可见控件，裁切外和滚动条不响应字段动作
    pub(in crate::features::preferences) fn hit(&self, point: (f64, f64)) -> Option<Hit> {
        if !contains(self.viewport, point)
            || self
                .scrollbar
                .as_ref()
                .is_some_and(|bar| contains(bar.track, point))
        {
            return None;
        }
        self.controls
            .iter()
            .find(|(_, bounds)| contains(*bounds, point))
            .map(|(hit, _)| *hit)
    }

    /// 【翻译设置】【焦点定位】按 field 返回保持字段完整可见所需的相对滚动距离
    pub(super) fn reveal(&self, field: Field) -> isize {
        let Some(row) = self.rows.iter().find(|row| row.field == field) else {
            return 0;
        };
        if row.label.top() < self.viewport.top() {
            (row.label.top() - self.viewport.top() - 10.0).floor() as isize
        } else {
            (row.bounds.bottom() + 10.0 - self.viewport.bottom())
                .max(0.0)
                .ceil() as isize
        }
    }
}
