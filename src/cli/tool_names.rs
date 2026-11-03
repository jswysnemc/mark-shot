use crate::tools::Tool;

/// 【命令行】【工具名称】将原版工具名称或别名转换为原生工具；未知名称返回错误
pub(super) fn parse(value: &str) -> Result<Tool, String> {
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
