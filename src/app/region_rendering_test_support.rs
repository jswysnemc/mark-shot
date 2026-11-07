//! 【区域绘制测试】【场景辅助】通过实际编辑器帧入口比较区域模式和旧缓存模式

use super::frame_render::{Frame, rebuild_layer, render_monitor};
use crate::{editor::EditorState, types::Annotation};
use tiny_skia::{Color, Pixmap, Rect};

/// 【区域绘制测试】【环境构造】按scale与region返回同源编辑器，背景覆盖确定纹理，不连接显示服务
pub(crate) fn editor(scale: f32, region: bool) -> EditorState {
    editor_size((640, 480), scale, region)
}

/// 【区域绘制测试】【尺寸环境】按size、scale与region返回固定纹理编辑器，不连接显示服务
pub(crate) fn editor_size(size: (i32, i32), scale: f32, region: bool) -> EditorState {
    let mut state = super::input::tests::editor();
    state.placements[0].scale = scale;
    state.placements[0].size = size;
    let mut image = Pixmap::new(
        (size.0 as f32 * scale).round() as u32,
        (size.1 as f32 * scale).round() as u32,
    )
    .unwrap();
    let width = image.width() as usize;
    for (index, pixel) in image.pixels_mut().iter_mut().enumerate() {
        let (x, y) = (index % width, index / width);
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(
            ((x * 31 + y * 7) % 256) as u8,
            ((x * 3 + y * 41) % 256) as u8,
            ((x * 11 + y * 13) % 256) as u8,
            255,
        )
        .unwrap();
    }
    state.captures[0].pixmap = image.clone().into();
    state.captures[0].scale = scale;
    state.canvas[0] = image.clone();
    state.dimmed[0] = image.clone();
    crate::renderer::init_dimming(&mut state.dimmed[0], None, None, scale);
    state.annotations_layer[0] = Pixmap::new(image.width(), image.height()).unwrap();
    state.toolbar.opacity = 0.0;
    state.action_toolbar.opacity = 0.0;
    state.settings_panel.visible = false;
    if region {
        state.enable_region_rendering();
    }
    state
}

/// 【区域绘制测试】【实际帧】按state、dirty与hidden调用实际帧绘制，返回本帧像素，完整帧先重建旧缓存
pub(crate) fn render(state: &mut EditorState, dirty: Option<Rect>, hidden: bool) -> Pixmap {
    paint(state, dirty, hidden);
    state.canvas[0].clone()
}

/// 【区域绘制测试】【帧执行】按state、dirty与hidden调用实际帧入口，不复制输出像素，无返回值
pub(crate) fn paint(state: &mut EditorState, dirty: Option<Rect>, hidden: bool) {
    let frame = Frame {
        active_text_id: super::frame_render::active_text_id(state),
        scan_badge: None,
        current_color: Color::BLACK,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: hidden,
    };
    if dirty.is_none() {
        state.annotations_dirty = true;
        rebuild_layer(state, 0, &frame);
    }
    paint_current(state, dirty, hidden);
}

/// 【区域绘制测试】【实时帧】按state、dirty与hidden绘制当前图层，不重建正在缓存的画笔片段，无返回值
pub(crate) fn paint_current(state: &mut EditorState, dirty: Option<Rect>, hidden: bool) {
    let frame = Frame {
        active_text_id: super::frame_render::active_text_id(state),
        scan_badge: None,
        current_color: Color::BLACK,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: hidden,
    };
    let mut dimmed_for = state.selection.prev_zone;
    render_monitor(state, 0, &frame, dirty, &mut dimmed_for);
    state.selection.prev_zone = state.selection.zone;
}

/// 【区域绘制测试】【指针输入】按state、position、button及pressed分派真实指针与按钮，dirty返回重绘掩码，无返回值
pub(crate) fn pointer(
    state: &mut EditorState,
    position: (f64, f64),
    button: Option<(crate::types::MouseButton, bool)>,
    dirty: &mut u32,
) {
    super::input::handle_pointer_move(state, 0, position.0, position.1, dirty);
    if let Some((button, pressed)) = button {
        super::input::handle_pointer_button(state, button, pressed, dirty);
    }
}

/// 【区域绘制测试】【按键输入】按state与key分派真实按键，dirty返回重绘掩码，无返回值
pub(crate) fn key(state: &mut EditorState, key: &str, dirty: &mut u32) {
    super::input::handle_key(state, Some(crate::keys::parse(key).unwrap()), None, dirty);
}

/// 【区域绘制测试】【像素断言】比较actual和expected全部字节，label标明场景，差异时报告首个坐标，无返回值
pub(crate) fn equal(actual: &Pixmap, expected: &Pixmap, label: &str) {
    if actual.data() == expected.data() {
        return;
    }
    let width = actual.width() as usize;
    let differences: Vec<_> = actual
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.data().as_chunks::<4>().0)
        .enumerate()
        .filter(|(_, (actual, expected))| actual != expected)
        .map(|(index, (actual, expected))| (index % width, index / width, *actual, *expected))
        .take(12)
        .collect();
    panic!(
        "{label}: width={} differences={differences:?}",
        actual.width()
    );
}

/// 【区域绘制测试】【半透明对象】按id、rotation与shape返回统一透明样式的标注，自动更新边界
pub(crate) fn annotation(
    id: u64,
    rotation: f64,
    shape: crate::types::AnnotationShape,
) -> Annotation {
    let color = Color::from_rgba8(210, 90, 40, 137);
    let mut annotation = Annotation {
        id,
        style_scale: 1.0,
        rotation_degrees: rotation,
        frozen_bounds: None,
        color,
        shadow_color: crate::renderer::shadow_color_for(color),
        stroke_width: 7.25,
        shape,
        bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}
