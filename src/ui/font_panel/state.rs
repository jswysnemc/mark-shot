//! 【字体弹层】【输入状态】维护搜索、可见行、滚动和控件焦点，不直接修改标注

use super::layout::{FontLayout, ROW_HEIGHT};
use crate::{
    fonts::font_catalog::FontCatalog,
    ui::{
        panel::{HoverablePanel, UiPanel},
        settings_panel::SettingsWidget,
        text_field::{CursorInit, TextFieldGroup},
    },
};
use tiny_skia::{Pixmap, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Search,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Search,
    Clear,
    Family(usize),
    Size,
    SizeUp,
    SizeDown,
    Bold,
    Italic,
    Scrollbar,
    ScrollUp,
    ScrollDown,
}

pub struct FontPanel {
    pub open: bool,
    pub dirty: bool,
    pub render_pos: (f32, f32),
    pub monitor_idx: usize,
    pub layout: FontLayout,
    pub catalog: Option<FontCatalog>,
    pub filtered: Vec<usize>,
    pub offset: f32,
    pub family: String,
    pub points: f32,
    pub bold: bool,
    pub italic: bool,
    pub fields: TextFieldGroup<Field>,
    pub hovered: Option<Hit>,
    pub pressed: Option<Hit>,
    pub scrollbar_drag: Option<f32>,
    pub pixmap: Option<Pixmap>,
    pub preview: super::preview::Preview,
}

impl Default for FontPanel {
    /// 【字体弹层】【初始状态】返回没有系统目录和预览字库的面板，首次打开前不扫描字体
    fn default() -> Self {
        Self {
            open: false,
            dirty: true,
            render_pos: (0.0, 0.0),
            monitor_idx: 0,
            layout: FontLayout::new(10, (640.0, 480.0)),
            catalog: None,
            filtered: Vec::new(),
            offset: 0.0,
            family: String::new(),
            points: 20.0,
            bold: true,
            italic: false,
            fields: TextFieldGroup::new(),
            hovered: None,
            pressed: None,
            scrollbar_drag: None,
            pixmap: None,
            preview: Default::default(),
        }
    }
}

impl FontPanel {
    /// 【字体弹层】【打开准备】清空搜索并居中 family 对应行，无返回值，已经枚举的目录继续复用
    pub fn begin(&mut self) {
        if self.catalog.is_none() {
            self.catalog = Some(FontCatalog::load());
        }
        self.open = true;
        self.fields
            .begin_edit(Field::Search, String::new(), CursorInit::End);
        self.fields.values.insert(Field::Search, String::new());
        self.filter();
        self.center_current();
        self.hovered = None;
        self.pressed = None;
        self.dirty = true;
    }

    /// 【字体弹层】【查询读取】返回当前搜索字段内容，不返回字号输入文字
    pub fn query(&self) -> &str {
        self.fields
            .editing
            .as_ref()
            .filter(|edit| edit.key == Field::Search)
            .map(|edit| edit.field.text.as_str())
            .or_else(|| self.fields.values.get(&Field::Search).map(String::as_str))
            .unwrap_or("")
    }

    /// 【字体弹层】【目录过滤】按当前搜索字段过滤目录，返回值为空，过滤不改变面板总高度
    pub fn filter(&mut self) {
        let query = self.query().trim().to_lowercase();
        self.filtered = self
            .catalog
            .as_ref()
            .map(|catalog| {
                catalog
                    .families
                    .iter()
                    .enumerate()
                    .filter(|(_, family)| {
                        query.is_empty() || family.name.to_lowercase().contains(&query)
                    })
                    .map(|(index, _)| index)
                    .collect()
            })
            .unwrap_or_default();
        self.offset = self.offset.min(self.max_offset());
        self.dirty = true;
    }

    /// 【字体弹层】【当前项居中】将当前 family 居中到列表，边界限制到有效滚动范围，无返回值
    pub fn center_current(&mut self) {
        if let Some(catalog) = &self.catalog
            && let Some(row) = self
                .filtered
                .iter()
                .position(|index| catalog.families[*index].name == self.family)
        {
            let preceding = (self.layout.list.height() / ROW_HEIGHT / 2.0).floor() as usize;
            self.offset =
                (row.saturating_sub(preceding) as f32 * ROW_HEIGHT).clamp(0.0, self.max_offset());
        }
    }

    /// 【字体弹层】【滚动边界】返回当前过滤列表允许的最大像素偏移
    pub fn max_offset(&self) -> f32 {
        let visible = (self.layout.list.height() / ROW_HEIGHT).floor().max(1.0) as usize;
        self.filtered.len().saturating_sub(visible) as f32 * ROW_HEIGHT
    }

    /// 【字体弹层】【滚轮移动】按 delta 像素更新滚动偏移，返回是否改变，不保留跨控件滚轮积累
    pub fn scroll(&mut self, delta: f32) -> bool {
        let old = self.offset;
        self.offset = (self.offset + delta).clamp(0.0, self.max_offset());
        self.dirty |= old != self.offset;
        old != self.offset
    }

