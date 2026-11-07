pub(crate) mod annotation_collection;
pub(crate) mod annotation_selection;
pub(crate) mod canvas_navigation;
mod capture_setup;
mod cursor_magnifier;
use capture_setup::start_capture;
mod event_loop;
mod external_actions;
pub(crate) mod region_detection;
pub(crate) mod selection_size;
pub(crate) mod selection_translation;
mod theme_switch;
pub(crate) mod upload;
use event_loop::run_overlay;
mod frame_backdrop;
#[cfg(all(test, target_os = "linux"))]
mod frame_lifecycle_overlay;
#[cfg(all(test, target_os = "linux"))]
mod frame_lifecycle_tests;
mod frame_render;
use frame_render::Pacing;
use frame_render::{clear_frame_state, render_dirty_monitors};
mod export;
use export::refresh_panels_after_history;
pub(crate) use export::render_final;
pub use export::selection_render_info;

mod actions;
mod delay;
mod finish;
mod image_export;
mod image_setup;
mod init;
mod input;
mod instant;
mod layers;
mod ocr;
mod panels;
mod pointer_history;
#[cfg(test)]
pub(crate) mod region_rendering_test_support;
mod save_as;
mod scroll;
mod startup_tool;
pub(crate) mod text_caret;
pub(crate) mod text_context_menu;
pub(crate) mod text_input;
pub(crate) mod text_menu_surface;

pub(crate) use init::{build_captures, build_placements, load_icons_cache};
pub(crate) use input::select_tool;

use crate::backend::ScreenOverlay;
use crate::backend::{FRAME_FALLBACK, initialize_overlay};
use crate::editor::dirty::{apply_damage_rects, is_dirty, mark_all_dirty, mark_dirty};
use crate::editor::{EditorState, Layers, OcrState, TextState};
use crate::profiler::Profiler;
use crate::renderer;
use crate::theme::anim;
use crate::tools::Tool;
use crate::tools::selection::{global_selection_to_local, selection_edges_for_monitor};
use crate::types::{DamageRect, Mode, Outputs, OverlayEvent, Placement, SelectionEdges};
use crate::ui::panel::UiPanel;
use crate::ui::panel::{AnimatedPanel, panel_to_draw, tick_panel_animation};
use crate::utils::{encode_png, get_full_workspace_rect};

use cosmic_text::SwashCache;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tiny_skia::Rect;

/// download progress updates each tick, but if there is no animation
/// then manually after each 50ms
const DOWNLOAD_POLL: Duration = Duration::from_millis(50);

const POINTER_GRACE: Duration = Duration::from_millis(100);

// ************************* //
//      ENTRY POINT          //
// ************************* //

/// Launch options from the command line.
#[derive(Debug, Clone, Default)]
pub struct Launch {
    pub mode: Mode,
    pub one_monitor: bool,
    pub to: Option<Outputs>,
    pub output: Option<PathBuf>,
    pub speed: bool,
    pub shot_at: Option<Instant>,
    pub image: Option<PathBuf>,
    pub fullscreen: bool,
    pub display_names: Vec<String>,
    pub include_cursor: bool,
    pub default_tool: Option<Tool>,
    pub fullscreen_default_tool: Option<Tool>,
    pub file_default_tool: Option<Tool>,
    pub default_color: Option<crate::theme::Rgba>,
}

impl Launch {
    /// 【截图】【编辑模式】本地图片、模型引导和无导出输出的 OCR 使用交互编辑器；返回实际会话模式
    fn editor_mode(&self, has_model: bool) -> Mode {
        if self.mode == Mode::Ocr
            && (self.image.is_some() || !has_model || self.outputs().is_empty())
        {
            Mode::Editor
        } else {
            self.mode
        }
    }

    /// 【截图】【任务检查】编辑与 OCR 可以仅显示交互结果；其他快捷模式必须具有导出输出
    fn has_work(&self) -> bool {
        matches!(self.mode, Mode::Editor | Mode::Ocr | Mode::Scroll) || !self.outputs().is_empty()
    }

