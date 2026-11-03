//! 【截图另存为】【会话交接】隐藏截图表面，取消或失败后恢复同一冻结图像与编辑状态

use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::EditorState,
    features::save_as::{self, Outcome, Snapshot},
    types::AnnotationShape,
    ui::toast::ToastKind,
};

/// 【截图另存为】【错误提示】在 state 中显示 message，保留截图和标注；无返回值
fn error(state: &mut EditorState, message: String) {
    log::error!("【截图】【另存为】{message}");
    state.toasts.show_text(
        ToastKind::SaveAsFailed,
        message,
        &mut state.text.font_system,
    );
}

/// 【截图另存为】【原图准备】合成 state 的最终 PNG 和可选文字索引；返回冻结快照，不修改完成状态
fn snapshot(state: &mut EditorState) -> AppResult<Snapshot> {
    let (png, _, scale) = super::render_final(state).ok_or("cannot render screenshot")?;
    let search = if crate::config::history::current().index_annotations {
        crate::features::capture_history::png_metadata::joined_search(
            state.annotations.iter().filter_map(|annotation| {
                if let AnnotationShape::Text { content, .. } = &annotation.shape {
                    Some(content.as_str())
                } else {
                    None
                }
            }),
        )
    } else {
        String::new()
    };
    Ok(Snapshot::new(png, scale, search)?)
}

/// 【截图另存为】【界面恢复】在 overlay 重建 state 的冻结背景；不重新捕获桌面，不改变标注或撤销记录
fn restore(state: &EditorState, overlay: &mut dyn ScreenOverlay) -> AppResult {
    overlay.resume()?;
    for (index, capture) in state.captures.iter().enumerate() {
        overlay.set_background(
            index,
            capture.pixmap.data(),
            capture.pixmap.width(),
            capture.pixmap.height(),
        )?;
    }
    Ok(())
}

/// 【截图另存为】【完整交接】为 state 打开独立文件对话框；overlay 先隐藏，返回已保存路径或恢复后的 None
pub(super) fn open(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
) -> AppResult<Option<std::path::PathBuf>> {
    let prepared = snapshot(state).and_then(|snapshot| {
        let path = crate::utils::screenshot_path::planned(state.output.as_deref())?;
        Ok((snapshot, path))
    });
    let (snapshot, initial_path) = match prepared {
        Ok(prepared) => prepared,
        Err(failure) => {
            error(state, format!("Cannot prepare screenshot: {failure}"));
            return Ok(None);
        }
    };
    // 1. 【截图另存为】【输入释放】独立窗口必须位于真实桌面，不能留在截图层和键盘抓取之后
    if let Some(focus) = state.input.text_input.focus {
        super::text_input::event(
            state,
            crate::types::OverlayEvent::TextInputLeave { focus_id: focus.id },
            &mut 0,
        );
    }
    overlay.set_text_input(None)?;
    overlay.suspend()?;
    log::info!("【截图】【另存为】打开文件对话框");
    let result = save_as::run(snapshot, &initial_path);
    if let Ok(Outcome::Saved(path)) = &result {
        log::info!("【截图】【另存为】已保存 {}", path.display());
        return Ok(Some(path.clone()));
    }
    // 2. 【截图另存为】【取消恢复】保存失败或取消都回到原编辑状态，不使用已经变化的桌面图像
    restore(state, overlay)?;
    match result {
        Ok(Outcome::Cancelled) => log::info!("【截图】【另存为】已取消，恢复原截图"),
        Ok(Outcome::Failed(failure)) => error(state, format!("Save failed: {failure}")),
        Err(failure) => error(state, format!("Cannot open save dialog: {failure}")),
        Ok(Outcome::Saved(_)) => unreachable!(),
    }
    Ok(None)
}
