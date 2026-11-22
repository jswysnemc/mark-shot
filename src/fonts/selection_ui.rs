//! 【选区尺寸】【界面字体】按原版 Sans Serif 定向加载常规与半粗体，不扫描系统字体目录

use cosmic_text::FontSystem;

/// 【选区尺寸】【字体创建】返回原版无衬线界面家族的独立字库，无参数，仅在首次显示尺寸面板时调用
pub(crate) fn new() -> FontSystem {
    super::ui_system::new()
}

/// 【选区尺寸】【缺字准备】按 text 在 fonts 检查非 ASCII 字符，仅加载实际缺失的半粗体回退，无返回值
pub(crate) fn prepare(fonts: &mut FontSystem, text: &str) {
    super::ui_fallback::ensure_family(fonts, text, "Sans Serif", true);
}
