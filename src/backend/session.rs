//! 【平台会话】【连接入口】业务不直接创建Wayland连接或持有Windows窗口

#[cfg(target_os = "linux")]
#[derive(Clone)]
pub enum Session {
    Wayland(wayland_client::Connection),
    X11(super::x11::connection::Display),
}

#[cfg(target_os = "linux")]
impl Session {
    /// 【Linux会话】【环境连接】无参数，按显式选择和会话变量连接真实显示服务，返回连接错误
    pub fn connect_to_env() -> crate::AppResult<Self> {
        let backend = select(
            &std::env::var("MARK_SHOT_BACKEND").unwrap_or_default(),
            &std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
            std::env::var_os("WAYLAND_DISPLAY").is_some(),
            std::env::var_os("WAYLAND_SOCKET").is_some(),
            std::env::var_os("DISPLAY").is_some(),
        )?;
        match backend {
            "x11" => Ok(Self::X11(super::x11::connection::Display::connect()?)),
            _ => Ok(Self::Wayland(wayland_client::Connection::connect_to_env()?)),
        }
    }
}

/// 【Linux会话】【后端选择】按 forced、session 和显示变量判断协议，返回后端名称，拒绝未知强制值
#[cfg(target_os = "linux")]
fn select(
    forced: &str,
    session: &str,
    wayland: bool,
    socket: bool,
    x11: bool,
) -> crate::AppResult<&'static str> {
    match forced {
        "x11" => return Ok("x11"),
        "wayland" => return Ok("wayland"),
        "" | "auto" => {}
        _ => return Err("MARK_SHOT_BACKEND must be auto, x11 or wayland".into()),
    }
    if socket || wayland && session != "x11" {
        Ok("wayland")
    } else if x11 && session != "wayland" {
        Ok("x11")
    } else {
        Err("no display for the current session; set DISPLAY for X11 or WAYLAND_DISPLAY for Wayland".into())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::select;

    /// 【Linux会话测试】【协议选择】无参数，验证混合变量、嵌套会话、显式选择及无显示环境
    #[test]
    fn selects_the_session_without_silently_using_xwayland() {
        assert_eq!(select("", "wayland", true, false, true).unwrap(), "wayland");
        assert_eq!(select("", "x11", true, false, true).unwrap(), "x11");
        assert_eq!(select("", "x11", false, true, true).unwrap(), "wayland");
        assert_eq!(select("", "", false, false, true).unwrap(), "x11");
        assert_eq!(select("x11", "wayland", true, false, true).unwrap(), "x11");
        assert!(select("", "wayland", false, false, true).is_err());
        assert!(select("invalid", "", false, false, true).is_err());
        assert!(select("", "", false, false, false).is_err());
    }
}

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
