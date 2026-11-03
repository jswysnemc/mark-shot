// Shot delivery: pin, save, copy to clipboard, stdout, and scrolling capture finalisation.

use std::path::Path;

use crate::app::{Launch, render_final, scroll};
use crate::backend::notify::{self, Notice};
use crate::backend::{ScreenOverlay, initialize_clipboard};
use crate::editor::EditorState;
use crate::types::Outputs;
use crate::utils::{save_to_file, write_to_stdout};

/// Runs the scrolling capture on the still-open overlay, then delivers
/// whatever it produced. An empty result means the user cancelled.
pub async fn finish_scroll(
    conn: &wayland_client::Connection,
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
    launch: &Launch,
) -> Result<(), Box<dyn std::error::Error>> {
    let png = scroll::capture(conn, editor_state, overlay).await?;
    if let Some(png) = png {
        deliver(
            png,
            1.0,
            launch.outputs(),
            editor_state.output.as_deref(),
            "",
        )
        .await;
    }
    Ok(())
}

pub async fn finish_capture(
    editor_state: &mut EditorState,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = editor_state.saved_as.take() {
        // 1. 【截图另存为】【完成通知】只通知已经选定的目标，不能再次保存默认目录或执行预设复制
        notify::send(Notice::Saved(path)).await;
        return Ok(());
    }
    let Some(outputs) = editor_state.finish else {
        return Ok(());
    };
    let Some((png, _, scale)) = render_final(editor_state) else {
        return Ok(());
    };
    let search = if crate::config::history::current().index_annotations {
        crate::features::capture_history::png_metadata::joined_search(
            editor_state
                .annotations
                .iter()
                .filter_map(|annotation| match &annotation.shape {
                    crate::types::AnnotationShape::Text { content, .. } => Some(content.as_str()),
                    _ => None,
                }),
        )
    } else {
        String::new()
    };
    deliver_positioned(
        png,
        scale,
        outputs | editor_state.to,
        editor_state.output.as_deref(),
        &search,
        editor_state
            .selection
            .zone
            .map(|rect| (rect.left().round() as i32, rect.top().round() as i32)),
    )
    .await;
    Ok(())
}

/// 【截图】【完成交付】记录最终 png 后按 outputs 贴图、保存与复制；scale 为设备比例，dir 为输出目录，search 为搜索文字
pub async fn deliver(png: Vec<u8>, scale: f32, outputs: Outputs, dir: Option<&Path>, search: &str) {
    deliver_positioned(png, scale, outputs, dir, search, None).await;
}

/// 【截图】【定位交付】按 position 恢复贴图位置，其余保存、复制与历史流程保持相同
async fn deliver_positioned(
    png: Vec<u8>,
    scale: f32,
    outputs: Outputs,
    dir: Option<&Path>,
    search: &str,
    position: Option<(i32, i32)>,
) {
    crate::features::capture_history::remember(&png, scale, search);
    if outputs.pin
        && let Err(e) = crate::backend::wayland::pin::spawn_at(&png, scale, position)
    {
        log::error!("pin failed: {e}");
        notify::send(Notice::PinFailed(e.to_string())).await;
    }
    if outputs.stdout {
        to_stdout(&png, true).await;
    }
    let saved = if outputs.save {
        match save_to_file(&png, dir) {
            Ok(path) => Some(path),
            Err(e) => {
                log::error!("save failed: {e}");
                notify::send(Notice::SaveFailed(e.to_string())).await;
                None
            }
        }
    } else {
        None
    };
    if outputs.copy {
        match initialize_clipboard().copy_image_to_clipboard(&png) {
            Ok(()) => {
                let preview = saved.is_none().then_some(png.as_slice());
                notify::send(Notice::Copied(saved, preview)).await
            }
            Err(e) => {
                log::error!("copy failed: {e}");
                notify::send(Notice::CopyFailed(e.to_string())).await
            }
        }
    } else if outputs.save
        && let Some(path) = saved
    {
        notify::send(Notice::Saved(path)).await;
    }
}

pub(crate) async fn to_stdout(data: &[u8], binary: bool) {
    if let Err(e) = write_to_stdout(data, binary) {
        log::error!("can't write to stdout: {e}");
        notify::send(Notice::StdoutFailed(e.to_string())).await;
    }
}
