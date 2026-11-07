//! 【Windows资源】【原生入口】显示器与位图资源独立于编辑、配置和渲染业务

#[path = "native/bitmap.rs"]
mod bitmap;
#[path = "native/capture.rs"]
mod capture;
#[path = "native/events.rs"]
mod events;
#[path = "native/ime.rs"]
mod ime;
#[path = "native/message_loop.rs"]
mod message_loop;
#[path = "native/monitors.rs"]
mod monitors;
#[path = "native/wake.rs"]
mod wake;
#[path = "native/window.rs"]
mod window;
#[path = "native/window_state.rs"]
mod window_state;

pub use bitmap::Bitmap;
pub use capture::capture;
pub use events::{Event, EventQueue, Modifiers};
pub use message_loop::{Wait, dispatch};
pub use monitors::{Bounds, Monitor, enable_dpi, monitors};
pub use wake::Wake;
pub use window::{Cursor, Options as WindowOptions, Window};

/// 【Windows资源】【系统错误】无参数，返回最近系统错误；系统未提供错误码时返回明确的资源操作错误
fn last_error() -> std::io::Error {
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(0) {
        std::io::Error::other("Windows resource operation failed")
    } else {
        error
    }
}
