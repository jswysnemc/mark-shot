// Everything related to -r ocr:
// dragging with no toolbar, immediate closure after choosing a region,
// with the result being scanned in the background,
// after which the user gets a notification about the outcome, successful or not
//
// maybe in the future it's worth implementing an option to make a very small
// toolbar appear in -r ocr, with the ability to choose an ocr model,
// but for now the user has to go to the editor to choose/download something specifically

use crate::backend::notify::{self, Notice};
use crate::editor::EditorState;
use crate::ocr::{self, StartOutcome};
use crate::theme::anim;
use crate::types::Outputs;
use crate::utils::{copy_to_clipboard, copy_to_primary};
use log::{error, info};

use super::finish::to_stdout;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

pub async fn finish(editor_state: &mut EditorState) -> Result {
    let Some(outputs) = editor_state.finish else {
        return Ok(());
    };
    let Some(region) = editor_state.selection.zone else {
        return Ok(());
    };
    let Some(image) =
        ocr::composite_region(&editor_state.captures, &editor_state.placements, region)
    else {
        return Ok(());
    };

    let runtime = &mut editor_state.ocr.runtime;
    if let StartOutcome::Unavailable(e) = runtime.start(image) {
        error!("ocr: engine unavailable: {e}");
        notify::send(Notice::TextFailed(e)).await;
        return Ok(());
    }
    let result = loop {
        if let Some(result) = runtime.poll() {
            break result;
        }
        tokio::time::sleep(anim::frame()).await;
    };

    let text = match result {
        Ok(text) => text,
        Err(e) => {
            error!("ocr: recognition failed: {e}");
            notify::send(Notice::TextFailed(e)).await;
            return Ok(());
        }
    };
    if text.is_empty() {
        info!("ocr: no text found in the region");
        notify::send(Notice::NoText).await;
        return Ok(());
    }

    let view = &mut editor_state.ocr.view;
    view.set_region(region);
    view.set_lines(text.lines);
    info!("ocr: {} line(s)", view.lines.len());
    deliver(view.text_to_copy(), outputs).await;
    if crate::features::ocr_result::enabled() {
        crate::tools::ocr::open_result(editor_state);
    }
    Ok(())
}

async fn deliver(text: String, outputs: Outputs) {
    if outputs.stdout {
        to_stdout(format!("{text}\n").as_bytes(), false).await;
    }
    let in_clipboard = outputs.copy && copied(copy_to_clipboard(&text)).await;
    let in_primary = outputs.primary && copied(copy_to_primary(&text)).await;
    if in_clipboard {
        notify::send(Notice::TextCopied(text)).await;
    } else if in_primary {
        notify::send(Notice::TextInPrimary(text)).await;
    }
}

async fn copied(result: Result) -> bool {
    if let Err(e) = &result {
        error!("ocr: copy failed: {e}");
        notify::send(Notice::TextCopyFailed(e.to_string())).await;
    }
    result.is_ok()
}
