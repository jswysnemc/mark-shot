//! 【工具栏设置】【页面状态】只保存页面交互状态，业务配置通过统一设置事务写入

pub(super) mod draw;
pub(super) mod input;
pub(super) mod layout;
#[cfg(test)]
mod tests;

use crate::{
    config::toolbar_layout::{Profile, ToolbarLayout},
    renderer::Icons,
};

#[derive(Default)]
pub(super) struct Editor {
    pub profile: Profile,
    pub config: ToolbarLayout,
    pub selected: Option<usize>,
    pub drag: Option<Drag>,
    pub pressed: Option<layout::Hit>,
    pub recording: bool,
    pub scroll: usize,
    pub scrollbar_drag: Option<crate::ui::scrollbar::Drag>,
    pub icons: Icons,
}

pub(super) struct Drag {
    pub source: usize,
    pub start: (f64, f64),
    pub target: usize,
    pub active: bool,
}

impl Editor {
    /// 【工具栏设置】【页面构造】按 values 载入有效布局，图标延迟到首次打开页面解析，返回状态
    pub fn new(values: &serde_json::Value) -> Self {
        Self {
            config: serde_json::from_value(values["toolbar_layout"].clone()).unwrap_or_default(),
            ..Default::default()
        }
    }

    /// 【工具栏设置】【图标准备】无参数，仅首次打开页面解析实际工具栏 SVG，不创建其他资源
    pub fn prepare(&mut self) {
        if self.icons.is_empty() {
            for entry in crate::ui::toolbar::catalog::ENTRIES {
                let svg = crate::ui::toolbar::icons::icon(entry.button);
                if let Ok(tree) = usvg::Tree::from_str(svg, &usvg::Options::default()) {
                    self.icons.insert(svg, tree);
                }
            }
        }
    }

    /// 【工具栏设置】【取消手势】无参数，清除拖动和录入，不改变已保存按钮顺序
    pub fn cancel(&mut self) {
        self.drag = None;
        self.pressed = None;
        self.recording = false;
        self.scrollbar_drag = None;
    }
}
