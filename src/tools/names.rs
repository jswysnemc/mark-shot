//! 【标注工具】【名称解析】命令行与配置使用相同的原版工具名称及别名

use super::Tool;

/// 【标注工具】【原版名称】将 value 的工具名称或别名转换为原生工具，返回未知名称错误
pub(crate) fn parse(value: &str) -> Result<Tool, String> {
    match value.to_ascii_lowercase().as_str() {
        "move" | "selection" => Ok(Tool::Selection),
        "select" | "pick" => Ok(Tool::Pick),
        "pen" => Ok(Tool::Pen),
        "highlighter" | "highlight" => Ok(Tool::Highlighter),
        "laser" => Ok(Tool::Laser),
        "marker" | "shape" => Ok(Tool::Marker),
        "ruler" | "measure" => Ok(Tool::Ruler),
        "magnifier" | "magnify" | "loupe" | "zoom" => Ok(Tool::Magnifier),
        "line" => Ok(Tool::Line),
        "arrow" => Ok(Tool::Arrow),
        "rectangle" | "rect" => Ok(Tool::Rectangle),
        "ellipse" | "circle" => Ok(Tool::Circle),
        "text" => Ok(Tool::Text),
        "number" | "serial" | "sequence" => Ok(Tool::Number),
        "numbered" | "numerated-arrow" => Ok(Tool::NumeratedArrow),
        "mosaic" | "redact" => Ok(Tool::Redact),
        "ocr" => Ok(Tool::Ocr),
        "color" | "eyedropper" => Ok(Tool::Eyedropper),
        _ => Err(format!("unknown annotation tool: {value}")),
    }
}
