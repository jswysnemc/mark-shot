//! 【界面控件】【布尔开关】设置窗口与录制窗口共用同一开关外观

use tiny_skia::Rect;

use crate::{renderer::Painter, theme::window::Palette};

/// 开关轨道的逻辑尺寸，命中范围由调用方决定
pub const SWITCH_SIZE: (f32, f32) = (38.0, 22.0);

/// 【界面控件】【布尔开关】用 painter 在 bounds 右侧垂直居中绘制开关，无返回值
///
/// 参数：painter 目标画布、bounds 所在行的控件区域、checked 当前取值、hovered 是否悬停或聚焦
pub fn switch(painter: &mut Painter<'_>, bounds: Rect, checked: bool, hovered: bool) {
    let palette = Palette::current();
    let Some(track) = Rect::from_xywh(
        bounds.right() - SWITCH_SIZE.0,
        bounds.top() + (bounds.height() - SWITCH_SIZE.1) / 2.0,
        SWITCH_SIZE.0,
        SWITCH_SIZE.1,
    ) else {
        return;
    };
    // 1. 【界面控件】【开关轨道】开启使用强调色，关闭使用中性底色，悬停只改变描边
    super::frame(
        painter,
        track,
        if checked {
            palette.accent
        } else {
            palette.alternate
        },
        if hovered || checked {
            palette.accent
        } else {
            palette.border
        },
        11.0,
    );
    // 2. 【界面控件】【开关滑块】滑块位置表达取值，颜色与轨道保持足够对比
    if let Some(knob) = Rect::from_xywh(
        track.left() + if checked { 19.0 } else { 3.0 },
        track.top() + 3.0,
        16.0,
        16.0,
    ) {
        painter.fill_rounded(
            knob,
            8.0,
            if checked {
                palette.on_accent
            } else {
                palette.base
            },
        );
    }
}
