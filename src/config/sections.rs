use super::*;

section!(General {
    to: Outputs = Outputs {
        copy: true,
        save: true,
        ..Outputs::default()
    },
    overlay: OverlayShell = OverlayShell::Auto,
});

section!(Save {
    directory: String = "screenshots".into(),
    month_format: String = "%Y-%m".into(),
    filename_format: String = "%Y-%m-%d_%H-%M".into(),
});

section!(Pinned {
    always_on_top: bool = true,
    border_enabled: bool = true,
    border_color: Rgba = rgb(94, 234, 212),
    border_width: f32 = 2.0,
});

section!(Notifications {
    enabled: bool = true,
    preview: bool = true,
});

section!(Animation {
    speed: f32 = 1.0,
    frame_ms: u64 = 10,
    popover_fade: f32 = 8.0,
    toolbar_fade: f32 = 5.0,
    toolbar_slide: f32 = 12.0,
    toolbar_slide_distance: f32 = 340.0,
    toast_fade_in: f32 = 0.18,
    toast_fade_out: f32 = 0.14,
});

section!(Theme {
    font_size: f32 = 11.0,
    small_font_size: f32 = 11.0,
    line_height: f32 = 1.2,
    background: Rgba = Rgba(248, 250, 252, 242),
    foreground: Rgba = rgb(51, 65, 85),
    muted: Rgba = rgb(100, 116, 139),
    hover: Rgba = rgb(13, 148, 136),
    accent: Rgba = Rgba(13, 148, 136, 36),
    on_accent: Rgba = rgb(51, 65, 85),
    border: Rgba = Rgba(15, 23, 42, 30),
    border_on_light: Rgba = CRUST.with_alpha(55),
    field: Rgba = Rgba(15, 23, 42, 10),
    track: Rgba = Rgba(15, 23, 42, 30),
    caret: Rgba = rgb(15, 23, 42),
    text_selection: Rgba = BLUE.with_alpha(110),
    panel_height: f32 = 42.0,
    panel_padding: f32 = 6.0,
    panel_margin: f32 = 5.0,
    panel_radius: f32 = 11.0,
    item_radius: f32 = 4.0,
    border_width: f32 = 1.0,
    separator_width: f32 = 2.0,
    separator_length: f32 = 0.5,
    separator_radius: f32 = 1.0,
    progress_height: f32 = 4.0,
    caret_width: f32 = 1.5,
    field_text_height: f32 = 0.75,
});

section!(ToolbarIcons {
    selection: f32 = 31.0,
    pick: f32 = 26.0,
    ocr: f32 = 29.5,
    eyedropper: f32 = 28.0,
    text: f32 = 22.0,
    pen: f32 = 20.0,
    highlighter: f32 = 27.0,
    laser: f32 = 25.0,
    marker: f32 = 26.0,
    ruler: f32 = 26.0,
    magnifier: f32 = 26.0,
    line: f32 = 28.0,
    arrow: f32 = 24.0,
    rectangle: f32 = 31.0,
    circle: f32 = 25.0,
    numerated_arrow: f32 = 25.0,
    number: f32 = 26.0,
    redact: f32 = 26.0,
    scroll: f32 = 26.0,
    pin: f32 = 21.0,
    copy: f32 = 20.0,
    save: f32 = 20.0,
});

section!(Toolbar {
    button_size: f32 = 30.0,
    button_gap: f32 = 2.0,
    separator_slot: f32 = 1.0,
    highlight_height: f32 = 0.8,
    highlight_to: bool = false,
    background: ThemeColor = ThemeColor::default(),
    icon: ThemeColor = ThemeColor::default(),
    icon_hovered: ThemeColor = ThemeColor::default(),
    icon_selected: ThemeColor = ThemeColor::default(),
    icon_to: ThemeColor = ThemeColor::default(),
    button_hovered: ThemeColor = ThemeColor::default(),
    button_selected: ThemeColor = ThemeColor::default(),
    separator: ThemeColor = ThemeColor::default(),
    icons: ToolbarIcons = ToolbarIcons::default(),
});

