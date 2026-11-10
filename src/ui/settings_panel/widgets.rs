//! 【属性面板】【控件组合】定义值、动作和各工具的属性控件

use crate::tools::Tool;
use crate::types::annotations::{Annotation, AnnotationShape};
use crate::ui::color_popover::ColorTarget;
use crate::ui::panel::PanelItem;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueField {
    Hex,
    Rgb,
    Hsl,
}

impl ValueField {
    /// 【属性面板】【颜色文字】将 color 转为当前字段格式；返回十六进制或 RGB 文字
    pub fn text(self, color: tiny_skia::Color) -> String {
        let c = color.to_color_u8();
        match self {
            ValueField::Hex => format!("#{}", crate::ui::color_popover::color_to_hex_string(color)),
            ValueField::Rgb => format!("{}, {}, {}", c.red(), c.green(), c.blue()),
            ValueField::Hsl => {
                let red = color.red();
                let green = color.green();
                let blue = color.blue();
                let maximum = red.max(green).max(blue);
                let minimum = red.min(green).min(blue);
                let delta = maximum - minimum;
                let lightness = (maximum + minimum) / 2.0;
                let saturation = if delta == 0.0 {
                    0.0
                } else {
                    delta / (1.0 - (2.0 * lightness - 1.0).abs())
                };
                let hue = if delta == 0.0 {
                    0.0
                } else if maximum == red {
                    ((green - blue) / delta).rem_euclid(6.0)
                } else if maximum == green {
                    (blue - red) / delta + 2.0
                } else {
                    (red - green) / delta + 4.0
                } * 60.0;
                format!(
                    "hsl({:.0}, {:.0}%, {:.0}%)",
                    hue,
                    saturation * 100.0,
                    lightness * 100.0
                )
            }
        }
    }

