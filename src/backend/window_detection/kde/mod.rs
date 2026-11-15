//! 【KDE窗口识别】【窗口快照】同一 KWin 进程通过一次性脚本交付可见窗口，不保留桌面监听

mod identity;
mod protocol;
#[cfg(test)]
mod tests;
mod windows;

use super::{Result, Window};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use std::{sync::Mutex, time::Duration};
use tempfile::TempDir;
use tokio::sync::oneshot;
use zbus::{Connection, names::OwnedUniqueName};

struct Script {
    connection: Connection,
    owner: OwnedUniqueName,
    name: String,
    directory: Option<TempDir>,
}

impl Script {
    /// 【KDE窗口识别】【资源释放】无参数，卸载本次脚本并释放私有文件，返回总线错误；失败时析构再次清理
    async fn close(&mut self) -> Result<()> {
        tokio::time::timeout(
            Duration::from_millis(100),
            unload(&self.connection, &self.owner, &self.name),
        )
        .await??;
        self.directory.take();
        Ok(())
    }
}

impl Drop for Script {
    /// 【KDE窗口识别】【取消清理】无参数，取消或超时后尽力卸载脚本，文件随清理任务结束释放
    fn drop(&mut self) {
        let Some(directory) = self.directory.take() else {
            return;
        };
        let connection = self.connection.clone();
        let owner = self.owner.clone();
        let name = self.name.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _directory = directory;
                match tokio::time::timeout(
                    Duration::from_millis(100),
                    unload(&connection, &owner, &name),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        log::info!("【KDE窗口识别】【取消清理】脚本卸载失败: {error}")
                    }
                    Err(error) => log::info!("【KDE窗口识别】【取消清理】脚本卸载超时: {error}"),
                }
            });
        }
    }
}

/// 【KDE窗口识别】【脚本移除】按 connection、owner 和 name 卸载唯一脚本，返回总线结果，不操作其他脚本
async fn unload(connection: &Connection, owner: &OwnedUniqueName, name: &str) -> zbus::Result<()> {
    protocol::ScriptingProxy::builder(connection)
        .destination(owner.clone())?
        .build()
        .await?
        .unload_script(name)
        .await?;
    Ok(())
}

/// 【KDE窗口识别】【桌面读取】按 wayland 验证总线与显示属于同一进程，返回有界单次快照；所有路径释放临时脚本
pub(super) async fn snapshot(wayland: &wayland_client::Connection) -> Result<Vec<Window>> {
    let connection = Connection::session().await?;
    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
    let owner = bus.get_name_owner("org.kde.KWin".try_into()?).await?;
    let peer = getsockopt(&wayland.backend().poll_fd(), PeerCredentials)?;
    let (pid, uid) = tokio::try_join!(
        bus.get_connection_unix_process_id(owner.as_ref().into()),
        bus.get_connection_unix_user(owner.as_ref().into())
    )?;
    if !identity::matches(pid, peer.pid(), uid, peer.uid())? {
        return Err(
            "KWin and the Wayland connection belong to different compositor processes".into(),
        );
    }
    let (sender, reply) = oneshot::channel();
    connection
        .object_server()
        .at(
            protocol::PATH,
            protocol::Reply {
                owner: owner.to_string(),
                sender: Mutex::new(Some(sender)),
            },
        )
        .await?;
    let directory = tempfile::Builder::new()
        .prefix("mark-shot-window-")
        .tempdir()?;
    let name = directory
        .path()
        .file_name()
        .ok_or("window snapshot directory has no name")?
        .to_string_lossy()
        .into_owned();
    let file = directory.path().join("snapshot.js");
    let destination = serde_json::to_string(
        connection
            .unique_name()
            .ok_or("window snapshot has no bus identity")?
            .as_str(),
    )?;
    std::fs::write(
        &file,
        include_str!("snapshot.js").replace("__DESTINATION__", &destination),
    )?;
    // 1. 【KDE窗口识别】【加载前守卫】载入请求取消时也按唯一名称卸载，避免迟到响应留下长期脚本
    let mut script = Script {
        connection: connection.clone(),
        owner: owner.clone(),
        name,
        directory: Some(directory),
    };
    let result = tokio::time::timeout(Duration::from_millis(300), async {
        let proxy = protocol::ScriptingProxy::builder(&connection)
            .destination(owner.clone())?
            .build()
            .await?;
        let id = proxy
            .load_script(
                file.to_str().ok_or("window snapshot path is not UTF-8")?,
                &script.name,
            )
            .await?;
        if id < 0 {
            return Err("KWin refused the window snapshot script".into());
        }
        connection
            .call_method(
                Some(owner.as_ref()),
                format!("/Scripting/Script{id}"),
                Some("org.kde.kwin.Script"),
                "run",
                &(),
            )
            .await?;
        windows::decode(&reply.await?)
    })
    .await;
    // 2. 【KDE窗口识别】【读取结束】正常完成及请求超时都先释放脚本，再返回检测数据
    script.close().await?;
    result?
}
