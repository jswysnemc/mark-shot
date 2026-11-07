//! 【niri】【共用连接】窗口捕获与窗口识别共享有界传输和合成器身份认证

pub(crate) mod ipc;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Result<T> = std::result::Result<T, Error>;