section!(SettingsPanel {
    item_gap: f32 = 8.0,
    item_height: f32 = 0.7,
    separator_slot: f32 = 16.0,
    char_width: f32 = 7.0,
    label_padding: f32 = 8.0,
    swatch_size: f32 = 28.0,
    swatch_dot: f32 = 0.35,
    icon_button_size: f32 = 28.0,
    icon_size: f32 = 16.0,
    copy_icon_size: f32 = 15.0,
    stepper_width: f32 = 84.0,
    stepper_bold: bool = true,
    stepper_arrow_zone: f32 = 30.0,
    stepper_arrow_width: f32 = 7.0,
    stepper_arrow_height: f32 = 3.5,
    stepper_arrow_gap: f32 = 5.0,
    stepper_arrow_stroke: f32 = 1.25,
    hex_width: f32 = 84.0,
    rgb_width: f32 = 120.0,
    download_width: f32 = 196.0,
    download_label_width: f32 = 96.0,
    download_percent_width: f32 = 38.0,
    checkbox_size: f32 = 18.0,
    checkbox_gap: f32 = 6.0,
    checkbox_radius: f32 = 3.0,
    checkmark_width: f32 = 2.0,
    background: ThemeColor = ThemeColor::default(),
    text: ThemeColor = ThemeColor::default(),
    icon: ThemeColor = ThemeColor::default(),
    icon_hovered: ThemeColor = ThemeColor::default(),
    icon_active: ThemeColor = ThemeColor::default(),
    separator: ThemeColor = ThemeColor::default(),
});

section!(ColorPicker {
    max_recent: usize = 6,
    palette: Vec<Rgba> = vec![
        rgb(0, 0, 0),
        rgb(255, 255, 255),
        rgb(243, 139, 168),
        rgb(250, 179, 135),
        rgb(249, 226, 175),
        rgb(166, 227, 161),
    ],
    outline_palette: Vec<Rgba> = vec![
        rgb(0, 0, 0),
        rgb(255, 255, 255),
        rgb(243, 139, 168),
        rgb(249, 226, 175),
        rgb(137, 180, 250),
        rgb(166, 227, 161),
    ],
    padding: f32 = 15.0,
    sv_size: f32 = 170.0,
    sv_radius: f32 = 10.0,
    hue_gap: f32 = 12.0,
    hue_width: f32 = 17.0,
    hue_radius: f32 = 5.0,
    marker_radius: f32 = 10.0,
    marker_stroke: f32 = 2.0,
    marker_outline: f32 = 0.1,
    border_width: f32 = 2.0,
    label_gap: f32 = 10.0,
    label_height: f32 = 14.0,
    label_font_size: f32 = 13.0,
    swatch_row_gap: f32 = 6.0,
    swatch_size: f32 = 22.0,
    swatch_gap: f32 = 8.0,
    swatch_hover_grow: f32 = 3.0,
    eyedropper_icon_size: f32 = 13.0,
    row_gap: f32 = 10.0,
    field_height: f32 = 24.0,
    field_gap: f32 = 6.0,
    field_radius: f32 = 4.0,
    field_text_inset: f32 = 6.0,
    hex_label_width: f32 = 28.0,
    rgba_label_width: f32 = 14.0,
    background: ThemeColor = ThemeColor::default(),
    text: ThemeColor = ThemeColor::default(),
    label: ThemeColor = ThemeColor::default(),
    marker: ThemeColor = ThemeColor::default(),
    marker_outline_color: Rgba = Rgba(0, 0, 0, 160),
});

section!(ModelPicker {
    width: f32 = 320.0,
    padding: f32 = 8.0,
    title_height: f32 = 44.0,
    title_gap: f32 = 4.0,
    row_height: f32 = 44.0,
    row_padding: f32 = 10.0,
    name_top: f32 = 5.0,
    name_height: f32 = 20.0,
    note_top: f32 = 24.0,
    note_height: f32 = 15.0,
    bar_top: f32 = 30.0,
    status_width: f32 = 62.0,
    button_size: f32 = 26.0,
    icon_size: f32 = 14.0,
    background: ThemeColor = ThemeColor::default(),
});

section!(ChoicePicker {
    columns: usize = 6,
    cell_size: f32 = 40.0,
    padding: f32 = 8.0,
    gap: f32 = 4.0,
    title_height: f32 = 26.0,
    label_height: f32 = 20.0,
});

