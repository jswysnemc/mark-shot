//! 【截图】【初始化】准备捕获、显示器布局和按需识别状态

use super::*;

/// Shows the overlay and builds the editor at the same time. The overlay renders
/// on its own thread while screens are captured, running both slow setup steps in parallel.
pub(super) async fn start_capture(
    conn: wayland_client::Connection,
    launch: &Launch,
    prof: &mut Profiler,
) -> Result<(EditorState, Box<dyn ScreenOverlay>), Box<dyn std::error::Error>> {
    let icons_handle = std::thread::spawn(init::load_icons_cache);
    let system_fonts = std::thread::spawn(crate::fonts::system);

    let mut overlay = initialize_overlay(conn.clone())?;
    prof.mark("overlay init");

    let all = overlay.discovered_outputs().to_vec();
    let hidpi = overlay.hidpi();
    let delayed = launch.shot_at.is_some();
    // with a delay the shot can't happen before the deadline anyway, so the backend is
    // created and prepared in advance (on KDE that's connecting to the session bus).
    // that time is part of the delay, not added to it.
    // without a delay it stays after the present spawn, so present starts sooner
    let mut early = None;
    if delayed {
        let capture = early.insert(crate::backend::initialize_capture_with_cursor(
            &conn,
            launch.include_cursor,
        ));
        overlay.prefault();
        prof.mark("pool prefaulted");
        delay::wait(launch.shot_at, &**capture, prof).await?;
    }

    let one = launch.one_monitor;
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
    let layers_handle = {
        let placements = init::build_placements(&outputs, hidpi);
        std::thread::spawn(move || init::build_layers(&placements))
    };
    // with delay window is created only after the shot, since at least
    // kwin puts a new window into the taskbar and pins it to the current
    // virtual desktop right at creation, while doing it before capturing
    // doesn't really gain anything, less than 1ms, and might be unstable
    let (shot_tx, shot_rx) = std::sync::mpsc::channel::<()>();
    let present_handle = std::thread::spawn(move || {
        if delayed {
            let _ = shot_rx.recv();
        }
        let t = std::time::Instant::now();
        let res = overlay.present(&targets).map_err(|e| e.to_string());
        (overlay, res, t.elapsed())
    });

    let capture = early.unwrap_or_else(|| {
        crate::backend::initialize_capture_with_cursor(&conn, launch.include_cursor)
    });
    let screenshots = capture.capture_frame(&outputs).await?;
    drop(shot_tx);
    prof.mark("capture");

    let kept: Vec<usize> = screenshots.frames.iter().map(|f| f.output).collect();
    if kept.is_empty() {
        return Err("no monitor was captured".into());
    }
    let shrunk = kept.len() < outputs.len();
    let outputs: Vec<_> = kept.iter().map(|&i| outputs[i].clone()).collect();

    let captures = init::build_captures(screenshots.frames)?;
    let placements = init::build_placements(&outputs, hidpi);
    let layers = layers_handle.join().expect("Failed to join layers thread");
    let (canvas, dimmed, annotations) = if kept.iter().copied().eq(0..kept.len()) && !shrunk {
        layers
    } else {
        init::build_layers(&placements)
    };
    prof.mark("base_pixmaps + layers + placements");

    let icons_cache = icons_handle.join().expect("Failed to join icons thread");

    let editor_state = EditorState::new(
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
    prof.mark("editor_state built");

    let (mut overlay, present_res, present_dt) = present_handle
        .join()
        .map_err(|_| "present thread panicked")?;
    present_res.map_err(|msg| -> Box<dyn std::error::Error> { msg.into() })?;
    if shrunk {
        overlay.retain_outputs(&kept)?;
    }
    for (i, capture) in editor_state.captures.iter().enumerate() {
        let pixmap = &capture.pixmap;
        overlay.set_background(i, pixmap.data(), pixmap.width(), pixmap.height())?;
    }
    prof.mark("backgrounds uploaded");
    prof.mark("present() joined");
    prof.mark_external("  ^ present_dt (thread-internal duration)", present_dt);

    Ok((editor_state, overlay))
}

pub(super) fn build_ocr_state() -> OcrState {
    let models = crate::ocr::models::OcrModels::load();
    let settings = crate::ocr::settings::EngineSettings::load();
    let mut runtime = crate::ocr::OcrRuntime::new(crate::ocr::daemon::resolve_mode(&settings));
    if let Some(files) = models.active().and_then(|idx| models.files(idx)) {
        runtime.load(files);
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
