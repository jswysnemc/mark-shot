//! 【RTF解析】【颜色资源】解析颜色分量与颜色索引

use crate::theme::Rgba;

/// 【RTF解析】【颜色分量】标识颜色表中正在设置的分量
#[derive(Clone, Copy)]
pub(super) enum Component {
    Red,
    Green,
    Blue,
}

/// 【RTF解析】【颜色条目】累计一条颜色的红绿蓝分量并记录是否真正出现分量
#[derive(Default)]
struct ColorEntry {
    red: u8,
    green: u8,
    blue: u8,
    seen: bool,
}

/// 【RTF解析】【颜色表】按分号边界积累 colortbl 条目，索引零表示自动色
#[derive(Default)]
pub(super) struct ColorTable {
    entries: Vec<Option<Rgba>>,
    current: ColorEntry,
}

impl ColorTable {
    /// 【RTF解析】【新建颜色表】无参数，返回空颜色表
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 【RTF解析】【设置分量】按 component 与 value 写入当前颜色，越界值裁剪到 0..=255
    pub(super) fn set(&mut self, component: Component, value: i32) {
        let value = value.clamp(0, 255) as u8;
        self.current.seen = true;
        match component {
            Component::Red => self.current.red = value,
            Component::Green => self.current.green = value,
            Component::Blue => self.current.blue = value,
        }
    }

    /// 【RTF解析】【结束条目】无参数，把当前累计颜色写入表；无分量的条目表示为自动色
    pub(super) fn semicolon(&mut self) {
        let entry = std::mem::take(&mut self.current);
        self.entries.push(
            entry
                .seen
                .then_some(Rgba(entry.red, entry.green, entry.blue, 255)),
        );
    }

    /// 【RTF解析】【收尾】无参数，若仍有未结束的累计颜色则补写一个条目
    pub(super) fn finish(&mut self) {
        if self.current.seen {
            self.semicolon();
        }
    }

    /// 【RTF解析】【查询颜色】按 index 返回颜色；索引零、负值与缺失项返回空值
    pub(super) fn get(&self, index: i32) -> Option<Rgba> {
        if index <= 0 {
            return None;
        }
        self.entries.get(index as usize).copied().flatten()
    }
}
