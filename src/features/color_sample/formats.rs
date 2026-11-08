//! 【取色面板】【颜色格式】与 Qt 共用格式语义，所有文本都从原图非预乘颜色生成

use tiny_skia::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Hex,
    LowerHex,
    RgbaHex,
    Rgb,
    Rgba,
    Hsl,
    Hsv,
    Qt,
}

impl Format {
    pub const ALL: [Self; 8] = [
        Self::Hex,
        Self::LowerHex,
        Self::RgbaHex,
        Self::Rgb,
        Self::Rgba,
        Self::Hsl,
        Self::Hsv,
        Self::Qt,
    ];

    /// 【取色面板】【格式名称】无参数，返回 Qt 对应行标题
    pub fn label(self) -> &'static str {
        match self {
            Self::Hex => "HEX",
            Self::LowerHex => "hex lower",
            Self::RgbaHex => "RGBA hex",
            Self::Rgb => "RGB",
            Self::Rgba => "RGBA",
            Self::Hsl => "HSL",
            Self::Hsv => "HSV",
            Self::Qt => "Qt",
        }
    }

    /// 【取色面板】【格式转换】将 color 转为可直接复制使用的字符串，保留透明度，灰度色相归零
    pub fn text(self, color: Color) -> String {
        let pixel = color.to_color_u8();
        let (red, green, blue, alpha) = (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha());
        let hex = format!("#{red:02X}{green:02X}{blue:02X}");
        match self {
            Self::Hex => hex,
            Self::LowerHex => hex.to_lowercase(),
            Self::RgbaHex => format!("{hex}{alpha:02X}"),
            Self::Rgb => format!("rgb({red}, {green}, {blue})"),
            Self::Rgba => format!("rgba({red}, {green}, {blue}, {})", channel(alpha)),
            Self::Qt => format!(
                "Qt.rgba({}, {}, {}, {})",
                channel(red),
                channel(green),
                channel(blue),
                channel(alpha)
            ),
            Self::Hsl | Self::Hsv => {
                let (red, green, blue) = (color.red(), color.green(), color.blue());
                let maximum = red.max(green).max(blue);
                let minimum = red.min(green).min(blue);
                let delta = maximum - minimum;
                let hue = if delta == 0.0 {
                    0.0
                } else if maximum == red {
                    ((green - blue) / delta).rem_euclid(6.0)
                } else if maximum == green {
                    (blue - red) / delta + 2.0
                } else {
                    (red - green) / delta + 4.0
                } * 60.0;
                let lightness = (maximum + minimum) / 2.0;
                let saturation = if delta == 0.0 {
                    0.0
                } else if self == Self::Hsl {
                    delta / (1.0 - (2.0 * lightness - 1.0).abs())
                } else {
                    delta / maximum
                };
                let level = if self == Self::Hsl {
                    lightness
                } else {
                    maximum
                };
                format!(
                    "{}({:.0}, {:.0}%, {:.0}%)",
                    self.label().to_lowercase(),
                    hue,
                    saturation * 100.0,
                    level * 100.0
                )
            }
        }
    }
}

/// 【取色面板】【归一通道】按 byte 返回三位精度的零到一通道，去除尾随零和小数点
fn channel(byte: u8) -> String {
    format!("{:.3}", f32::from(byte) / 255.0)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .into()
}

#[cfg(test)]
mod tests {
    /// 【取色回归】【Qt 格式】无参数，检查示例色八种格式和透明度、灰度边界
    #[test]
    fn matches_qt_inspector_formats() {
        use super::*;
        let color = Color::from_rgba8(32, 67, 92, 255);
        assert_eq!(
            Format::ALL.map(|format| format.text(color)),
            [
                "#20435C",
                "#20435c",
                "#20435CFF",
                "rgb(32, 67, 92)",
                "rgba(32, 67, 92, 1)",
                "hsl(205, 48%, 24%)",
                "hsv(205, 65%, 36%)",
                "Qt.rgba(0.125, 0.263, 0.361, 1)",
            ]
        );
        assert_eq!(
            Format::Rgba.text(Color::from_rgba8(0, 0, 0, 128)),
            "rgba(0, 0, 0, 0.502)"
        );
        assert_eq!(Format::Hsl.text(Color::WHITE), "hsl(0, 0%, 100%)");
        assert_eq!(Format::Hsv.text(Color::BLACK), "hsv(0, 0%, 0%)");
    }
}
