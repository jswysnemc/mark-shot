pub(crate) mod annotation_collection;
pub mod annotation_drag;
mod annotation_style;
pub mod canvas_navigation;
pub mod color_selection;
pub mod cursor_magnifier;
pub mod dirty;
pub mod drawing_gesture;
pub mod edits;
pub mod file_canvas;
pub mod filter_cache;
pub mod filter_edits;
pub mod history;
pub mod history_snapshot;
pub mod line_edits;
pub mod line_session;
pub mod magnifier_edits;
pub mod number_edits;
mod pen_cache;
pub mod pen_layer_backup;
pub(crate) mod region_rendering;
pub mod ruler_labels;
pub mod ruler_session;
pub mod saved;
pub mod selection_geometry;
pub mod shape_edits;
pub mod text_composition;
pub mod text_edits;
pub mod text_session;
pub mod text_session_format;
pub mod text_session_history;
mod text_session_layout;
pub mod text_size;
pub mod text_size_history;

use cosmic_text::{Editor, FontSystem, SwashCache};
use std::collections::HashMap;
use std::time::Instant;
use tiny_skia::{Pixmap, Rect};
use usvg::Tree;

use crate::editor::filter_cache::FilterCache;
use crate::interaction::{ClickTarget, DoubleClickTracker};
use crate::tools::Tool;
use crate::types::{
    AnnDragState, Annotation, Capture, Mode, Placement, PointerState, SelectionState,
    TextEditState, ToolSettings,
};
use crate::ui::color_popover::ColorPickerPopover;
use crate::ui::magnifier::MagnifierState;
use crate::ui::settings_panel::SettingsPanel;
use crate::ui::toolbar::Toolbar;
use crate::utils::rects_overlap;

/// Pointer and keyboard state of the current gesture.
pub struct InputState {
    pub(crate) capture_gestures: crate::app::input::capture_actions::Gestures,
    pub text_input: crate::features::text::input_method::focus::Target,
    pub keyboard_focus: Option<usize>,
    pub popup_serial: Option<u32>,
    pub pointer: PointerState,
    /// 【选区尺寸】【平台时间】本次按钮携带的 Wayland 毫秒时间，事件分派结束后清理
    pub pointer_timestamp: Option<u32>,
    pub drag_start: Option<(f64, f64)>,
    pub mouse_down: bool,
    pub left_canvas: bool,
    pub toolbar_press: Option<(bool, usize, usize)>,
    pub color_button_press: Option<(crate::ui::color_popover::ColorTarget, usize, bool)>,
    pub text_property_press: Option<crate::ui::text_properties::state::Press>,
    pub text_wheels: crate::ui::text_properties::state::WheelInputs,
    pub right_clicks: DoubleClickTracker<ClickTarget>,
    pub ctrl: bool,
    pub shift: bool,
    pub straight: bool,
    pub clicks: DoubleClickTracker<ClickTarget>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            capture_gestures: Default::default(),
            text_input: Default::default(),
            keyboard_focus: None,
            popup_serial: None,
            pointer: PointerState::default(),
            pointer_timestamp: None,
            drag_start: None,
            mouse_down: false,
            left_canvas: false,
            toolbar_press: None,
            color_button_press: None,
            text_property_press: None,
            text_wheels: Default::default(),
            right_clicks: DoubleClickTracker::new(),
            ctrl: false,
            shift: false,
            straight: false,
            clicks: DoubleClickTracker::new(),
        }
    }
}

/// The loupe under the cursor: where it is now and where the last frame drew it.
#[derive(Default)]
pub struct Magnifier {
    pub current: Option<MagnifierState>,
    pub prev: Option<MagnifierState>,
    pub hidden: bool,
    pub visible: bool,
}

/// Text shaping and the live editors, one per text annotation.
///
/// Its fields are borrowed apart all over the tools (an editor and the font
/// system at the same time), so this deliberately has no `&mut self` methods.
pub struct TextState {
    pub font_system: FontSystem,
    /// everything not included in Intern font loads up later after launch
    pub system_fonts: Option<std::thread::JoinHandle<cosmic_text::fontdb::Database>>,
    pub swash_cache: SwashCache,
    pub editors: HashMap<u64, Editor<'static>>,
    pub editing: Option<TextEditState>,
}

