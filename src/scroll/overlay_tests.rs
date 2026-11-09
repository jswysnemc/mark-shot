// Wayland overlay regression test: panel stays visible and mapped during live frame capture.
// Run with: cargo test --bin mark-shot overlay_scroll -- --ignored --nocapture

use crate::backend::ScreenOverlay;

fn presented_overlay() -> (
    crate::backend::session::Session,
    Box<dyn ScreenOverlay>,
    Vec<crate::types::Output>,
) {
    crate::config::init(None, |_| {});
    let conn = crate::backend::session::Session::Wayland(
        wayland_client::Connection::connect_to_env().unwrap(),
    );
    let mut overlay = crate::backend::initialize_overlay(conn.clone()).unwrap();
    let outputs = overlay.discovered_outputs().to_vec();
    let targets: Vec<usize> = (0..outputs.len()).collect();
    overlay.present(&targets).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(80));
    (conn, overlay, outputs)
}

#[test]
#[ignore = "needs a live Wayland session"]
fn overlay_keeps_a_visible_panel_while_capturing() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let (conn, mut overlay, outputs) = presented_overlay();
        let capture = crate::backend::initialize_capture(&conn);
        let (w, h) = outputs[0].info.logical_size.unwrap_or((64, 64));
        let canvas = vec![0u8; w.max(1) as usize * h.max(1) as usize * 4];

        // drop static background so real desktop shows through
        overlay.clear_background(0).unwrap();

        for round in 0..3 {
            overlay
                .stage_frame(0, &canvas, None)
                .unwrap_or_else(|e| panic!("round {round}: stage failed: {e}"));
            overlay.set_input_rect(0, Some((10, 10, 40, 30)));

            let shot = capture.capture_frame(&outputs[..1]).await;
            assert!(shot.is_ok(), "round {round}: capture failed");

            overlay
                .stage_frame(0, &canvas, None)
                .unwrap_or_else(|e| panic!("round {round}: second stage failed: {e}"));
            std::thread::sleep(std::time::Duration::from_millis(24));
        }
    });
}
