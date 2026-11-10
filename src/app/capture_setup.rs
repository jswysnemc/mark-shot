//! 【截图】【初始化】准备捕获、显示器布局和按需识别状态

use super::*;

/// 【截图】【初始化顺序】用 conn 和 launch 先捕获桌面再显示覆盖层，prof 记录耗时，返回编辑器和覆盖层
pub(super) async fn start_capture(
    conn: crate::backend::session::Session,
    launch: &mut Launch,
    prof: &mut Profiler,
) -> Result<(EditorState, Box<dyn ScreenOverlay>), Box<dyn std::error::Error>> {
    let icons_handle = std::thread::spawn(init::load_icons_cache);
    let system_fonts = std::thread::spawn(crate::fonts::system);

    let mut overlay = initialize_overlay(conn.clone())?;
    let preferred = overlay.set_composited_background(true);
    prof.mark("overlay init");

    let all = overlay.discovered_outputs().to_vec();
    let hidpi = overlay.hidpi();
    let delayed = launch.shot_at.is_some();
    // with a delay the shot can't happen before the deadline anyway, so the backend is
    // created and prepared in advance (on KDE that's connecting to the session bus).
    // that time is part of the delay, not added to it.
    // 1. 【截图】【延迟准备】仅预先连接捕获后端和准备内存，读取桌面像素前不映射覆盖层
    let mut early = None;
    if delayed {
        let capture = early.insert(crate::backend::initialize_capture_with_cursor(
            &conn,
            launch.include_cursor,
        ));
        overlay.prefault();
        prof.mark("pool prefaulted");
        delay::wait_launch(launch, &**capture, prof).await?;
    } else {
        delay::begin_launch(launch)?;
    }

    let one = crate::config::preferences::current().capture.one_monitor(
        launch.one_monitor,
        launch.all_outputs,
        launch.fullscreen,
    );
    let names: Vec<_> = all
        .iter()
        .enumerate()
        .map(|(index, output)| crate::backend::output_selection::name(output, index))
        .collect();
    let active = if one && launch.display_names.is_empty() {
        overlay.active_output()
    } else {
        None
    };
    let targets =
        crate::backend::output_selection::indices(&names, &launch.display_names, !one, active)?;
    let outputs: Vec<_> = targets.iter().map(|&i| all[i].clone()).collect();
    if one {
        prof.mark("active output probed");
    }
    let capture = early.unwrap_or_else(|| {
        crate::backend::initialize_capture_with_cursor(&conn, launch.include_cursor)
    });
    let window_work = async {
        if !launch.fullscreen && crate::config::get().window_detection.enabled {
            crate::backend::window_detection::snapshot(&conn).await
        } else {
            Vec::new()
        }
    };
    // 2. 【截图】【窗口快照】窗口信息与像素都在覆盖层映射前获取，不把本程序收入候选
    let visibility = crate::backend::capture_visibility::Guard::begin(
        crate::config::preferences::current()
            .capture
            .hide_own_windows,
    )?;
    let (screenshots, windows) = tokio::join!(capture.capture_frame(&outputs), window_work);
    drop(visibility);
    let screenshots = screenshots?;
    prof.mark("capture");

    let kept: Vec<usize> = screenshots.frames.iter().map(|f| f.output).collect();
    if kept.is_empty() {
        return Err("no monitor was captured".into());
    }
    let shrunk = kept.len() < outputs.len();
    let outputs: Vec<_> = kept.iter().map(|&i| outputs[i].clone()).collect();

    let captures = init::build_captures(screenshots.frames)?;
    let placements = init::build_placements(&outputs, hidpi);
    let compatible = super::frame_backdrop::compatible(&captures, &placements);
    if !compatible {
        overlay.set_composited_background(false);
    }
    // 3. 【截图】【覆盖层显示】原始帧已经进入私有来源后才分配绘制与原生表面，避免两份整屏缓冲叠加缓存写入峰值
    let present_at = std::time::Instant::now();
    overlay.present(&targets)?;
    let present_dt = present_at.elapsed();
    let (canvas, dimmed, annotations) =
        init::build_layers_with_transparency(&placements, launch.fullscreen);
    prof.mark("base_pixmaps + layers + placements");

    let icons_cache = icons_handle.join().expect("Failed to join icons thread");

    let mut editor_state = EditorState::new(
        Layers {
            captures,
            canvas,
            dimmed,
            annotations,
        },
        placements,
        icons_cache,
        TextState {
            font_system: crate::fonts::bundled(),
            system_fonts: Some(system_fonts),
            swash_cache: SwashCache::new(),
            editors: HashMap::new(),
            editing: None,
        },
        build_ocr_state(),
    );
    editor_state.enable_region_rendering();
    prof.mark("editor_state built");

    super::frame_backdrop::configure(&mut editor_state, overlay.as_mut(), preferred && compatible);
    if shrunk {
        overlay.retain_outputs(&kept)?;
    }
    for (i, capture) in editor_state.captures.iter().enumerate() {
        let pixmap = &capture.pixmap;
        overlay.set_background_source(i, pixmap)?;
    }
    prof.mark("backgrounds uploaded");
    prof.mark("native surfaces prepared");
    prof.mark_external("  ^ native surface preparation", present_dt);

    if !launch.fullscreen {
        super::region_detection::task::start(&mut editor_state, overlay.as_mut(), &windows);
    }

    Ok((editor_state, overlay))
}

pub(super) fn build_ocr_state() -> OcrState {
    let models = crate::ocr::models::OcrModels::load();
    let settings = crate::ocr::settings::EngineSettings::load();
    let mut runtime = crate::ocr::OcrRuntime::new(crate::ocr::daemon::resolve_mode(&settings));
    if let Some(source) = crate::ocr::engine_choice::source(&models) {
        runtime.load_source(source);
    }

    OcrState {
        runtime,
        models,
        view: crate::ocr::OcrView::default(),
        redrag: false,
        redrag_from: None,
        await_region: false,
        scan_started: None,
        result_scale: 1.0,
        open_result_on_completion: false,
        copy_on_completion: false,
    }
}