section!(Magnifier {
    zoom: f32 = 6.0,
    cells: u32 = 15,
    offset: f32 = 24.0,
    outline_width: f32 = 2.0,
    grid_width: f32 = 1.0,
    label_height: f32 = 20.0,
    label_gap: f32 = 4.0,
    swatch_size: f32 = 12.0,
    swatch_gap: f32 = 6.0,
    swatch_radius: f32 = 3.0,
    grid: Rgba = TEXT.with_alpha(40),
    crosshair: Rgba = OVERLAY2.with_alpha(80),
    outline: ThemeColor = ThemeColor::default(),
    label_background: ThemeColor = ThemeColor::default(),
    label_text: ThemeColor = ThemeColor::default(),
});

impl Magnifier {
    pub fn size(&self) -> u32 {
        (self.cells as f32 * self.zoom) as u32
    }
}

section!(Selection {
    dim: Rgba = rgb(0, 0, 0).with_alpha(140),
    border: ThemeColor = ThemeColor::default(),
    border_width: f32 = 2.0,
    border_radius: f32 = 8.0,
    nudge: f32 = 1.0,
    nudge_fast: f32 = 10.0,
});

section!(Shadow {
    color: Rgba = CRUST.with_alpha(130),
    offset: [f32; 2] = [0.0, 3.0],
    layers: usize = 2,
    spread: f32 = 1.5,
    falloff: f32 = 1.2,
    pen_halo: f32 = 3.0,
});

section!(Handles {
    color: ThemeColor = ThemeColor::default(),
    width: f32 = 3.0,
    radius: f32 = 4.0,
    min_length: f32 = 8.0,
    corner_length: f32 = 0.2,
    middle_length: f32 = 0.32,
});

section!(Arrow {
    head_length: f32 = 4.0,
    head_min: f32 = 12.0,
    head_max: f32 = 0.6,
    head_width: f32 = 0.55,
});

section!(Numbered {
    circle: f32 = 3.0,
    tail_width: f32 = 0.48,
    digit_sizes: [f32; 4] = [1.1, 0.85, 0.65, 0.5],
    bold: bool = true,
    dark_digits: Rgba = rgb(0, 0, 0),
    light_digits: Rgba = rgb(255, 255, 255),
    digits_flip: f32 = 0.55,
});

section!(TextBox {
    line_height: f32 = 1.2,
    min_width: f32 = 10.0,
    resize_min: f32 = 6.0,
    resize_max: f32 = 300.0,
    outline_width: f32 = 0.04,
    selected_text: ThemeColor = ThemeColor::default(),
});

section!(Annotations {
    shadow: Shadow = Shadow::default(),
    handles: Handles = Handles::default(),
    arrow: Arrow = Arrow::default(),
    numbered: Numbered = Numbered::default(),
    text: TextBox = TextBox::default(),
});

section!(Stroke {
    width: f32 = 12.0,
    min: f32 = 1.0,
    max: f32 = 40.0,
    step: f32 = 1.0,
});

section!(Font {
    size: f32 = 20.0,
    min: f32 = 8.0,
    max: f32 = 300.0,
    step: f32 = 1.0,
    bold: bool = true,
    italic: bool = false,
    outline: bool = false,
    outline_color: Rgba = rgb(0, 0, 0),
});

section!(Redact {
    strength: f32 = crate::features::filter::model::DEFAULT_STRENGTH,
    step: f32 = 1.0,
    mode: Option<crate::features::filter::FilterMode> = None,
    pixelate: Option<bool> = None,
});

section!(Pen {
    smoothing: f32 = 0.7,
    min_distance: f32 = 1.0,
});

section!(Highlighter {
    smoothing: f32 = 0.7,
    min_distance: f32 = 1.0,
    opacity: f32 = 0.4,
});

section!(Laser { width: f32 = 10.0 });

section!(Marker {
    width: f32 = 3.0,
    shape: crate::types::MarkerShape = crate::types::MarkerShape::Triangle,
    filled: bool = true,
});

section!(Ruler {
    width: f32 = 3.0,
    mode: crate::features::ruler::RulerMode = crate::features::ruler::RulerMode::Length,
});

section!(MagnifierTool {
    width: f32 = 3.0,
    zoom: f32 = 2.75,
    shape: crate::features::magnifier::MagnifierShape =
        crate::features::magnifier::MagnifierShape::Circle,
});

section!(NumberTool {
    width: f32 = 3.0,
    style: crate::features::number::NumberStyle = crate::features::number::NumberStyle::Arabic,
});

section!(ArrowTool {
    width: f32 = 3.0,
    style: crate::features::arrow::ArrowStyle = crate::features::arrow::ArrowStyle::Fletched,
    line_style: crate::features::line::LineStyle = crate::features::line::LineStyle::Solid,
});

