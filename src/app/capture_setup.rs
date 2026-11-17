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

    // 1. 【截图】【提前授权】先让 Portal 走完选择来源，再初始化覆盖层读取显示器列表：
    //    用户先开机、再插外接屏时，合成器的布局通知往往和授权窗口的关闭同时到达，
    //    先授权能保证下面读到的位置和数量已经是当前布局，而不是上一版桌面；
    //    只有 Portal 是唯一可用后端时才提前授权，原生协议可用时返回 None，避免多弹权限窗口
    let mut prepared = crate::backend::prepare_portal_capture(&conn, launch.include_cursor).await;

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
        // 2. 【截图】【授权延续】延迟截图本来就要等，把已经完成的授权直接交给提前创建的捕获器，
        //    否则这里会重新申请一次权限，用掉的时间落到倒计时之外
        let capture = early.insert(prepared.take().unwrap_or_else(|| {
            crate::backend::initialize_capture_with_cursor(&conn, launch.include_cursor)
        }));
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
    let capture = early.or(prepared).unwrap_or_else(|| {
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

    let mut captures = init::build_captures(screenshots.frames)?;
    let placements = init::build_placements(&outputs, hidpi);
    let compatible = super::frame_backdrop::compatible(&captures, &placements);
    if !compatible {
        overlay.set_composited_background(false);
    }
    let native_scene = compatible
        && std::env::var_os("MARK_SHOT_CAPTURE_SCENE").is_none_or(|value| value != "0")
        && overlay.enable_capture_scene(&mut captures)?;
    log::info!("【截图场景】【能力选择】稀疏前景={native_scene}，原始网格兼容={compatible}");
    // 3. 【截图】【覆盖层显示】原始帧已经进入私有来源后才分配绘制与原生表面，避免两份整屏缓冲叠加缓存写入峰值
    let present_at = std::time::Instant::now();
    overlay.present(&targets)?;
    let present_dt = present_at.elapsed();
    let (canvas, dimmed, annotations) =
        init::build_layers_with_transparency(&placements, launch.fullscreen || native_scene);
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
    editor_state.region_rendering.native_scene = native_scene;
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