    /// 【属性面板】【字段尺寸】返回当前文字值控件的逻辑宽度
    fn width(self) -> f32 {
        let cfg = &crate::config::get().settings_panel;
        match self {
            ValueField::Hex => cfg.hex_width,
            ValueField::Rgb | ValueField::Hsl => cfg.rgb_width.max(165.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ToggleVisual {
    Icon { svg: &'static str },
    Checkbox { label: &'static str },
}

/// 【属性面板】【动作类型】只触发操作，不保存开关或数值状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    OcrRescan,
    OcrCopyAll,
    OcrOpenResult,
    OcrTranslate,
    OcrLanguages,
    MarkerShapes,
    RulerModes,
    NumberStyles,
    ResetNumber,
    ArrowStyles,
    LineStyles,
    FilterModes,
    ColorFormats,
}

impl SettingsAction {
    /// 【属性面板】【动作图标】返回当前动作使用的逻辑图标尺寸
    pub fn icon_size(self) -> f32 {
        let cfg = &crate::config::get().settings_panel;
        match self {
            // The copy glyph reads bigger than the rest at the same size.
            SettingsAction::OcrCopyAll => cfg.copy_icon_size,
            SettingsAction::MarkerShapes => crate::config::get()
                .toolbar
                .icons
                .marker
                .min(cfg.icon_button_size - 4.0)
                .max(1.0),
            _ => cfg.icon_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepperRange {
    Stroke,
    Font,
    Strength,
    Magnification,
    NumberWidth,
    LineWidth,
    CornerRadius,
}

impl StepperRange {
    /// 【属性面板】【数值范围】返回当前数值类型的最小值、最大值与步长
    pub fn get(self) -> (f32, f32, f32) {
        let tools = &crate::config::get().tools;
        match self {
            StepperRange::CornerRadius => (0.0, 256.0, 1.0),
            StepperRange::Stroke => (tools.stroke.min, tools.stroke.max, tools.stroke.step),
            StepperRange::Font => (tools.font.min, tools.font.max, tools.font.step),
            StepperRange::Strength => {
                let (min, max) = crate::types::tool_settings::STRENGTH;
                (min, max, tools.redact.step)
            }
            StepperRange::Magnification => (1.25, 6.0, 0.25),
            StepperRange::NumberWidth => {
                let (min, max) = crate::features::number::WIDTH;
                (min, max, 1.0)
            }
            StepperRange::LineWidth => {
                let (min, max) = crate::features::line::WIDTH;
                (min, max, 1.0)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingsWidget {
    TextControl(crate::ui::text_properties::layout::Control),
    ColorSwatch(ColorTarget),
    Action {
        action: SettingsAction,
        svg: &'static str,
    },
    Stepper {
        label: &'static str,
        range: StepperRange,
        unit: &'static str,
    },
    Toggle {
        visual: ToggleVisual,
        field: ToggleField,
    },
    Label(&'static str),
    Value {
        field: ValueField,
    },
    Download,
    Separator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepperArrow {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowHoldState {
    pub widget_idx: usize,
    pub arrow: StepperArrow,
    pub started_at: Instant,
    pub last_step_at: Instant,
    pub repeat_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSource {
    Tool(Tool),
    Annotation(u64),
    Collection { revision: u64, count: usize },
    OcrScanning,
    OcrAwaiting { downloading: bool },
    OcrNoModel { downloading: bool },
    OcrResult { downloading: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleField {
    Bold,
    Italic,
    Outline,
    Fill,
    MarkerFill,
    MagnifierRectangle,
    FixedRatio,
}

impl ToggleField {
    /// 【属性面板】【样式图标】返回当前开关使用的逻辑图标尺寸
    pub fn icon_size(self) -> f32 {
        crate::config::get().settings_panel.icon_size
    }
}

/// 【属性面板】【工具控件】依据 tool 返回颜色、尺寸和样式等属性控件
pub fn widgets_for_tool(tool: Tool) -> &'static [SettingsWidget] {
    match tool {
        Tool::Number => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Action {
                action: SettingsAction::NumberStyles,
                svg: crate::ui::icons::NUMBER,
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::NumberWidth,
                unit: "px",
            },
            SettingsWidget::Action {
                action: SettingsAction::ResetNumber,
                svg: crate::ui::icons::RESET_NUMBER,
            },
        ],
        Tool::Magnifier => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Magnification,
                unit: "x",
            },
            SettingsWidget::Toggle {
                visual: ToggleVisual::Icon {
                    svg: crate::ui::icons::RECTANGLE,
                },
                field: ToggleField::MagnifierRectangle,
            },
            CORNER_RADIUS,
        ],
        Tool::Ruler => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Action {
                action: SettingsAction::RulerModes,
                svg: crate::ui::icons::RULER,
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
        ],
        Tool::Marker => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Action {
                action: SettingsAction::MarkerShapes,
                svg: crate::ui::icons::MARKER,
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
            SettingsWidget::Separator,
            SettingsWidget::Toggle {
                visual: ToggleVisual::Checkbox { label: "Fill" },
                field: ToggleField::MarkerFill,
            },
            CORNER_RADIUS,
        ],
        Tool::Arrow => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Action {
                action: SettingsAction::ArrowStyles,
                svg: crate::ui::icons::ARROW,
            },
            SettingsWidget::Action {
                action: SettingsAction::LineStyles,
                svg: crate::ui::icons::LINE,
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::LineWidth,
                unit: "px",
            },
        ],
        Tool::Line => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Action {
                action: SettingsAction::LineStyles,
                svg: crate::ui::icons::LINE,
            },
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::LineWidth,
                unit: "px",
            },
        ],
        Tool::Pen | Tool::Highlighter | Tool::Laser | Tool::NumeratedArrow => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
        ],
        Tool::Text => TEXT_WIDGETS,
        Tool::Rectangle => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
            SettingsWidget::Toggle {
                visual: ToggleVisual::Checkbox { label: "Fill" },
                field: ToggleField::Fill,
            },
            FIXED_RATIO,
            CORNER_RADIUS,
        ],
        Tool::Circle => &[
            FIXED_RATIO,
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Stroke,
                unit: "px",
            },
            SettingsWidget::Separator,
            SettingsWidget::Toggle {
                visual: ToggleVisual::Checkbox { label: "Fill" },
                field: ToggleField::Fill,
            },
        ],
        Tool::Redact => &[
            SettingsWidget::ColorSwatch(ColorTarget::Main),
            SettingsWidget::Separator,
            SettingsWidget::Stepper {
                label: "",
                range: StepperRange::Strength,
                unit: "",
            },
            SettingsWidget::Separator,
            SettingsWidget::Action {
                action: SettingsAction::FilterModes,
                svg: crate::ui::icons::REDACT,
            },
            CORNER_RADIUS,
        ],
        Tool::Eyedropper => EYEDROPPER_WIDGETS,
        Tool::Ocr => OCR_WIDGETS,
        _ => &[],
    }
}

const CORNER_RADIUS: SettingsWidget = SettingsWidget::Stepper {
    label: "R",
    range: StepperRange::CornerRadius,
    unit: "px",
};
const FIXED_RATIO: SettingsWidget = SettingsWidget::Toggle {
    visual: ToggleVisual::Checkbox { label: "1:1" },
    field: ToggleField::FixedRatio,
};

pub const TEXT_WIDGETS: &[SettingsWidget] = &[
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Size),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Opacity),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Color),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Background),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Font),
];
pub const TEXT_EDIT_WIDGETS: &[SettingsWidget] = &[
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Size),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Opacity),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Color),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Background),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Font),
    SettingsWidget::TextControl(crate::ui::text_properties::layout::Control::Edit),
];

