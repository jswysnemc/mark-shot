pub mod annotation_drag;
pub mod color_selection;
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
pub mod ruler_labels;
pub mod ruler_session;
pub mod saved;
pub mod text_composition;
pub mod text_edits;
pub mod text_session;
pub mod text_session_format;
pub mod text_session_history;
pub mod text_size;
pub mod text_size_history;

use cosmic_text::{Editor, FontSystem, SwashCache};
use std::collections::HashMap;
use std::time::Instant;
use tiny_skia::{Color, PathBuilder, Pixmap, Rect};
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
    pub text_input: crate::features::text::input_method::focus::Target,
    pub keyboard_focus: Option<usize>,
    pub popup_serial: Option<u32>,
    pub pointer: PointerState,
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
            text_input: Default::default(),
            keyboard_focus: None,
            popup_serial: None,
            pointer: PointerState::default(),
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
    pub tool_active: bool,
    pub input: InputState,
    pub magnifier: Magnifier,
    pub selection: SelectionState,
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

    pub selected_annotation: Option<usize>,
    pub ann_drag: Option<AnnDragState>,

    pub annotations_layer: Vec<Pixmap>,
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
    pub finish: Option<crate::types::Outputs>,
    /// 【截图另存为】【请求状态】一次性打开对话框的请求，不属于标注或撤销历史
    pub save_as_requested: bool,
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
            tool_active: false,
            selection: SelectionState::default(),
            input: InputState::default(),
            magnifier: Magnifier {
                hidden: crate::editor::saved::get().magnifier == Some(false),
                ..Magnifier::default()
            },

            toolbar: Toolbar::new(),
            action_toolbar: Toolbar::actions(),
            toolbar_drag: None,
            scope_before_fullscreen: None,
            fullscreen_scope: false,
            settings_panel: SettingsPanel::new(),
            font_panel: Default::default(),
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
            selected_annotation: None,
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
            finish: None,
            save_as_requested: false,
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
        !self.magnifier.hidden || self.picking()
    }

    // to avoid revbuilding the entire annotation layer like it was implemented before
    // instead commited annotations are `baked`, so pending new annotations are separate from them
    // so there will be absolutely no lags while drawing something on top of 10000th circles
    pub fn bake_annotation(&mut self, ann: &Annotation) {
        self.cache_filter(ann);
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
            }
        }
    }

    /// where the dragged annotation is drawn, it's out of `annotations` while dragged
    pub fn dragged_index(&self) -> Option<usize> {
        self.ann_drag
            .as_ref()
            .filter(|_| self.pending.is_some())
            .map(|drag| drag.orig_index.min(self.annotations.len()))
    }

    /// 【画笔缓存】【片段绘制】缓存 start、control、end 构成的笔迹；color、stroke_width 控制样式，返回全局重绘区域
    pub fn bake_pen_segment(
        &mut self,
        start: (f32, f32),
        control: Option<(f32, f32)>,
        end: (f32, f32),
        color: Color,
        stroke_width: f32,
    ) -> Rect {
        let mut pb = PathBuilder::new();
        pb.move_to(start.0, start.1);
        if let Some(ctrl) = control {
            pb.quad_to(ctrl.0, ctrl.1, end.0, end.1);
        } else {
            pb.line_to(end.0, end.1);
        }

        let pad = crate::renderer::visual_pad(stroke_width);
        let min_x = start
            .0
            .min(end.0)
            .min(control.map(|c| c.0).unwrap_or(start.0))
            - pad;
        let min_y = start
            .1
            .min(end.1)
            .min(control.map(|c| c.1).unwrap_or(start.1))
            - pad;
        let max_x = start
            .0
            .max(end.0)
            .max(control.map(|c| c.0).unwrap_or(start.0))
            + pad;
        let max_y = start
            .1
            .max(end.1)
            .max(control.map(|c| c.1).unwrap_or(start.1))
            + pad;
        let segment_bbox = Rect::from_ltrb(min_x, min_y, max_x, max_y).unwrap_or_else(|| {
            Rect::from_xywh(start.0 - pad, start.1 - pad, pad * 2.0, pad * 2.0).unwrap()
        });

        if let Some(path) = pb.finish() {
            for (i, placement) in self.placements.iter().enumerate() {
                let offset = placement.offset();
                let visual = Rect::from_ltrb(
                    segment_bbox.left() - offset.0,
                    segment_bbox.top() - offset.1,
                    segment_bbox.right() - offset.0,
                    segment_bbox.bottom() - offset.1,
                );
                if let (Some(vis), Some(mon)) = (visual, placement.rect())
                    && rects_overlap(&vis, &mon)
                {
                    // 1. 【画笔缓存】【片段绘制】先保存当前像素，提交和取消时才能精确还原
                    if let Some(pixel_rect) = crate::renderer::scale_rect(&vis, placement.scale)
                        && let Some((x, y, width, height)) = crate::renderer::rect_bounds(
                            &pixel_rect,
                            self.annotations_layer[i].width(),
                            self.annotations_layer[i].height(),
                        )
                        && let Some(rect) =
                            tiny_skia::IntRect::from_xywh(x as i32, y as i32, width, height)
                    {
                        self.pen_backup
                            .snapshot(i, &self.annotations_layer[i], rect);
                    }
                    crate::renderer::stroke_pen_segment(
                        &mut crate::renderer::Painter::new(&mut self.annotations_layer[i])
                            .with_scale(placement.scale)
                            .with_origin(offset),
                        &path,
                        color,
                        stroke_width,
                    );
                }
            }
        }

        segment_bbox
    }
}
