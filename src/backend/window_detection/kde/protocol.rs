//! 【KDE窗口识别】【单次协议】对 KWin 脚本接口和回调来源进行明确约束

use std::sync::Mutex;
use tokio::sync::oneshot;
use zbus::{message::Header, proxy};

pub(super) const PATH: &str = "/io/github/jswysnemc/MarkShot/WindowSnapshot";

#[proxy(
    interface = "org.kde.kwin.Scripting",
    default_service = "org.kde.KWin",
    default_path = "/Scripting"
)]
pub(super) trait Scripting {
    /// 【KDE窗口识别】【脚本加载】按 file_path 和 plugin_name 载入本次脚本，返回 KWin 脚本标识
    #[zbus(name = "loadScript")]
    async fn load_script(&self, file_path: &str, plugin_name: &str) -> zbus::Result<i32>;
    /// 【KDE窗口识别】【脚本卸载】按 plugin_name 移除本次脚本，返回是否存在该脚本
    #[zbus(name = "unloadScript")]
    async fn unload_script(&self, plugin_name: &str) -> zbus::Result<bool>;
}

pub(super) struct Reply {
    pub owner: String,
    pub sender: Mutex<Option<oneshot::Sender<String>>>,
}

#[zbus::interface(name = "io.github.jswysnemc.MarkShot.WindowSnapshot")]
impl Reply {
    /// 【KDE窗口识别】【结果交付】按 reply 和 header 接受已校验 KWin 的唯一结果，返回拒绝原因或成功
    async fn submit(
        &self,
        reply: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        if header
            .sender()
            .is_none_or(|sender| sender.as_str() != self.owner)
        {
            return Err(zbus::fdo::Error::AccessDenied(
                "window snapshot sender is not KWin".into(),
            ));
        }
        if reply.len() > super::windows::MAX_REPLY {
            return Err(zbus::fdo::Error::InvalidArgs(
                "window snapshot is oversized".into(),
            ));
        }
        if let Some(sender) = self.sender.lock().unwrap().take() {
            let _ = sender.send(reply.to_owned());
        }
        Ok(())
    }
}
