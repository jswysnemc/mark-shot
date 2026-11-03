// Scrolling capture event loop: periodically grabs frames, stitches them,
// renders the preview panel and handles user interaction.
//
// The overlay clears its background snapshot so the live page shows through,
// while its input region is shrunk down to just the panel. Scrolling passes
// straight to the window underneath.

use log::info;
use std::time::{Duration, Instant};

use crate::backend::CaptureMethod;
use crate::backend::ScreenOverlay;
use crate::editor::EditorState;
use crate::renderer::Painter;
use crate::scroll::frame;
use crate::scroll::panel::{self, PanelButton, PanelLayout};
use crate::scroll::session::{ScrollSession, to_pixmap};
use crate::types::{OverlayEvent, PointerState};
use tiny_skia::Rect;
use wayland_client::Connection;

/// Minimal interval between panel repaints to avoid excessive commits.
const REPAINT_INTERVAL: Duration = Duration::from_millis(60);

struct ScrollRun {
    session: ScrollSession,
    layout: PanelLayout,
    hovered: Option<PanelButton>,
}

/// Runs scrolling capture until accepted or cancelled, returning the final stitched PNG.
pub async fn run(
    conn: &Connection,
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let Some(zone) = editor_state.selection.zone else {
        return Err("scrolling capture needs a selected region".into());
    };
    let monitor_idx = editor_state.input.pointer.monitor_idx;
    let placement = &editor_state.placements[monitor_idx];

    // convert selection to monitor local coordinates
    let Some(local_region) = crate::tools::selection::global_selection_to_local(&zone, placement)
    else {
        return Err("the selected region is outside the monitor".into());
    };
    let session = ScrollSession::new(local_region, monitor_idx)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let mut run = ScrollRun {
        layout: panel::layout(
            local_region,
            (placement.size.0 as f32, placement.size.1 as f32),
            None,
        ),
        session,
        hovered: None,
    };

    // drop the static snapshot so the live page shows through;
    // confine overlay input strictly to the panel rectangle
    overlay.clear_background(monitor_idx)?;
    overlay.set_input_rect(monitor_idx, panel_input_rect(&run));
    paint(editor_state, &mut run, overlay.as_mut())?;

    let capture = crate::backend::initialize_capture(conn);
    capture.prepare().await?;

    let mut last_paint = Instant::now();
    let mut idle_logged = false;

    loop {
        let now = Instant::now();
        if run.session.wants_capture(now) {
            if let Err(e) = capture_once(&mut run, overlay.as_mut(), &*capture).await {
                log::error!("capture failed: {e}");
                run.session.status = "Capture error".into();
                run.session.paused = true;
            }
            paint(editor_state, &mut run, overlay.as_mut())?;
            last_paint = Instant::now();
        }

        // back off capture interval when idle, but keep polling since
        // scroll events go directly to the client window below
        if run.session.should_idle(now) {
            if !idle_logged {
                idle_logged = true;
                info!("scrolling capture idle, backing off capture interval");
            }
        } else {
            idle_logged = false;
        }

        // refresh panel on a timer to keep hover transitions responsive
        if last_paint.elapsed() >= REPAINT_INTERVAL {
            paint(editor_state, &mut run, overlay.as_mut())?;
            last_paint = Instant::now();
        }

        // wake up around the next expected frame capture
        let deadline = Instant::now() + run.session.capture_interval();
        let event = loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match overlay.next_event(Some(left))? {
                OverlayEvent::FrameDone { .. } => {}
                event => break event,
            }
        };
        match event {
            OverlayEvent::EscapePressed => return Ok(None),
            OverlayEvent::Key {
                chord: Some(chord), ..
            } if chord.mods == crate::keys::Mods::default() => match chord.key {
                crate::keys::Key::Enter => return finish(&mut run),
                crate::keys::Key::Escape => return Ok(None),
                _ => {}
            },
            OverlayEvent::PointerMove { monitor_idx, x, y } => {
                editor_state.input.pointer = PointerState::new(monitor_idx, (x, y), (x, y));
                let hovered = panel::hit_test(&run.layout, (x, y));
                if hovered != run.hovered {
                    run.hovered = hovered;
                    paint(editor_state, &mut run, overlay.as_mut())?;
                    last_paint = Instant::now();
                }
            }
            OverlayEvent::PointerButton { pressed: true, .. } => {
                let Some(button) = run.hovered else { continue };
                if !panel::is_button_enabled(button, &run.session) {
                    continue;
                }
                match button {
                    PanelButton::TogglePause => {
                        run.session.paused = !run.session.paused;
                        run.session.status = if run.session.paused {
                            "Paused".into()
                        } else {
                            "Capturing".into()
                        };
                    }
                    PanelButton::ToggleAxis => {
                        run.session.toggle_axis();
                    }
                    PanelButton::Finish => return finish(&mut run),
                    PanelButton::Cancel => return Ok(None),
                }
                paint(editor_state, &mut run, overlay.as_mut())?;
                last_paint = Instant::now();
            }
            _ => {}
        }
    }
}

