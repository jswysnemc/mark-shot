//! 【界面字体】【系统家族】按需加载 Fontconfig 首选界面家族，内置字体用于配置缺失时回退

use cosmic_text::FontSystem;

/// 【界面字体】【字库创建】无参数，返回定向加载常规与半粗体的界面字库，不枚举全部系统字形
pub(crate) fn new() -> FontSystem {
    let mut fonts = super::bundled();
    let family = super::text_faces::family(&mut fonts, "Sans Serif", false, false);
    super::text_faces::family(&mut fonts, "Sans Serif", true, false);
    fonts.db_mut().set_sans_serif_family(family);
    fonts
}