    fn outputs(&self) -> Outputs {
        let mut outputs = self.to.unwrap_or(crate::config::get().general.to);
        outputs.save |= self.output.is_some();
        if self.mode == Mode::Ocr {
            outputs.pin = false;
            outputs.save = false;
        } else {
            outputs.primary = false;
        }
        outputs
    }

    fn warn_ignored(&self) {
        let to = self.to.unwrap_or_default();
        if self.mode != Mode::Ocr {
            if to.primary {
                log::info!("r (primary selection) works only with -r ocr, ignoring it");
            }
            return;
        }
        if to.pin || to.save {
            log::info!("p and s don't apply to -r ocr, ignoring them");
        }
        if self.output.is_some() {
            log::info!("--output has no effect with -r ocr, ignoring it, use stdout");
        }
    }
}

pub async fn run(
    conn: crate::backend::session::Session,
    launch: Launch,
) -> Result<(), Box<dyn std::error::Error>> {
    launch.warn_ignored();
    if !launch.has_work() {
        return Err("nothing to do with the shot: pass --to or set general.to".into());
    }
    match launch.mode {
        Mode::Editor | Mode::Region | Mode::Ocr | Mode::Scroll => {
            make_screenshot(conn, launch).await
        }
        Mode::Full => instant::full(conn, launch).await,
        Mode::Window => instant::window(conn, launch).await,
    }
}

