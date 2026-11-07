pub(super) use crate::theme::window::Palette;

#[cfg(test)]
mod tests {
    use super::*;

    /// 【结果主题测试】【真实记录】逐项比较原版完整窗口的暗色调色板，不依赖当前用户主题，无参数、无返回值
    #[test]
    fn dark_palette_matches_the_actual_qt_window() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/qt-ocr-titlebar.json"))
                .unwrap();
        let colors = &fixture["feedback"]["states"][0]["state"]["colors"];
        let palette = Palette::new(false);
        for (key, rgba) in [
            ("window", palette.window),
            ("text", palette.text),
            ("alternate", palette.alternate),
            ("accent", palette.accent),
            ("on_accent", palette.on_accent),
        ] {
            assert_eq!(
                serde_json::json!([rgba.0, rgba.1, rgba.2, rgba.3]),
                colors[key]
            );
        }
    }
}