/// The OCR engine, the installed models and the result on screen.
pub struct OcrState {
    pub runtime: crate::ocr::OcrRuntime,
    pub models: crate::ocr::models::OcrModels,
    pub view: crate::ocr::OcrView,
    /// Set when the user drags to select a new OCR area. It saves the original
    /// selection from before the drag started.
    ///
    /// When the drag ends, OCR only re-runs if the box size actually changed.
    /// This prevents a simple click from clearing the current result.
    pub redrag: bool,
    pub redrag_from: Option<Rect>,
    pub await_region: bool,
    /// When the running recognition started, driving the progress badge's spin.
    /// `None` whenever nothing is in flight.
    pub scan_started: Option<Instant>,
    pub result_scale: f32,
    /// 【文字识别】【结果流程】主 OCR 命令识别完成后转交独立结果窗口
    pub open_result_on_completion: bool,
    pub copy_on_completion: bool,
}

pub struct EditorState {
    pub captures: Vec<Capture>,
    pub canvas: Vec<Pixmap>,
    pub dimmed: Vec<Pixmap>,
    pub placements: Vec<Placement>,
    pub selected_tool: Tool,
    pub after_selection_tool: Tool,
    pub file_canvas: Option<file_canvas::FileCanvas>,
    pub(crate) selection_history: crate::features::selection_history::History,
    pub(crate) image_window: crate::ui::image_window::State,
    pub(crate) capture_start: crate::ui::capture_start::State,
    pub(crate) navigation: canvas_navigation::state::State,
    pub tool_active: bool,
    pub input: InputState,
    pub magnifier: Magnifier,
    pub selection: SelectionState,
    pub(crate) region_detection: crate::app::region_detection::State,
    pub icons_cache: HashMap<&'static str, Tree>,
    pub damage_rects: Vec<DamageZone>,

    pub toolbar: Toolbar,
    pub action_toolbar: Toolbar,
    /// 【截图工具栏】【拖动状态】面板类型及指针相对面板左上角的位置
    pub toolbar_drag: Option<(bool, (f32, f32))>,
    pub scope_before_fullscreen: Option<Rect>,
    pub fullscreen_scope: bool,
    pub settings_panel: SettingsPanel,
    pub font_panel: crate::ui::font_panel::FontPanel,
    pub external_panel: crate::ui::external_panel::ExternalPanel,
    pub external_action_requested: Option<crate::features::external_actions::Action>,
    pub(crate) upload: crate::app::upload::State,
    pub(crate) selection_translation: crate::app::selection_translation::State,
    pub(crate) selection_size: crate::app::selection_size::State,
    pub color_popover: ColorPickerPopover,
    pub model_popover: crate::ui::model_popover::ModelPopover,
    pub marker_popover: crate::ui::choice_popover::ChoicePopover,
    pub ruler_popover: crate::ui::choice_popover::ChoicePopover,
    pub number_popover: crate::ui::choice_popover::ChoicePopover,
    pub arrow_popover: crate::ui::choice_popover::ChoicePopover,
    pub line_popover: crate::ui::choice_popover::ChoicePopover,
    pub filter_popover: crate::ui::choice_popover::ChoicePopover,
    pub toasts: crate::ui::toast::Toasts,
    // annotations
    pub annotations: Vec<Annotation>,
    pub pending: Option<Annotation>,
    pub prev_pending: Option<Annotation>,
    pub next_id: u64,

    pub undo_stack: Vec<history_snapshot::HistorySnapshot>,
    pub redo_stack: Vec<history_snapshot::HistorySnapshot>,
    pub number_sequence: crate::features::number::NumberSequence,
    pub line_session: line_session::LineSession,

    pub(crate) collection: annotation_collection::state::State,
    pub(crate) annotation_selection: crate::app::annotation_selection::State,
    pub ann_drag: Option<AnnDragState>,

    pub annotations_layer: Vec<Pixmap>,
    pub(crate) region_rendering: region_rendering::State,
    pub annotations_dirty: bool,
    pub filters: FilterCache,
    /// 【激光】【临时状态】不进入持久标注、撤销记录或导出图像
    pub laser: crate::features::laser_trace::LaserTrace,
    pub ruler: crate::features::ruler::state::RulerState,

