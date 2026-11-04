//! 【原生弹层】【传输类型】后端只接收已排版像素，带会话编号的输入不会污染新菜单

use crate::{keys::Chord, types::MouseButton};

pub struct PopupFrame<'a> {
    pub id: u64,
    pub monitor_idx: usize,
    pub serial: Option<u32>,
    pub anchor: (f32, f32),
    pub size: (u32, u32),
    pub scale: f32,
    pub pixels: &'a [u8],
    pub pixel_size: (u32, u32),
}

#[derive(Clone, Debug)]
pub enum PopupEvent {
    Configured {
        position: (i32, i32),
    },
    Pointer {
        x: f64,
        y: f64,
    },
    Leave,
    Button {
        x: f64,
        y: f64,
        button: MouseButton,
        pressed: bool,
    },
    Key {
        chord: Option<Chord>,
        text: Option<String>,
    },
    Closed,
}

/// 【截图弹层】【传输校验】验证 frame 尺寸、比例与像素长度，返回合法状态；拒绝溢出或错误比例，避免协议错误
pub(crate) fn validate(frame: &PopupFrame<'_>) -> crate::AppResult {
    let expected = (
        (frame.size.0 as f32 * frame.scale).ceil() as u32,
        (frame.size.1 as f32 * frame.scale).ceil() as u32,
    );
    if frame.id == 0
        || frame.size.0 == 0
        || frame.size.1 == 0
        || frame.size.0 > 4096
        || frame.size.1 > 4096
        || !frame.scale.is_finite()
        || frame.scale <= 0.0
        || frame.scale > 16.0
        || frame.pixel_size != expected
        || u64::from(expected.0) * u64::from(expected.1) > 64 * 1024 * 1024
        || frame.pixels.len() as u64 != u64::from(expected.0) * u64::from(expected.1) * 4
    {
        return Err("invalid native popup frame".into());
    }
    Ok(())
}
