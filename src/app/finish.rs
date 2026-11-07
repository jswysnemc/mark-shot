// Shot delivery: pin, save, copy to clipboard, stdout, and scrolling capture finalisation.

use std::path::Path;

use crate::app::{render_final, scroll};
use crate::backend::notify::{self, Notice};
use crate::backend::{ScreenOverlay, initialize_clipboard};
use crate::editor::EditorState;
use crate::types::Outputs;
use crate::utils::{save_to_file, write_to_stdout};

/// 【滚动截图】【用户交付】按用户操作复制或保存，标注操作返回原图及输出名称供调用方创建编辑会话
pub async fn finish_scroll(
    conn: &crate::backend::session::Session,
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
) -> crate::AppResult<Option<(tiny_skia::Pixmap, String)>> {
    match scroll::capture(conn, editor_state, overlay).await? {
        scroll::Outcome::Cancelled => {}
        scroll::Outcome::Annotate { image, monitor_idx } => {
            let output = overlay
                .surface_output(monitor_idx)
                .ok_or("the scroll capture output is gone")?;
            let name = crate::backend::output_selection::name(output, monitor_idx);
            return Ok(Some((image, name)));
        }
        scroll::Outcome::Copy { png, scale } => {
            deliver(
                png,
                scale,
                Outputs {
                    copy: true,
                    ..Default::default()
                },
                None,
                "",
            )
            .await;
        }
        scroll::Outcome::Saved(path) => notify::send(Notice::Saved(path)).await,
    }
    Ok(None)
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
    let pin_translation = (outputs | editor_state.to)
        .pin
        .then(|| super::selection_translation::handoff::prepare(editor_state))
        .flatten();
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
        true,
        pin_translation,
    )
    .await;
    Ok(())
}

/// 【截图】【完成交付】记录最终 png 后按 outputs 贴图、保存与复制；scale 为设备比例，dir 为输出目录，search 为搜索文字
pub async fn deliver(png: Vec<u8>, scale: f32, outputs: Outputs, dir: Option<&Path>, search: &str) {
    deliver_positioned(png, scale, outputs, dir, search, None, false, None).await;
}

/// 【截图】【定位交付】按 position 恢复贴图位置，其余保存、复制与历史流程保持相同
async fn deliver_positioned(
    png: Vec<u8>,
    scale: f32,
    outputs: Outputs,
    dir: Option<&Path>,
    search: &str,
    position: Option<(i32, i32)>,
    user_image: bool,
    pin_translation: Option<super::selection_translation::handoff::Image>,
) {
    // 1. 【图像导出】【共享图片】只在用户保存或复制时合成一次外框，贴图与标准输出始终使用原图
    let exported = match crate::features::export_image::for_delivery(
        &png,
        user_image && (outputs.save || outputs.copy),
    ) {
        Ok(exported) => exported,
        Err(error) => {
            log::error!("【图像导出】【外框合成】{error}");
            let notice = if outputs.save {
                Notice::SaveFailed(error.to_string())
            } else {
                Notice::CopyFailed(error.to_string())
            };
            notify::send(notice).await;
            return;
        }
    };
    crate::features::capture_history::remember(&exported, scale, search);
    if outputs.pin {
        let result = if let Some(image) = pin_translation {
            crate::backend::pin::spawn_translated(&image.png, scale, position, &image.seed)
        } else {
            crate::backend::pin::spawn_at(&png, scale, position)
        };
        if let Err(e) = result {
            log::error!("pin failed: {e}");
            notify::send(Notice::PinFailed(e.to_string())).await;
        }
    }
    if outputs.stdout {
        to_stdout(&png, true).await;
    }
    let saved = if outputs.save {
        match save_to_file(&exported, dir) {
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
        match initialize_clipboard().copy_image_to_clipboard(&exported) {
            Ok(()) => {
                let preview = saved.is_none().then_some(exported.as_ref());
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