    // Regions of the persistent annotation layers that must be cleared
    // and rebuilt. Separate from `damage_rects`, since layer damage
    // describes what must be rerendered in the cached annotation layer,
    // while damage_rects describes what must be rerendered on the final canvas.
    pub layer_damage_rects: Vec<Rect>,
    pub pending_pen_baked: usize,
    /// 【画笔缓存】【原始像素】仅保存当前手势触及的像素块，提交和取消后释放
    pub(crate) pen_backup: pen_layer_backup::PenLayerBackup,
    pub text: TextState,
    pub text_session: Option<text_session::Session>,
    pub text_context_menu: Option<crate::ui::context_menu::ContextMenu>,
    pub next_menu_id: u64,
    pub pending_text_menu_action: Option<crate::tools::text::session_input::Command>,
    pub text_size_history: text_size_history::TextSizeHistory,
    pub tool_settings: ToolSettings,

    /// Set to true when the user is about to pick a color from the palette, since the action is a one-time thing
    pub pick_once: bool,
    pub color_inspector: crate::features::color_sample::inspector::Inspector,
    pub finish: Option<crate::types::Outputs>,
    /// 【截图另存为】【请求状态】一次性打开对话框的请求，不属于标注或撤销历史
    pub save_as_requested: bool,
    /// 【设置窗口】【请求状态】临时隐藏覆盖层打开设置，返回时保留截图、选区和历史
    pub preferences_requested: bool,
    /// 【截图另存为】【完成事实】已成功保存的独立目标路径，不触发常规预设输出
    pub saved_as: Option<std::path::PathBuf>,
    /// What to do with the screenshot
    /// in the ui, finish adds it alongside the user chioce
    pub to: crate::types::Outputs,
    pub cancel: bool,
    pub ui_hidden: bool,
    /// `--region`: no toolbar, releasing a freshly dragged selection finishes the shot.
    pub mode: Mode,
    /// `--output`: explicitely set the output folder path, overriding the one in config
    pub output: Option<std::path::PathBuf>,

    pub ocr: OcrState,
}

// types.rs
#[derive(Clone, Copy)]
pub enum DamageZone {
    Global(Rect),
    Local { monitor_idx: usize, rect: Rect },
}

/// What the capture pipeline has to build before the editor can start.
pub struct Layers {
    pub captures: Vec<Capture>,
    pub canvas: Vec<Pixmap>,
    pub dimmed: Vec<Pixmap>,
    pub annotations: Vec<Pixmap>,
}

impl EditorState {
    pub fn new(
        layers: Layers,
        placements: Vec<Placement>,
        icons_cache: HashMap<&'static str, Tree>,
        text: TextState,
        ocr: OcrState,
    ) -> Self {
        Self {
            captures: layers.captures,
            canvas: layers.canvas,
            dimmed: layers.dimmed,
            annotations_layer: layers.annotations,
            region_rendering: Default::default(),
            placements,
            icons_cache,
            text,
            text_session: None,
            text_context_menu: None,
            next_menu_id: 1,
            pending_text_menu_action: None,
            text_size_history: Default::default(),
            ocr,

            selected_tool: Tool::Selection,
            after_selection_tool: Tool::Selection,
            file_canvas: None,
            selection_history: Default::default(),
            image_window: Default::default(),
            capture_start: Default::default(),
            navigation: Default::default(),
            selection_size: Default::default(),
            tool_active: false,
            selection: SelectionState::default(),
            region_detection: Default::default(),
            input: InputState::default(),
            magnifier: Magnifier {
                hidden: crate::config::preferences::current()
                    .startup
                    .selection_loupe
                    == Some(false),
                ..Magnifier::default()
            },

            toolbar: Toolbar::new(),
            action_toolbar: Toolbar::actions(),
            toolbar_drag: None,
            scope_before_fullscreen: None,
            fullscreen_scope: false,
            settings_panel: SettingsPanel::new(),
            font_panel: Default::default(),
            external_panel: Default::default(),
            external_action_requested: None,
            upload: Default::default(),
            selection_translation: Default::default(),
            color_popover: ColorPickerPopover::new(),
            model_popover: crate::ui::model_popover::ModelPopover::new(),
            marker_popover: crate::ui::choice_popover::ChoicePopover::new(
                "Marker shapes",
                crate::types::MarkerShape::ALL
                    .map(|shape| crate::ui::choice_popover::ChoiceOption {
                        label: shape.label(),
                    })
                    .to_vec(),
            ),
            ruler_popover: crate::ui::choice_popover::ChoicePopover::new(
                "Measure mode",
                crate::features::ruler::RulerMode::ALL
                    .map(|mode| crate::ui::choice_popover::ChoiceOption {
                        label: mode.label(),
                    })
                    .to_vec(),
            ),
            number_popover: crate::ui::choice_popover::ChoicePopover::new(
                "Number style",
                crate::features::number::NumberStyle::ALL
                    .map(|style| crate::ui::choice_popover::ChoiceOption {
                        label: style.description(),
                    })
                    .to_vec(),
            ),
            arrow_popover: crate::ui::line_properties::LineChoice::Heads.popover(),
            line_popover: crate::ui::line_properties::LineChoice::Pattern.popover(),
            filter_popover: crate::ui::filter_properties::popover(),
            toasts: crate::ui::toast::Toasts::default(),

            annotations: Vec::new(),
            pending: None,
            prev_pending: None,
            next_id: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            number_sequence: crate::features::number::NumberSequence::default(),
            line_session: line_session::LineSession::default(),
            collection: Default::default(),
            annotation_selection: Default::default(),
            ann_drag: None,
            annotations_dirty: false,
            filters: FilterCache::default(),
            laser: crate::features::laser_trace::LaserTrace::default(),
            ruler: crate::features::ruler::state::RulerState::default(),
            // Number of points of the current pending Pen already baked into persistent layer
            pending_pen_baked: 0,
            pen_backup: pen_layer_backup::PenLayerBackup::default(),

            damage_rects: Vec::new(),
            layer_damage_rects: Vec::new(),

            tool_settings: ToolSettings::default(),
            pick_once: false,
            color_inspector: Default::default(),
            finish: None,
            save_as_requested: false,
            preferences_requested: false,
            saved_as: None,
            to: crate::config::get().general.to,
            cancel: false,
            ui_hidden: false,
            mode: Mode::Editor,
            output: None,
        }
    }