impl SettingsWidget {
    /// 【属性面板】【颜色目标】返回普通或文字控件对应的取色目标，非颜色按钮返回空值
    pub fn color_target(&self) -> Option<ColorTarget> {
        use crate::ui::text_properties::layout::Control;
        match self {
            Self::ColorSwatch(target) => Some(*target),
            Self::TextControl(Control::Color) => Some(ColorTarget::Main),
            Self::TextControl(Control::Background) => Some(ColorTarget::TextBackground),
            _ => None,
        }
    }
}

pub const EYEDROPPER_WIDGETS: &[SettingsWidget] = &[
    SettingsWidget::Value {
        field: ValueField::Hex,
    },
    SettingsWidget::Action {
        action: SettingsAction::ColorFormats,
        svg: crate::ui::icons::CHEVRON_DOWN,
    },
];

/// What the panel offers once text has been read. The hint is the answer to
/// "how do I read something else": dragging on empty space inside the tool
/// boxes out a new region - on this screen or any other - and reads it on
/// release, so nothing has to be re-picked from the toolbar.
pub const OCR_WIDGETS: &[SettingsWidget] = &[
    OCR_LANGUAGES,
    SettingsWidget::Separator,
    OCR_RESCAN,
    OCR_COPY_ALL,
    OCR_OPEN_RESULT,
    OCR_TRANSLATE,
    SettingsWidget::Separator,
    SettingsWidget::Label("Drag a new box to read another area"),
];

/// when downloading
pub const OCR_WIDGETS_DOWNLOADING: &[SettingsWidget] = &[
    OCR_LANGUAGES,
    SettingsWidget::Separator,
    OCR_RESCAN,
    OCR_COPY_ALL,
    OCR_OPEN_RESULT,
    OCR_TRANSLATE,
    SettingsWidget::Separator,
    SettingsWidget::Download,
];

/// Shown while the worker thread is busy; the progress badge sits over the
/// region itself, this just keeps the panel from advertising dead buttons.
pub const OCR_SCANNING_WIDGETS: &[SettingsWidget] = &[SettingsWidget::Label("Reading text...")];

pub const OCR_AWAITING_WIDGETS: &[SettingsWidget] = &[OCR_LANGUAGES];

pub const OCR_NO_MODEL_WIDGETS: &[SettingsWidget] = &[
    OCR_LANGUAGES,
    SettingsWidget::Separator,
    SettingsWidget::Label("Choose an OCR engine and resources"),
];

pub const OCR_DOWNLOADING_WIDGETS: &[SettingsWidget] = &[
    OCR_LANGUAGES,
    SettingsWidget::Separator,
    SettingsWidget::Download,
];

const OCR_LANGUAGES: SettingsWidget = SettingsWidget::Action {
    action: SettingsAction::OcrLanguages,
    svg: crate::ui::icons::GLOBE,
};

const OCR_RESCAN: SettingsWidget = SettingsWidget::Action {
    action: SettingsAction::OcrRescan,
    svg: crate::ui::icons::RETRY,
};

const OCR_COPY_ALL: SettingsWidget = SettingsWidget::Action {
    action: SettingsAction::OcrCopyAll,
    svg: crate::ui::icons::COPY,
};

const OCR_OPEN_RESULT: SettingsWidget = SettingsWidget::Action {
    action: SettingsAction::OcrOpenResult,
    svg: crate::ui::icons::TEXT,
};

