//! 【界面字体】【系统家族】按需加载 Fontconfig 首选界面家族，内置字体用于配置缺失时回退

use cosmic_text::FontSystem;

/// 【界面字体】【字库创建】无参数，返回定向加载常规、半粗体和粗体的界面字库，不枚举全部系统字形
pub(crate) fn new() -> FontSystem {
    with_family("Sans Serif")
}

/// 【界面字体】【指定家族】按 family 返回定向加载常规、半粗体和粗体的字库，保留组件明确指定的字体设计
pub(crate) fn with_family(family: &str) -> FontSystem {
    let mut fonts = super::bundled();
    let resolved = super::text_faces::family(&mut fonts, family, false, false);
    super::text_faces::family(&mut fonts, family, true, false);
    super::text_faces::family_with_weight(&mut fonts, family, cosmic_text::Weight::BOLD, false);
    fonts.db_mut().set_sans_serif_family(resolved);
    fonts
}