    pub fn region(&self) -> bool {
        matches!(self.mode, Mode::Region | Mode::Ocr)
    }

    /// Returns true if the user is currently picking a color
    pub fn picking(&self) -> bool {
        self.selected_tool == Tool::Eyedropper || self.pick_once
    }

    pub fn magnifier_shown(&self) -> bool {
        cursor_magnifier::visible(self)
    }

    /// 【标注绘制】【正文提交】按ann建立滤镜来源；区域模式保留矢量正文，旧缓存模式绘入持久图层，无返回值
    pub fn bake_annotation(&mut self, ann: &Annotation) {
        self.cache_filter(ann);
        if !self.cache_annotation_pixels(ann) {
            return;
        }
        for (i, placement) in self.placements.iter().enumerate() {
            let offset = placement.offset();
            let pad = ann.visual_pad();
            let visual = Rect::from_ltrb(
                ann.bbox.left() - offset.0 - pad,
                ann.bbox.top() - offset.1 - pad,
                ann.bbox.right() - offset.0 + pad,
                ann.bbox.bottom() - offset.1 + pad,
            );
            if let (Some(vis), Some(mon)) = (visual, placement.rect())
                && rects_overlap(&vis, &mon)
            {
                crate::renderer::draw_annotation(
                    &mut crate::renderer::Painter::new(&mut self.annotations_layer[i])
                        .with_scale(placement.scale)
                        .with_frozen_image(crate::renderer::FrozenImage::from_parts(
                            self.file_canvas.as_ref(),
                            &self.captures,
                            &self.placements,
                        ))
                        .with_drawing_bounds(self.selection.zone)
                        .with_text(&mut self.text.font_system, &mut self.text.swash_cache)
                        .with_origin(offset),
                    ann,
                    false,
                    &mut self.text.editors,
                    &self.filters.images,
                    None,
                );
                // 1. 【标注缓存】【提交后归还】完整正文已经落入原有栅格，归还透明页而不裁切笔迹或改变预乘颜色
                crate::utils::transparent_pixmap::reclaim_blank(&mut self.annotations_layer[i]);
            }
        }
    }

    /// 【标注编辑】【调整顺序】无额外参数，返回实时标注在持久数组中的原始位置；没有调整草稿时返回空值
    pub fn dragged_index(&self) -> Option<usize> {
        self.ann_drag
            .as_ref()
            .filter(|_| self.pending.is_some())
            .map(|drag| drag.orig_index.min(self.annotations.len()))
    }
}
