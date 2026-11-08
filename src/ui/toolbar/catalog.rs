//! 【工具栏配置】【按钮目录】预览与实际工具栏共用稳定身份、中文名称和动作映射

use super::{Integration, ToolbarButton as Button, ToolbarItem};
use crate::{tools::Tool, types::Finish};

pub struct Entry {
    pub id: &'static str,
    pub button: Button,
    pub labels: (&'static str, &'static str),
}

impl Entry {
    /// 【工具栏配置】【显示名称】按 chinese 返回按钮名称，不使用图标字符代替文字
    pub fn label(&self, chinese: bool) -> &'static str {
        if chinese {
            self.labels.0
        } else {
            self.labels.1
        }
    }

    /// 【工具栏配置】【快捷键身份】无参数，返回此按钮在共享快捷键映射中的配置键
    pub fn shortcut(&self) -> Option<&'static str> {
        let action = self.button.action()?;
        crate::keys::ACTIONS
            .iter()
            .find(|(_, candidate, _)| *candidate == action)
            .map(|(key, _, _)| *key)
    }
}

macro_rules! entry {
    ($id:literal, $button:expr, $cn:literal, $en:literal) => {
        Entry {
            id: $id,
            button: $button,
            labels: ($cn, $en),
        }
    };
}

pub const ENTRIES: &[Entry] = &[
    entry!(
        "selection",
        Button::Tool(Tool::Selection),
        "调整选区",
        "Selection"
    ),
    entry!(
        "pick",
        Button::Tool(Tool::Pick),
        "选择标注",
        "Select annotation"
    ),
    entry!(
        "rectangle",
        Button::Tool(Tool::Rectangle),
        "矩形",
        "Rectangle"
    ),
    entry!("circle", Button::Tool(Tool::Circle), "椭圆", "Ellipse"),
    entry!("line", Button::Tool(Tool::Line), "直线", "Line"),
    entry!("pen", Button::Tool(Tool::Pen), "画笔", "Pen"),
    entry!("arrow", Button::Tool(Tool::Arrow), "箭头", "Arrow"),
    entry!("number", Button::Tool(Tool::Number), "序号", "Number"),
    entry!(
        "numerated_arrow",
        Button::Tool(Tool::NumeratedArrow),
        "编号箭头",
        "Numbered arrow"
    ),
    entry!("marker", Button::Tool(Tool::Marker), "形状标记", "Marker"),
    entry!(
        "highlighter",
        Button::Tool(Tool::Highlighter),
        "荧光笔",
        "Highlighter"
    ),
    entry!("redact", Button::Tool(Tool::Redact), "马赛克", "Redact"),
    entry!("text", Button::Tool(Tool::Text), "文字", "Text"),
    entry!(
        "magnifier",
        Button::Tool(Tool::Magnifier),
        "放大镜",
        "Magnifier"
    ),
    entry!("ruler", Button::Tool(Tool::Ruler), "尺子", "Ruler"),
    entry!("laser", Button::Tool(Tool::Laser), "激光笔", "Laser"),
    entry!(
        "eyedropper",
        Button::Tool(Tool::Eyedropper),
        "取色标注",
        "Color sample"
    ),
    entry!("clear", Button::Clear, "清除全部标注", "Clear annotations"),
    entry!("undo", Button::Undo, "撤销", "Undo"),
    entry!("redo", Button::Redo, "重做", "Redo"),
    entry!("scope", Button::Scope, "切换全图选区", "Toggle full image"),
    entry!(
        "layout",
        Button::Layout,
        "切换横纵排列",
        "Toggle orientation"
    ),
    entry!(
        "open_with",
        Button::Integration(Integration::OpenWith),
        "打开方式",
        "Open with"
    ),
    entry!(
        "extensions",
        Button::Integration(Integration::Extensions),
        "扩展操作",
        "Extensions"
    ),
    entry!("scroll", Button::Scroll, "滚动截图", "Scrolling capture"),
    entry!("pin", Button::Finish(Finish::Pin), "贴图", "Pin"),
    entry!("ocr", Button::Tool(Tool::Ocr), "文字识别", "OCR"),
    entry!(
        "translate",
        Button::Integration(Integration::Translate),
        "翻译",
        "Translate"
    ),
    entry!("copy", Button::Finish(Finish::Copy), "复制截图", "Copy"),
    entry!("save", Button::Finish(Finish::Save), "保存截图", "Save"),
    entry!("save_as", Button::SaveAs, "另存为", "Save as"),
    entry!(
        "upload",
        Button::Integration(Integration::Upload),
        "上传图片",
        "Upload"
    ),
    entry!(
        "settings",
        Button::Integration(Integration::Settings),
        "设置",
        "Settings"
    ),
    entry!("cancel", Button::Cancel, "关闭截图", "Close capture"),
];

/// 【工具栏配置】【项目编码】按 item 返回稳定配置值，分隔线允许重复出现
pub fn id(item: ToolbarItem) -> &'static str {
    match item {
        ToolbarItem::Seperator => "separator",
        ToolbarItem::Button(button) => {
            ENTRIES
                .iter()
                .find(|entry| entry.button == button)
                .expect("toolbar button is registered")
                .id
        }
    }
}

/// 【工具栏配置】【项目解析】按 id 返回已实现按钮或分隔线，未知身份返回空值
pub fn parse(id: &str) -> Option<ToolbarItem> {
    if id == "separator" {
        return Some(ToolbarItem::Seperator);
    }
    ENTRIES
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| ToolbarItem::Button(entry.button))
}

/// 【工具栏配置】【目录查询】按 item 返回按钮元数据，分隔线没有快捷键
pub fn entry(item: ToolbarItem) -> Option<&'static Entry> {
    ENTRIES
        .iter()
        .find(|entry| ToolbarItem::Button(entry.button) == item)
}
