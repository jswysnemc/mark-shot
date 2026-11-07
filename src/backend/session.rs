//! 【平台会话】【连接入口】业务不直接创建Wayland连接或持有Windows窗口

#[cfg(target_os = "linux")]
pub type Session = wayland_client::Connection;

#[cfg(target_os = "windows")]
#[derive(Clone, Debug)]
pub struct Session;

#[cfg(target_os = "windows")]
impl Session {
    /// 【Windows会话】【环境连接】无参数，先登记DPI上下文，返回本进程平台会话，不创建表面
    pub fn connect_to_env() -> std::io::Result<Self> {
        super::windows::native::enable_dpi()?;
        Ok(Self)
    }
}
