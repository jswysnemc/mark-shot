//! 【文字识别】【滚动条绘制】原版横向颜色条带与纵向悬停滑块，不借用共享工具栏主题

use super::{
    palette::Palette,
    table_scroll_input::Interaction,
    table_scroll_layout::{Axis, Bar, Part},
};
use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::{PathBuilder, Rect};

const DARK_NORMAL: [[u8; 4]; 14] = [
    [18, 25, 39, 255],
    [77, 87, 104, 255],
    [53, 64, 83, 255],
    [52, 64, 82, 255],
    [52, 63, 81, 255],
    [51, 62, 81, 255],
    [51, 62, 80, 255],
    [50, 61, 79, 255],
    [50, 61, 78, 255],
    [49, 60, 78, 255],
    [49, 59, 77, 255],
    [48, 59, 76, 255],
    [48, 58, 75, 255],
    [58, 67, 81, 255],
];

const DARK_PRESSED: [[u8; 4]; 14] = [
    [17, 24, 38, 255],
    [73, 83, 98, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [59, 68, 82, 255],
];

const DARK_GROOVE: [[u8; 4]; 14] = [
    [14, 21, 37, 255],
    [21, 33, 60, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [23, 36, 65, 255],
    [22, 33, 61, 255],
];

const DARK_BUTTON: [[u8; 4]; 14] = [
    [12, 18, 32, 255],
    [63, 72, 94, 255],
    [50, 60, 78, 255],
    [49, 60, 77, 255],
    [49, 60, 77, 255],
    [49, 59, 77, 255],
    [48, 59, 76, 255],
    [48, 59, 76, 255],
    [48, 58, 75, 255],
    [48, 58, 75, 255],
    [47, 58, 74, 255],
    [47, 57, 74, 255],
    [47, 57, 73, 255],
    [61, 71, 91, 255],
];

const LIGHT_NORMAL: [[u8; 4]; 14] = [
    [191, 192, 193, 255],
    [251, 251, 251, 255],
    [251, 251, 251, 255],
    [251, 251, 251, 255],
    [252, 252, 252, 255],
    [252, 252, 252, 255],
    [252, 252, 252, 255],
    [253, 253, 253, 255],
    [253, 253, 253, 255],
    [253, 253, 253, 255],
    [254, 254, 254, 255],
    [254, 254, 254, 255],
    [255, 255, 255, 255],
    [229, 229, 229, 255],
];

const LIGHT_PRESSED: [[u8; 4]; 14] = [
    [191, 192, 194, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [229, 229, 229, 255],
];

const LIGHT_GROOVE: [[u8; 4]; 14] = [
    [196, 198, 199, 255],
    [232, 232, 232, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [243, 243, 243, 255],
    [242, 242, 242, 255],
    [229, 229, 229, 255],
];

const LIGHT_BUTTON: [[u8; 4]; 14] = [
    [183, 184, 186, 255],
    [249, 249, 249, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [255, 255, 255, 255],
    [247, 247, 247, 255],
];

/// 【文字识别】【颜色条带】按 painter、rect、profile 绘制原版十四像素横截面，窗口比例由画布统一缩放，无返回值
fn bands(painter: &mut Painter<'_>, rect: Rect, profile: &[[u8; 4]; 14]) {
    for (index, value) in profile.iter().enumerate() {
        painter.fill_rounded(
            Rect::from_xywh(rect.x(), rect.y() + index as f32, rect.width(), 1.0).unwrap(),
            0.0,
            Rgba(value[0], value[1], value[2], value[3]),
        );
    }
}

/// 【文字识别】【滚动条绘制】按 painter、bars、interaction、colors 绘制可见方向，纵向只有滑块悬停或拖动时出现，无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    bars: &[Option<Bar>; 2],
    interaction: &Interaction,
    colors: Palette,
) {
    let light = colors.base.0 > 128;
    for bar in bars.iter().flatten() {
        if bar.axis == Axis::Vertical {
            if interaction.hovered == Some((Axis::Vertical, Part::Slider))
                || interaction.pressed(Axis::Vertical, Part::Slider)
            {
                painter.fill_rounded(bar.slider, 3.0, colors.border);
            }
            continue;
        }
        bands(
            painter,
            bar.groove,
            if light { &LIGHT_GROOVE } else { &DARK_GROOVE },
        );
        bands(
            painter,
            bar.slider,
            match (light, interaction.pressed(Axis::Horizontal, Part::Slider)) {
                (true, false) => &LIGHT_NORMAL,
                (true, true) => &LIGHT_PRESSED,
                (false, false) => &DARK_NORMAL,
                (false, true) => &DARK_PRESSED,
            },
        );
        for (rect, forward) in [(bar.previous_line, false), (bar.next_line, true)] {
            let Some(rect) = rect else {
                continue;
            };
            bands(
                painter,
                rect,
                if light { &LIGHT_BUTTON } else { &DARK_BUTTON },
            );
            let center = (
                rect.x() + rect.width() / 2.0,
                rect.y() + rect.height() / 2.0,
            );
            let direction = if forward { 1.0 } else { -1.0 };
            let mut path = PathBuilder::new();
            path.move_to(center.0 + direction * 2.0, center.1);
            path.line_to(center.0 - direction * 2.0, center.1 - 3.0);
            path.line_to(center.0 - direction * 2.0, center.1 + 3.0);
            path.close();
            if let Some(path) = path.finish() {
                painter.fill_path(&path, colors.muted);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【滚动条绘制测试】【原版条带】横向滑块常态和按下条带逐项匹配原版实际 PNG，避免凭视觉猜测颜色，无参数、无返回值
    #[test]
    fn horizontal_profiles_match_the_actual_qt_bar_pixels() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/qt-ocr-table-scroll-paint.json"
        ))
        .unwrap();
        for (theme, profiles) in [
            (
                "dark",
                [&DARK_NORMAL, &DARK_PRESSED, &DARK_GROOVE, &DARK_BUTTON],
            ),
            (
                "light",
                [&LIGHT_NORMAL, &LIGHT_PRESSED, &LIGHT_GROOVE, &LIGHT_BUTTON],
            ),
        ] {
            for (name, profile) in ["normal", "pressed", "groove", "button"]
                .into_iter()
                .zip(profiles)
            {
                assert_eq!(serde_json::json!(profile), reference[theme][name]);
            }
        }
    }
}
