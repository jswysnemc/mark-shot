//! 【标注多选】【标题字形】数量变化才重排原版界面字体，不创建独立字库或空闲定时器

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight};

pub(crate) struct Title {
    pub text: String,
    pub buffer: Buffer,
    pub lines: Vec<cosmic_text::LayoutLine>,
    pub width: f32,
    pub height: f32,
    pub cell_width: f32,
}

impl Title {
    /// 【标注多选】【标题构造】按text与fonts排版十一像素粗体数量标题，返回字形缓冲与原生标签宽度
    pub(crate) fn new(text: String, fonts: &mut FontSystem) -> Self {
        let family = crate::fonts::text_faces::family(fonts, "Noto Sans", false, false);
        crate::fonts::text_faces::family_with_weight(fonts, &family, Weight::BOLD, false);
        for character in text.chars().filter(|character| !character.is_ascii()) {
            let fallback =
                crate::fonts::text_faces::fallback(fonts, &family, true, false, character);
            crate::fonts::text_faces::family_with_weight(fonts, &fallback, Weight::BOLD, false);
        }
        let mut buffer = Buffer::new_empty(Metrics::new(11.0, 15.0));
        buffer.set_size(None, None);
        // 1. 【标注多选】【字距单位】原版零点二像素字距截断为六十四分之十二像素，排版接口再转换为字号比例
        buffer.set_text(
            &text,
            &Attrs::new()
                .family(Family::Name(&family))
                .weight(Weight::BOLD)
                .letter_spacing(12.0 / 64.0 / 11.0),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(fonts, false);
        let lines =
            crate::fonts::fixed_layout::lines(fonts, &buffer, 11.0, None, cosmic_text::Wrap::None);
        let width = lines.iter().map(|line| line.w).fold(0.0, f32::max);
        let height = buffer.layout_runs().map(|run| run.line_height).sum();
        Self {
            text,
            buffer,
            lines,
            width,
            height,
            cell_width: (width + 18.0).ceil().max(58.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Rect;

    /// 【标注多选界面测试】【原生尺寸】数量标题与三个通用控件逐项匹配真实Qt样式布局，明暗主题不能改变几何
    #[test]
    fn group_title_and_controls_match_native_qt_sizes() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qt-annotation-collection/panel.json"
        )))
        .unwrap();
        let mut fonts = crate::fonts::bundled();
        let rect = |actual: Rect, expected: &serde_json::Value| {
            for (actual, expected) in [actual.x(), actual.y(), actual.width(), actual.height()]
                .iter()
                .zip(expected.as_array().unwrap())
            {
                assert_eq!(f64::from(*actual), expected.as_f64().unwrap());
            }
        };
        for sample in reference["samples"].as_array().unwrap() {
            let count = sample["count"].as_u64().unwrap();
            let text = sample["text"]
                .as_str()
                .map_or_else(|| format!("Group {count}"), str::to_owned);
            let title = Title::new(text, &mut fonts);
            let layout = crate::ui::annotation_collection::layout::new(title.cell_width);
            rect(
                crate::ui::annotation_collection::layout::title_rect(title.cell_width),
                &sample["title"],
            );
            assert_eq!(
                f64::from(layout.size.0),
                sample["size"][0].as_f64().unwrap()
            );
            assert_eq!(
                f64::from(layout.size.1),
                sample["size"][1].as_f64().unwrap()
            );
            for index in 0..2 {
                rect(layout.glyphs[index], &sample["glyphs"][index]);
                rect(layout.values[index], &sample["values"][index]);
            }
            for index in 0..3 {
                rect(layout.controls[index].unwrap(), &sample["controls"][index]);
            }
        }
    }
}
