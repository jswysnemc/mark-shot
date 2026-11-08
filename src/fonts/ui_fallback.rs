//! 【界面字体】【按需缺字】仅加载当前菜单或提示实际缺失的字体，不扫描整个系统目录

use cosmic_text::{Font, FontSystem, Weight, fontdb};
use std::{collections::HashSet, sync::Arc};

#[cfg(test)]
#[path = "ui_fallback_tests.rs"]
mod tests;

/// 【界面字体】【文字准备】按fonts与text补充缺失字形，无返回值；内置西文字体和序号字体保持独立
pub fn ensure_text(fonts: &mut FontSystem, text: &str) {
    ensure_family(fonts, text, "Sans Serif", false);
}

/// 【界面字体】【定向缺字】按 fonts、text、family 和 bold 仅解析匹配字形的字体，无返回值，不把系统目录视为已加载字形
pub(super) fn ensure_family(fonts: &mut FontSystem, text: &str, family: &str, bold: bool) {
    if text.is_ascii() {
        return;
    }
    let weight = if bold { Weight(600) } else { Weight::NORMAL };
    let mut faces: Vec<Arc<Font>> = Vec::new();
    let mut checked = HashSet::new();
    // 1. 【界面字体】【覆盖检查】只检查本次匹配的字体，系统目录中的其他家族不参与解析和常驻缓存
    for character in text.chars().filter(|ch| !ch.is_ascii() && !ch.is_control()) {
        if !checked.insert(character)
            || faces
                .iter()
                .any(|font| font.as_swash().charmap().map(character) != 0)
        {
            continue;
        }
        // 2. 【界面字体】【实际加载】缺字匹配先确定家族，再解析所需字重的单个字体，复用 FontSystem 已有缓存
        let resolved = super::text_faces::fallback(fonts, family, bold, false, character);
        let selected = fonts.db().query(&fontdb::Query {
            families: &[fontdb::Family::Name(&resolved)],
            weight,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        });
        if let Some(font) = selected.and_then(|id| fonts.get_font(id, weight)) {
            faces.push(font);
        }
    }
}