/// Captures a single screen frame and pushes the cropped selection into the stitcher.
async fn capture_once(
    run: &mut ScrollRun,
    overlay: &mut dyn ScreenOverlay,
    capture: &dyn CaptureMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    let monitor_idx = run.session.monitor_idx;
    let outputs = overlay.discovered_outputs().to_vec();
    let Some(output) = outputs.get(monitor_idx).cloned() else {
        return Err("the monitor used for scrolling is gone".into());
    };

    let result = capture.capture_frame(&[output]).await?;
    let Some(frame_data) = result.frames.first() else {
        return Ok(());
    };

    // scale selection coordinates to backing pixel rectangle
    let scale = frame_scale(frame_data);
    let image = frame::crop(frame_data, pixel_rect(run.session.region, scale))?;
    run.session.push(image);
    Ok(())
}

fn pixel_rect(region: Rect, scale: f32) -> (u32, u32, u32, u32) {
    let x = (region.left() * scale).round().max(0.0) as u32;
    let y = (region.top() * scale).round().max(0.0) as u32;
    let w = (region.width() * scale).round().max(1.0) as u32;
    let h = (region.height() * scale).round().max(1.0) as u32;
    (x, y, w, h)
}

fn frame_scale(frame_data: &crate::types::MonitorFrame) -> f32 {
    match frame_data.info.size {
        Some((logical_w, _)) if logical_w > 0 => frame_data.pw_width as f32 / logical_w as f32,
        _ => 1.0,
    }
}

/// Clears canvas and renders the floating control panel.
fn paint(
    editor_state: &mut EditorState,
    run: &mut ScrollRun,
    overlay: &mut dyn ScreenOverlay,
) -> Result<(), Box<dyn std::error::Error>> {
    let idx = run.session.monitor_idx;
    let Some(canvas) = editor_state.canvas.get_mut(idx) else {
        return Ok(());
    };

    // transparent canvas leaves the live window visible everywhere except the panel
    canvas.data_mut().fill(0);

    let preview_size = run.session.preview().map(|p| (p.width(), p.height()));
    let placement = &editor_state.placements[idx];
    run.layout = panel::layout(
        run.session.region,
        (placement.size.0 as f32, placement.size.1 as f32),
        preview_size,
    );

    panel::draw(
        &mut Painter::new(canvas)
            .with_scale(placement.scale)
            .with_icons(&editor_state.icons_cache)
            .with_text(
                &mut editor_state.text.font_system,
                &mut editor_state.text.swash_cache,
            ),
        &mut run.session,
        &run.layout,
        run.hovered,
    );

    overlay.stage_frame(idx, canvas.data(), None)?;
    overlay.flush()?;

    // keep input region in sync with panel position
    overlay.set_input_rect(idx, panel_input_rect(run));
    Ok(())
}

fn panel_input_rect(run: &ScrollRun) -> Option<(i32, i32, i32, i32)> {
    let rect = run.layout.rect;
    Some((
        rect.left() as i32,
        rect.top() as i32,
        rect.width() as i32,
        rect.height() as i32,
    ))
}

fn finish(run: &mut ScrollRun) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let Some(image) = run.session.result() else {
        return Ok(None);
    };
    let Some(pixmap) = to_pixmap(&image) else {
        return Ok(None);
    };
    info!(
        "scrolling capture finished: {}x{}, {} frames",
        pixmap.width(),
        pixmap.height(),
        run.session.frame_count
    );
    Ok(Some(crate::utils::encode_png(&pixmap)))
}

pub async fn capture(
    conn: &Connection,
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let png = run(conn, editor_state, overlay).await?;
    editor_state.mode = crate::types::Mode::Editor;
    Ok(png)
}