const OCR_TRANSLATE: SettingsWidget = SettingsWidget::Action {
    action: SettingsAction::OcrTranslate,
    svg: crate::ui::toolbar::icons::TRANSLATE,
};

/// 【属性面板】【标注控件】依据 ann 的形状返回可编辑属性控件
pub fn widgets_for_annotation(ann: &Annotation) -> &'static [SettingsWidget] {
    match &ann.shape {
        AnnotationShape::ColorSample { .. } => widgets_for_tool(Tool::Eyedropper),
        AnnotationShape::Number { .. } => widgets_for_tool(Tool::Number),
        AnnotationShape::Magnifier { .. } => widgets_for_tool(Tool::Magnifier),
        AnnotationShape::Ruler { .. } => widgets_for_tool(Tool::Ruler),
        AnnotationShape::Marker { .. } => widgets_for_tool(Tool::Marker),
        AnnotationShape::Line { .. } => widgets_for_tool(Tool::Line),
        AnnotationShape::Arrow { .. } => widgets_for_tool(Tool::Arrow),
        AnnotationShape::Text { .. } => widgets_for_tool(Tool::Text),
        AnnotationShape::Redact { .. } => widgets_for_tool(Tool::Redact),
        AnnotationShape::Rectangle { .. } => widgets_for_tool(Tool::Rectangle),
        AnnotationShape::Circle { .. } => widgets_for_tool(Tool::Circle),
        AnnotationShape::Pen { .. }
        | AnnotationShape::Highlighter { .. }
        | AnnotationShape::NumeratedArrow { .. } => widgets_for_tool(Tool::Pen),
    }
}

impl PanelItem for SettingsWidget {
    /// 【属性面板】【控件尺寸】返回当前控件的逻辑宽度
    fn size(&self) -> f32 {
        let cfg = &crate::config::get().settings_panel;
        let width = match self {
            SettingsWidget::TextControl(_) => cfg.icon_button_size,
            SettingsWidget::Action {
                action: SettingsAction::NumberStyles,
                ..
            } => cfg.icon_button_size + cfg.char_width * 3.0,
            SettingsWidget::Action {
                action: SettingsAction::RulerModes,
                ..
            } => cfg.icon_button_size + cfg.char_width * 7.0,
            SettingsWidget::ColorSwatch(_) => cfg.swatch_size,
            SettingsWidget::Action {
                action: SettingsAction::MarkerShapes,
                ..
            } => cfg.icon_button_size + 8.0,
            SettingsWidget::Action {
                action:
                    SettingsAction::ArrowStyles
                    | SettingsAction::LineStyles
                    | SettingsAction::FilterModes,
                ..
            } => cfg.icon_button_size + 16.0,
            SettingsWidget::Action { .. } => cfg.icon_button_size,
            SettingsWidget::Stepper { .. } => cfg.stepper_width,
            SettingsWidget::Toggle { visual, .. } => match visual {
                ToggleVisual::Icon { .. } => cfg.icon_button_size,
                ToggleVisual::Checkbox { label } => {
                    cfg.checkbox_size + cfg.checkbox_gap + label.len() as f32 * cfg.char_width
                }
            },
            SettingsWidget::Label(text) => {
                text.chars().count() as f32 * cfg.char_width + cfg.label_padding
            }
            SettingsWidget::Value { field } => field.width(),
            SettingsWidget::Download => cfg.download_width,
            SettingsWidget::Separator => cfg.separator_slot,
        };
        width * crate::ui::toolbar::metrics::properties_scale()
    }

    /// 【属性面板】【控件间距】返回当前控件之后的逻辑留白
    fn trailing_padding(&self) -> f32 {
        match self {
            SettingsWidget::Separator => 0.0,
            _ => crate::config::get().settings_panel.item_gap,
        }
    }

    /// 【属性面板】【控件命中】返回当前控件是否接受按钮交互
    fn is_button(&self) -> bool {
        matches!(
            self,
            SettingsWidget::TextControl(_)
                | SettingsWidget::ColorSwatch(_)
                | SettingsWidget::Action { .. }
                | SettingsWidget::Stepper { .. }
                | SettingsWidget::Toggle { .. }
                | SettingsWidget::Value { .. }
        )
    }
}
