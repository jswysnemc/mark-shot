// --full and --window: take the shot and hand it over right away, no overlay and no editor

use crate::backend::initialize_overlay;
use crate::profiler::Profiler;
use crate::renderer;
use crate::utils::{encode_png, encode_png_with_alpha, get_full_workspace_rect};

use super::finish::deliver;
use super::{Launch, delay, init};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

pub async fn full(conn: crate::backend::session::Session, launch: Launch) -> Result {
    let mut prof = Profiler::new(launch.speed);

    // 'overlay' may be misleading, we use get monitor list and probe of the active
    // witout actually showing the overlay
    let mut overlay = initialize_overlay(conn.clone())?;
    prof.mark("outputs discovered");
    let capture = crate::backend::initialize_capture_with_cursor(&conn, launch.include_cursor);
    delay::wait(launch.shot_at, &*capture, &mut prof).await?;
    let all = overlay.discovered_outputs().to_vec();
    let names: Vec<_> = all
        .iter()
        .enumerate()
        .map(|(index, output)| crate::backend::output_selection::name(output, index))
        .collect();
    let active = if launch.one_monitor && launch.display_names.is_empty() {
        overlay.active_output()
    } else {
        None
    };
    let indices = crate::backend::output_selection::indices(
        &names,
        &launch.display_names,
        !launch.one_monitor,
        active,
    )?;
    let outputs: Vec<_> = indices
        .into_iter()
        .map(|index| all[index].clone())
        .collect();
    if launch.one_monitor {
        prof.mark("active output probed");
    }

    let shots = capture.capture_frame(&outputs).await?;
    drop(overlay);
    prof.mark("capture");

    let outputs: Vec<_> = shots
        .frames
        .iter()
        .map(|f| outputs[f.output].clone())
        .collect();
    let captures = init::build_captures(shots.frames)?;
    let placements = init::build_placements(&outputs, false);
    let region = get_full_workspace_rect(&placements).ok_or("no monitor was captured")?;
    let (pixmap, _, scale) = renderer::composite(&captures, &placements, region, None)
        .ok_or("couldn't put the monitors together")?;
    let png = encode_png(&pixmap);
    prof.mark("png encoded");

    deliver(png, scale, launch.outputs(), launch.output.as_deref(), "").await;
    prof.mark("delivered");
    prof.dump();
    Ok(())
}

pub async fn window(conn: crate::backend::session::Session, launch: Launch) -> Result {
    let mut prof = Profiler::new(launch.speed);
    if launch.one_monitor {
        log::info!("--monitor does nothing with --window, ignoring it");
    }

    let method = crate::backend::initialize_window_capture(
        &conn,
        launch.include_cursor,
        launch.outputs().copy,
    );
    delay::wait(launch.shot_at, &*method, &mut prof).await?;
    let capture = method.capture_active_window().await?;
    prof.mark("capture");
    let png = encode_png_with_alpha(capture.pixmap.into_pixmap());
    prof.mark("png encoded");

    deliver(
        png,
        capture.scale,
        launch.outputs(),
        launch.output.as_deref(),
        "",
    )
    .await;
    prof.mark("delivered");
    prof.dump();
    Ok(())
}