    /// 【字体弹层】【可见字体】返回当前可见字体目录索引与本地行矩形，只包含完整或部分可见行
    pub fn rows(&self) -> Vec<(usize, Rect)> {
        let first = (self.offset / ROW_HEIGHT).floor().max(0.0) as usize;
        self.filtered
            .iter()
            .enumerate()
            .skip(first)
            .take((self.layout.list.height() / ROW_HEIGHT).ceil() as usize + 1)
            .map(|(row, index)| {
                (
                    *index,
                    self.layout.row(row, self.offset, self.max_offset() > 0.0),
                )
            })
            .filter(|(_, row)| row.top() < self.layout.list.bottom())
            .collect()
    }

    /// 【字体弹层】【滚动手柄】返回本地纵向滚动条手柄，无需滚动时返回空值
    pub fn thumb(&self) -> Option<Rect> {
        let max = self.max_offset();
        if max <= 0.0 {
            return None;
        }
        let list = self.layout.list;
        let track = list.height() - 28.0;
        let height = (track * list.height() / (self.filtered.len() as f32 * ROW_HEIGHT))
            .max(26.0)
            .min(track);
        Rect::from_xywh(
            list.right() - 14.0,
            list.top() + 14.0 + ((track - height) * self.offset / max).floor(),
            14.0,
            height,
        )
    }

    /// 【字体弹层】【区域命中】返回 local 显示器坐标是否在可见面板内，关闭状态返回假
    pub fn hit_test(&self, local: (f64, f64)) -> bool {
        self.rect()
            .is_some_and(|rect| contains(rect, (local.0 as f32, local.1 as f32)))
    }

    /// 【字体弹层】【控件命中】将 local 映射为控件或字体目录索引，留白和空列表返回空值
    pub fn hit(&self, local: (f64, f64)) -> Option<Hit> {
        if !self.hit_test(local) {
            return None;
        }
        let point = (
            local.0 as f32 - self.render_pos.0,
            local.1 as f32 - self.render_pos.1,
        );
        let layout = self.layout;
        if contains(layout.search, point) {
            return Some(
                if !self.query().is_empty() && point.0 >= layout.search.right() - 25.0 {
                    Hit::Clear
                } else {
                    Hit::Search
                },
            );
        }
        if contains(layout.bold, point) {
            return Some(Hit::Bold);
        }
        if contains(layout.italic, point) {
            return Some(Hit::Italic);
        }
        if contains(layout.size_field, point) {
            return Some(if point.0 >= layout.size_field.right() - 16.0 {
                if point.1 < layout.size_field.top() + 13.0 {
                    Hit::SizeUp
                } else {
                    Hit::SizeDown
                }
            } else {
                Hit::Size
            });
        }
        if contains(layout.list, point) {
            if self.max_offset() > 0.0 && point.0 >= layout.list.right() - 14.0 {
                if point.1 < layout.list.top() + 14.0 {
                    return Some(Hit::ScrollUp);
                }
                if point.1 >= layout.list.bottom() - 14.0 {
                    return Some(Hit::ScrollDown);
                }
                return Some(Hit::Scrollbar);
            }
            let row = ((point.1 - layout.list.top() + self.offset) / ROW_HEIGHT).floor() as usize;
            return self.filtered.get(row).copied().map(Hit::Family);
        }
        None
    }
}

/// 【字体弹层】【矩形判断】返回 point 是否在 rect 内，右边与下边使用排他边界
fn contains(rect: Rect, point: (f32, f32)) -> bool {
    point.0 >= rect.left()
        && point.0 < rect.right()
        && point.1 >= rect.top()
        && point.1 < rect.bottom()
}

impl UiPanel for FontPanel {
    type Item = SettingsWidget;
    /// 【字体弹层】【绘制位置】返回当前面板的显示器本地坐标
    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    /// 【字体弹层】【面板尺寸】返回搜索、列表和样式行的总尺寸
    fn size(&self) -> (f32, f32) {
        self.layout.size
    }
    /// 【字体弹层】【控件集合】返回空通用控件集合，字体控件使用独立布局
    fn items(&self) -> &[SettingsWidget] {
        &[]
    }
    /// 【字体弹层】【面板留白】返回原版六像素留白
    fn padding(&self) -> f32 {
        6.0
    }
    /// 【字体弹层】【显示器索引】返回当前显示器索引
    fn monitor_idx(&self) -> usize {
        self.monitor_idx
    }
    /// 【字体弹层】【标记重绘】将当前面板标记为需要绘制，无返回值
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
    /// 【字体弹层】【重绘状态】返回是否需要绘制
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    /// 【字体弹层】【可见状态】返回面板是否打开
    fn is_visible(&self) -> bool {
        self.open
    }
    /// 【字体弹层】【可见范围】返回显示器本地矩形，关闭时返回空值
    fn rect(&self) -> Option<Rect> {
        self.open
            .then(|| {
                Rect::from_xywh(
                    self.render_pos.0,
                    self.render_pos.1,
                    self.layout.size.0,
                    self.layout.size.1,
                )
            })
            .flatten()
    }
}

impl HoverablePanel for FontPanel {
    type Hover = Option<Hit>;
    /// 【字体弹层】【悬停读取】返回当前悬停控件
    fn hovered(&self) -> Self::Hover {
        self.hovered
    }
    /// 【字体弹层】【悬停写入】以 hover 替换当前悬停控件，无返回值
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover;
    }
}
