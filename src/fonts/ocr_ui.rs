//! 【文字识别】【界面字库】定向加载原版界面家族及当前正文缺字，不枚举全部系统字形

use cosmic_text::FontSystem;
#[cfg(test)]
use cosmic_text::Weight;
#[cfg(test)]
use cosmic_text::fontdb::{Family, Query, Stretch, Style};

/// 【文字识别】【界面字体】返回原版 Noto Sans 家族的独立字库；仅加载常规、半粗体文件，不改变其他窗口
pub(crate) fn new() -> FontSystem {
    let mut fonts = super::bundled();
    let family = super::text_faces::family(&mut fonts, "Noto Sans", false, false);
    super::text_faces::family(&mut fonts, &family, true, false);
    fonts.db_mut().set_sans_serif_family(family);
    fonts
}

/// 【文字识别】【缺字补充】按 text 在 fonts 中检查当前字形，仅为实际缺字加载定向回退，无返回值
pub(crate) fn ensure_text(fonts: &mut FontSystem, text: &str) {
    super::ui_fallback::ensure_family(fonts, text, "Noto Sans", false);
}

/// 【文字识别】【主字体检查】从 fonts 返回当前默认家族的名字，供本地验证字体隔离，无其他参数
#[cfg(test)]
fn family(fonts: &FontSystem) -> &str {
    let id = fonts
        .db()
        .query(&Query {
            families: &[Family::SansSerif],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        })
        .unwrap();
    &fonts.db().face(id).unwrap().families[0].0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【界面字体测试】【局部加载】验证 OCR 家族单独生效，重复 ASCII 和中文输入不枚举整个系统，无参数、无返回值
    #[test]
    fn ocr_font_loading_is_targeted_and_does_not_change_other_windows() {
        let other = super::super::bundled();
        let mut fonts = new();
        assert_eq!(family(&other), "Inter");
        let initial = fonts.db().len();
        ensure_text(&mut fonts, "Text / Copy");
        assert_eq!(fonts.db().len(), initial);
        ensure_text(&mut fonts, "文字识别 / 日本語");
        let after = fonts.db().len();
        ensure_text(&mut fonts, "文字识别 / 日本語");
        assert_eq!(fonts.db().len(), after);
        assert!(after <= initial + 32);
        assert_eq!(family(&other), "Inter");
    }
}
