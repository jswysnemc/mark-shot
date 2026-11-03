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
