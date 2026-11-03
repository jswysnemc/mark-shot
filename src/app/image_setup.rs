use tiny_skia::{Color, Pixmap, PixmapPaint, Rect, Transform};

use super::{Launch, Profiler, init};
use crate::AppResult;
use crate::backend::{ScreenOverlay, initialize_overlay, output_selection};
use crate::editor::{EditorState, Layers, TextState, file_canvas::FileCanvas};
use crate::types::Capture;

/// 【图片编辑】【启动】在单个原生覆盖层中打开本地图片；原始图像独立保存，避免导出降采样
pub(super) fn start(
    connection: wayland_client::Connection,
    launch: &Launch,
    profiler: &mut Profiler,
) -> AppResult<(EditorState, Box<dyn ScreenOverlay>)> {
    let path = launch
        .image
        .as_deref()
        .ok_or("no image file was supplied")?;
    let mut overlay = initialize_overlay(connection)?;
    let outputs = overlay.discovered_outputs().to_vec();
    let names: Vec<_> = outputs
        .iter()
        .enumerate()
        .map(|(index, output)| output_selection::name(output, index))
        .collect();
    let chosen = output_selection::indices(
        &names,
        &launch.display_names,
        false,
        overlay.active_output(),
    )?;
    if chosen.len() != 1 {
        return Err("image editing uses one display at a time".into());
    }
    let index = chosen[0];
    let selected = vec![outputs[index].clone()];
    let placements = init::build_placements(&selected, overlay.hidpi());
    let placement = &placements[0];
    let bounds = Rect::from_xywh(
        placement.position.0 as f32,
        placement.position.1 as f32,
        placement.size.0 as f32,
        placement.size.1 as f32,
    )
    .ok_or("invalid display bounds")?;

    // 1. 原图与预览分开，低分辨率显示器不会降低保存后的图像分辨率
    let file = FileCanvas::load(path, bounds)?;
    let (width, height) = placement.px_size();
    let mut preview = Pixmap::new(width, height).ok_or("cannot allocate image preview")?;
    preview.fill(Color::from_rgba8(28, 31, 34, 255));
    let factor = file.viewport.width() / file.original.width() as f32 * placement.scale;
    let transform = Transform::from_scale(factor, factor).post_translate(
        (file.viewport.left() - bounds.left()) * placement.scale,
        (file.viewport.top() - bounds.top()) * placement.scale,
    );
    preview.draw_pixmap(
        0,
        0,
        file.original.as_ref(),
        &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        transform,
        None,
    );
    let (canvas, dimmed, annotations) = init::build_layers(&placements);
    let image_region = file.viewport;
    let mut editor = EditorState::new(
        Layers {
            captures: vec![Capture {
                pixmap: preview,
                scale: placement.scale,
            }],
            canvas,
            dimmed,
            annotations,
        },
        placements,
        init::load_icons_cache(),
        TextState {
            font_system: crate::fonts::bundled(),
            system_fonts: Some(std::thread::spawn(crate::fonts::system)),
            swash_cache: cosmic_text::SwashCache::new(),
            editors: std::collections::HashMap::new(),
            editing: None,
        },
        super::capture_setup::build_ocr_state(),
    );
    editor.file_canvas = Some(file);
    editor.selection.zone = Some(image_region);

    // 2. 仅映射目标显示器，背景仍使用共享内存表面，输入不影响其他显示器
    overlay.present(&[index])?;
    let preview = &editor.captures[0].pixmap;
    overlay.set_background(0, preview.data(), preview.width(), preview.height())?;
    profiler.mark("image loaded + preview mapped");
    Ok((editor, overlay))
}
