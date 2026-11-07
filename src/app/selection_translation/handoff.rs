//! 【框选翻译】【转贴图】原图暂时恢复后导出标注，翻译数据按新裁切原点和比例交接

use crate::{
    editor::EditorState,
    features::image_translation::{BoxRect, seed::Seed},
};

pub struct Image {
    pub png: Vec<u8>,
    pub seed: Seed,
}

/// 【框选翻译】【交接准备】从 state 返回带标注原图及按导出坐标换算的 seed，随后恢复当前译文显示
pub(in crate::app) fn prepare(state: &mut EditorState) -> Option<Image> {
    let session = state.selection_translation.session.as_ref()?;
    session
        .region
        .apply(&mut state.captures, state.file_canvas.as_mut(), None);
    let rendered = super::super::export::render_final_precise(state);
    let session = state.selection_translation.session.as_ref()?;
    let image = session
        .current()
        .filter(|_| session.showing_translation)
        .map(|t| &t.pixels);
    session
        .region
        .apply(&mut state.captures, state.file_canvas.as_mut(), image);
    let (png, origin, scale) = rendered?;
    let pixels = tiny_skia::Pixmap::decode_png(&png).ok()?;
    let mut seed = Seed {
        source_tokens: session.tokens.clone(),
        blocks: session
            .current()
            .map(|t| t.blocks.clone())
            .unwrap_or_default(),
        show_translation: session.showing_translation && session.current().is_some(),
    };
    let origin = if let Some(file) = &state.file_canvas {
        file.source_region(state.selection.zone.unwrap_or(file.viewport))?
            .global_origin
    } else {
        (origin.0 as f32, origin.1 as f32)
    };
    let factor = f64::from(scale / session.region.scale);
    let offset = (
        f64::from((session.region.origin.0 - origin.0) * scale),
        f64::from((session.region.origin.1 - origin.1) * scale),
    );
    // 1. 【框选翻译】【导出坐标】选区变化或跨输出比例变化时同步缩放底板、行框、字号及基线
    for token in &mut seed.source_tokens {
        token.rect = scaled(token.rect, factor);
    }
    for block in &mut seed.blocks {
        block.source = scaled(block.source, factor);
        block.plate = scaled(block.plate, factor);
        block.font_size *= factor as f32;
        for line in &mut block.lines {
            line.rect = scaled(line.rect, factor);
            line.baseline *= factor;
        }
    }
    seed = seed.shifted(
        offset,
        BoxRect::new(
            0.0,
            0.0,
            f64::from(pixels.width()),
            f64::from(pixels.height()),
        ),
    );
    seed.show_translation &= !seed.blocks.is_empty();
    Some(Image { png, seed })
}

/// 【框选翻译】【矩形换算】按 rect 和 factor 返回等比例矩形，不改变相对区域原点
fn scaled(rect: BoxRect, factor: f64) -> BoxRect {
    BoxRect::new(
        rect.left() * factor,
        rect.top() * factor,
        rect.width() * factor,
        rect.height() * factor,
    )
}