section!(LineTool {
    width: f32 = 3.0,
    style: crate::features::line::LineStyle = crate::features::line::LineStyle::Solid,
});

section!(Tools {
    color: Rgba = rgb(255, 77, 77),
    fill: bool = false,
    stroke: Stroke = Stroke::default(),
    font: Font = Font::default(),
    redact: Redact = Redact::default(),
    pen: Pen = Pen::default(),
    highlighter: Highlighter = Highlighter::default(),
    laser: Laser = Laser::default(),
    marker: Marker = Marker::default(),
    ruler: Ruler = Ruler::default(),
    magnifier: MagnifierTool = MagnifierTool::default(),
    number: NumberTool = NumberTool::default(),
    arrow: ArrowTool = ArrowTool::default(),
    line: LineTool = LineTool::default(),
});

section!(Toasts {
    height: f32 = 38.0,
    padding: f32 = 18.0,
    margin: f32 = 8.0,
    lifetime: f32 = 4.0,
    short_lifetime: f32 = 3.0,
    copied_lifetime: f32 = 1.4,
    background: ThemeColor = ThemeColor::default(),
    text: ThemeColor = ThemeColor::default(),
});

section!(Input {
    double_click_ms: u64 = 400,
    double_click_distance: f32 = 6.0,
    handle_hit_width: f32 = 20.0,
    corner_ratio: f32 = 0.3,
    corner_min: f32 = 8.0,
    corner_max: f32 = 40.0,
    scroll_sensitivity: f32 = 4.0,
    scroll_step_pixels: f32 = 10.0,
    hold_delay_ms: u64 = 400,
    hold_repeat_ms: u64 = 120,
    hold_fast_after: u32 = 8,
    hold_fast_repeat_ms: u64 = 40,
    snap_angle: f32 = 3.0,
});

section!(OcrLook {
    region_shade: Rgba = CRUST.with_alpha(96),
    plate: Rgba = TEXT.with_alpha(48),
    plate_pad_x: f32 = 4.0,
    plate_pad_y: f32 = 2.0,
    plate_radius: f32 = 5.0,
    selection_radius: f32 = 2.0,
    badge_size: f32 = 62.0,
    badge_radius: f32 = 17.0,
    badge_icon_size: f32 = 30.0,
    scan_rate: f32 = 1.15,
    scan_width: f32 = 2.5,
    badge_background: ThemeColor = ThemeColor::default(),
    badge_icon: ThemeColor = ThemeColor::default(),
    scan: ThemeColor = ThemeColor::default(),
});

section!(OcrLayout {
    block_gap: f32 = 0.4,
    block_gap_max: f32 = 0.7,
    block_overlap: f32 = 0.5,
    block_align: f32 = 0.35,
    row_tolerance: f32 = 0.6,
    row_split_gap: f32 = 4.0,
    column_gutter: f32 = 6.0,
});

section!(OcrDetect {
    max_side: u32 = 1920,
    pad: u32 = 16,
    score_threshold: f32 = 0.3,
    box_threshold: f32 = 0.6,
    unclip_ratio: f32 = 1.5,
    max_candidates: usize = 1000,
});

section!(OcrFilter {
    min_confidence: f32 = 0.55,
    single_char_confidence: f32 = 0.6,
    single_char_height: f32 = 0.6,
    glyph_height: f32 = 1.8,
    glyph_aspect: f32 = 1.2,
    speck_size: f32 = 0.5,
    rule_contrast: i32 = 32,
    rule_coverage: f32 = 0.9,
    rule_reach: f32 = 0.5,
});

section!(Ocr {
    engine: crate::ocr::engine_choice::Engine = crate::ocr::engine_choice::Engine::default(),
    mode: Mode = Mode::OnDemand,
    result_panel: bool = true,
    threads: usize = 4,
    device: Device = Device::Auto,
    daemon_idle_secs: u64 = 900,
    hit_slack: f32 = 3.0,
    vertical_slack: f32 = 10.0,
    min_region: f32 = 16.0,
    lookalikes: bool = true,
    look: OcrLook = OcrLook::default(),
    layout: OcrLayout = OcrLayout::default(),
    detect: OcrDetect = OcrDetect::default(),
    filter: OcrFilter = OcrFilter::default(),
});

section!(Log {
    level: Option<String> = None,
    path: Option<String> = None,
    max_file_size_mb: u64 = 1,
});