/// 【截图会话】【完整编辑】按 conn 与 launch 运行截图、滚动和结果交接，返回会话完成状态或具体错误
async fn make_screenshot(
    conn: crate::backend::session::Session,
    mut launch: Launch,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut prof = Profiler::new(launch.speed);

    // scrolling capture needs its own connection to grab frames
    let scroll_conn = conn.clone();
    let (mut editor_state, mut overlay) = if launch.image.is_some() {
        image_setup::start(conn, &launch, &mut prof)?
    } else {
        start_capture(conn, &mut launch, &mut prof).await?
    };
    editor_state.to = launch.outputs();
    editor_state.toolbar.to = editor_state.to;
    editor_state.output = launch.output.clone();
    editor_state.mode = launch.editor_mode(editor_state.ocr.models.active_installed());
    editor_state.ocr.copy_on_completion =
        launch.mode == Mode::Ocr && launch.to.is_some_and(|outputs| outputs.copy);
    editor_state.ocr.open_result_on_completion =
        launch.mode == Mode::Ocr && crate::features::ocr_result::enabled();
    editor_state.after_selection_tool = launch.default_tool.unwrap_or(Tool::Selection);
    if let Some(color) = launch.default_color {
        editor_state.tool_settings.color = color.color();
    }
    editor_state.fullscreen_scope = launch.fullscreen || launch.image.is_some();
    if launch.fullscreen || launch.image.is_some() {
        if editor_state.selection.zone.is_none() {
            editor_state.selection.zone = get_full_workspace_rect(&editor_state.placements);
        }
        let initial_tool = startup_tool::initial(&launch);
        select_tool(&mut editor_state, initial_tool, &mut 0);
    }
    if launch.mode == Mode::Ocr && editor_state.mode == Mode::Editor {
        // 1. 首次显示模型引导；已下载模型时直接识别本地原图
        select_tool(&mut editor_state, Tool::Ocr, &mut 0);
    }
    if editor_state.region() {
        editor_state.ui_hidden = true;
        editor_state.toolbar.start_hidden();
    }
    // when launched with -S, let the user select a region first; scrolling takes over once ready
    if launch.mode == Mode::Scroll {
        editor_state.toasts.show(
            crate::ui::toast::ToastKind::ScrollNeedsRegion,
            &mut editor_state.text.font_system,
        );
    }
    if launch.mode == Mode::Ocr {
        // preparing the ocr engine immediately after launch,
        // by default, in the editor, it does that only after choosing ocr
        editor_state.ocr.runtime.prepare();
    }

    input::refresh_panels(&mut editor_state, &mut 0);
    init::initial_paint(&mut editor_state, &mut overlay, &mut prof)?;
    if let Some(controller) = crate::features::single_instance::context::current() {
        controller.active();
    }
    prof.dump();

    loop {
        let scroll_mode = run_overlay(&mut editor_state, &mut overlay)?;
        // (saving what user chose)
        crate::editor::saved::save(
            &editor_state.tool_settings,
            &editor_state.color_popover.history,
            &editor_state.color_popover.initial_history,
            !editor_state.magnifier.hidden,
        );

        if scroll_mode {
            if let Some((image, display)) =
                finish::finish_scroll(&scroll_conn, &mut editor_state, &mut overlay).await?
            {
                // 1. 【滚动截图】【编辑入口】释放滚动浮层后映射原图编辑器，保留完整像素和当前导出策略
                overlay.suspend()?;
                editor_state.ocr.models.shutdown();
                // 2. 【滚动截图】【替换峰值】创建结果编辑器前释放旧画布和原生缓冲
                drop(editor_state);
                drop(overlay);
                crate::utils::memory::release_unused();
                let image_launch = Launch {
                    mode: Mode::Editor,
                    fullscreen: true,
                    display_names: vec![display],
                    ..launch.clone()
                };
                let (mut next, mut next_overlay) = image_setup::start_pixmap(
                    scroll_conn.clone(),
                    &image_launch,
                    &mut prof,
                    image,
                )?;
                next.to = launch.outputs();
                next.toolbar.to = next.to;
                next.output = launch.output.clone();
                next.fullscreen_scope = true;
                next.after_selection_tool = Tool::Selection;
                input::refresh_panels(&mut next, &mut 0);
                init::initial_paint(&mut next, &mut next_overlay, &mut prof)?;
                crate::utils::memory::release_unused();
                editor_state = next;
                overlay = next_overlay;
                continue;
            }
            editor_state.ocr.models.shutdown();
            return Ok(());
        }
        editor_state.ocr.models.shutdown();
        if editor_state.upload.copied {
            drop(overlay);
            crate::backend::notify::send(crate::backend::notify::Notice::ImageUrlCopied).await;
            return Ok(());
        }
        if editor_state.mode == Mode::Ocr {
            return ocr::finish(&mut editor_state).await;
        } else {
            return finish::finish_capture(&mut editor_state).await;
        }
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    /// 【启动测试】【缓存识别】验证本地图片在首次和已安装模型两种情况下都进入 OCR 编辑器
    #[test]
    fn file_ocr_always_keeps_an_interactive_result() {
        let file = Launch {
            mode: Mode::Ocr,
            image: Some(PathBuf::from("sample.png")),
            ..Launch::default()
        };
        assert_eq!(file.editor_mode(false), Mode::Editor);
        assert_eq!(file.editor_mode(true), Mode::Editor);
        let screen = Launch {
            mode: Mode::Ocr,
            ..Launch::default()
        };
        assert_eq!(screen.editor_mode(false), Mode::Editor);
        assert_eq!(screen.editor_mode(true), Mode::Ocr);
    }

    /// 【启动测试】【只保存配置】验证 OCR 不因全局输出仅包含图片保存而拒绝启动
    #[test]
    fn ocr_can_show_results_without_an_image_delivery_target() {
        let launch = Launch {
            mode: Mode::Ocr,
            to: Some(Outputs {
                save: true,
                ..Outputs::default()
            }),
            ..Launch::default()
        };
        assert!(launch.outputs().is_empty());
        assert!(launch.has_work());
        assert_eq!(launch.editor_mode(true), Mode::Editor);
    }

    /// 【启动测试】【空输出检查】验证纯交互模式允许空输出，快捷图片捕获仍要求导出目标
    #[test]
    fn only_interactive_modes_accept_empty_outputs() {
        for mode in [
            Mode::Editor,
            Mode::Ocr,
            Mode::Region,
            Mode::Full,
            Mode::Window,
            Mode::Scroll,
        ] {
            let launch = Launch {
                mode,
                to: Some(Outputs::default()),
                ..Launch::default()
            };
            assert_eq!(
                launch.has_work(),
                matches!(mode, Mode::Editor | Mode::Ocr | Mode::Scroll)
            );
        }
    }
}
