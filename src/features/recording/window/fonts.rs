//! 【录制窗口】【共享字库】准备窗口与倒计时小组件复用同一字体系统，避免再次加载系统字库

use crate::ui::fonts::UiFonts;
use std::{cell::RefCell, rc::Rc};

pub(super) struct Fonts {
    pub system: UiFonts,
    pub cache: cosmic_text::SwashCache,
}

/// 字库的共享句柄，各窗口在同一线程内顺序使用
pub(super) type Shared = Rc<RefCell<Fonts>>;

/// 【录制窗口】【字库创建】无参数，返回只含界面字体家族的轻量字库，系统字库在首次出现非 ASCII 文字时加载一次
pub(super) fn shared() -> Shared {
    Rc::new(RefCell::new(Fonts {
        system: UiFonts::with_system_family("Sans Serif"),
        cache: cosmic_text::SwashCache::new(),
    }))
}
