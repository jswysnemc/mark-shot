use crate::tools::Tool;
use crate::types::Finish;

pub const SELECTION: &str = include_str!("../../assets/icons/mark-shot/tool-move.svg");
pub const ARROW: &str = include_str!("../../assets/icons/mark-shot/tool-arrow.svg");
pub const RECTANGLE: &str = include_str!("../../assets/icons/mark-shot/tool-rectangle.svg");
pub const CIRCLE: &str = include_str!("../../assets/icons/mark-shot/tool-ellipse.svg");
pub const TEXT: &str = include_str!("../../assets/icons/mark-shot/tool-text.svg");
pub const PEN: &str = include_str!("../../assets/icons/mark-shot/tool-pen.svg");
pub const HIGHLIGHTER: &str = include_str!("../../assets/icons/mark-shot/tool-highlighter.svg");
pub const LASER: &str = include_str!("../../assets/icons/mark-shot/tool-laser.svg");
pub const MARKER: &str = include_str!("../../assets/icons/mark-shot/tool-marker.svg");
pub const RULER: &str = include_str!("../../assets/icons/mark-shot/tool-ruler.svg");
pub const RULER_ANGLE: &str = include_str!("../../assets/icons/ruler-angle.svg");
pub const MAGNIFIER: &str = include_str!("../../assets/icons/mark-shot/tool-magnifier.svg");
pub const LINE: &str = include_str!("../../assets/icons/mark-shot/tool-line.svg");
pub const PICK: &str = include_str!("../../assets/icons/mark-shot/tool-select.svg");
pub const EYEDROPPER: &str = include_str!("../../assets/icons/eyedropper.svg");
pub const NUMERATED_ARROW: &str = include_str!("../../assets/icons/numerated_arrow.svg");
pub const NUMBER: &str = include_str!("../../assets/icons/mark-shot/tool-number.svg");
pub const RESET_NUMBER: &str = include_str!("../../assets/icons/reset-number.svg");
pub const REDACT: &str = include_str!("../../assets/icons/mark-shot/tool-mosaic.svg");
pub const BLURRED: &str = include_str!("../../assets/icons/blur.svg");
pub const PIXELATED: &str = include_str!("../../assets/icons/pixelate.svg");
pub const OCR: &str = include_str!("../../assets/icons/mark-shot/ocr.svg");
pub const ITALIC: &str = include_str!("../../assets/icons/italic.svg");
pub const BOLD: &str = include_str!("../../assets/icons/bold.svg");
pub const RETRY: &str = include_str!("../../assets/icons/retry.svg");
pub const COPY: &str = include_str!("../../assets/icons/mark-shot/copy.svg");
pub const GLOBE: &str = include_str!("../../assets/icons/globe.svg");
pub const DOWNLOAD: &str = include_str!("../../assets/icons/download.svg");
pub const CLOSE: &str = include_str!("../../assets/icons/mark-shot/cancel.svg");
pub const CHEVRON_DOWN: &str = include_str!("../../assets/icons/mark-shot/chevron-down.svg");
pub const TRASH: &str = include_str!("../../assets/icons/trash.svg");
pub const CHECK: &str = include_str!("../../assets/icons/check.svg");
pub const MARK_SHOT_CHECK: &str = include_str!("../../assets/icons/mark-shot/check.svg");
pub const PIN: &str = include_str!("../../assets/icons/mark-shot/pin.svg");
pub const SAVE: &str = include_str!("../../assets/icons/mark-shot/save.svg");
pub const SCROLL: &str = include_str!("../../assets/icons/mark-shot/scroll-capture.svg");
pub const SCROLL_HORIZONTAL: &str = include_str!("../../assets/icons/scroll_horizontal.svg");
pub const SCROLL_PAUSE: &str = include_str!("../../assets/icons/scroll_pause.svg");
pub const SCROLL_RESUME: &str = include_str!("../../assets/icons/scroll_resume.svg");

/// used in the toolbar
pub fn get_svg_for_tool(tool: Tool) -> (&'static str, f32) {
    let size = &crate::config::get().toolbar.icons;
    match tool {
        Tool::Selection => (SELECTION, size.selection),
        Tool::Pick => (PICK, size.pick),
        Tool::Eyedropper => (EYEDROPPER, size.eyedropper),
        Tool::Text => (TEXT, size.text),
        Tool::Pen => (PEN, size.pen),
        Tool::Highlighter => (HIGHLIGHTER, size.highlighter),
        Tool::Laser => (LASER, size.laser),
        Tool::Marker => (MARKER, size.marker),
        Tool::Ruler => (RULER, size.ruler),
        Tool::Magnifier => (MAGNIFIER, size.magnifier),
        Tool::Line => (LINE, size.line),
        Tool::Arrow => (ARROW, size.arrow),
        Tool::Rectangle => (RECTANGLE, size.rectangle),
        Tool::Circle => (CIRCLE, size.circle),
        Tool::NumeratedArrow => (NUMERATED_ARROW, size.numerated_arrow),
        Tool::Number => (NUMBER, size.number),
        Tool::Redact => (REDACT, size.redact),
        Tool::Ocr => (OCR, size.ocr),
    }
}

pub fn get_svg_for_finish(finish: Finish) -> (&'static str, f32) {
    let size = &crate::config::get().toolbar.icons;
    match finish {
        Finish::Pin => (PIN, size.pin),
        Finish::Copy => (COPY, size.copy),
        Finish::Save => (SAVE, size.save),
    }
}

/// icons besides tools
pub const EXTRA_ICONS: &[&str] = &[
    crate::ui::text_properties::icons::SIZE,
    crate::ui::text_properties::icons::OPACITY,
    crate::ui::text_properties::icons::COLOR,
    crate::ui::text_properties::icons::BACKGROUND,
    crate::ui::text_properties::icons::FONT,
    crate::ui::text_properties::icons::EDIT,
    RESET_NUMBER,
    RULER_ANGLE,
    ITALIC,
    BOLD,
    BLURRED,
    PIXELATED,
    RETRY,
    COPY,
    GLOBE,
    DOWNLOAD,
    CLOSE,
    CHEVRON_DOWN,
    TRASH,
    CHECK,
    MARK_SHOT_CHECK,
    PIN,
    SAVE,
    SCROLL,
    SCROLL_HORIZONTAL,
    SCROLL_PAUSE,
    SCROLL_RESUME,
];

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// `load_icons_cache` parses every icon with `expect`, so inccorrent one is
    /// a panic on launch rather than a missing glyph. Catch it here instead.
    #[test]
    fn every_icon_parses() {
        let opt = usvg::Options::default();
        let all = Tool::iter()
            .map(|tool| get_svg_for_tool(tool).0)
            .chain(EXTRA_ICONS.iter().copied());
        for svg in all {
            usvg::Tree::from_str(svg, &opt).expect("embedded icon must parse");
        }
    }
}
