//! 【截图历史】【完整原图】冻结原图独立于裁切、标注及导出外框保存

use crate::{editor::EditorState, image_source::PixelView};

/// 【截图历史】【原件编码】按 state 编码完整未标注原图，历史停用或合成失败返回空值
pub(super) fn png(state: &EditorState) -> Option<Vec<u8>> {
    if !crate::config::history::current().enabled {
        return None;
    }
    if let Some(file) = &state.file_canvas {
        return Some(crate::utils::encode_png_with_alpha(
            file.original.pixels_view().to_owned(),
        ));
    }
    let region = crate::utils::get_full_workspace_rect(&state.placements)?;
    let (image, _, _) =
        crate::renderer::composite(&state.captures, &state.placements, region, None)?;
    Some(crate::utils::encode_png(&image))
}
