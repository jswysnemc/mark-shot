//! 【niri】【兼容窗口截图】已授权复制时使用精确窗口动作，私有临时文件不进入用户截图目录

use serde_json::json;

use super::{
    Result, frame,
    target::{Target, ensure_same_window},
};
use crate::{backend::wayland::niri::ipc::Client, types::Capture};

/// 【niri】【截图交接】使用 client 与 target 捕获固定窗口，include_cursor 控制指针；返回完整图片并回收事件连接和临时目录
pub(super) async fn capture(
    client: &mut Client,
    target: &Target,
    include_cursor: bool,
) -> Result<Capture> {
    let directory = tempfile::Builder::new()
        .prefix("mark-shot-window-")
        .tempdir()?;
    let path = directory.path().join("window.png");
    let path_string = path.to_str().ok_or("niri screenshot path is not UTF-8")?;
    // 1. 【niri】【截图通知】先订阅事件，确保快速编码也不会错过唯一临时路径的完成通知
    let mut events = client.separate().await?;
    let _: () = events.query(&json!("EventStream"), "Handled").await?;
    // 2. 【niri】【固定目标】复核焦点后使用显式 ID，窗口消失时不能截图另一窗口
    ensure_same_window(&target.window, &client.focused().await?)?;
    let _: () = client
        .query(
            &json!({"Action": {"ScreenshotWindow": {
                "id": target.window.id,
                "write_to_disk": true,
                "show_pointer": include_cursor,
                "path": path_string,
            }}}),
            "Handled",
        )
        .await?;
    loop {
        let event = events.event().await?;
        if event
            .get("WindowClosed")
            .and_then(|event| event.get("id"))
            .and_then(|id| id.as_u64())
            == Some(target.window.id)
        {
            return Err("the captured niri window closed before its image was ready".into());
        }
        if event
            .get("ScreenshotCaptured")
            .and_then(|event| event.get("path"))
            .and_then(|path| path.as_str())
            == Some(path_string)
        {
            break;
        }
    }
    // 3. 【niri】【图片导入】在有界后台任务中读取原图，不重采样尺寸，不再次预乘合成器颜色
    let capture = tokio::task::spawn_blocking({
        let path = path.clone();
        let scale = target.scale;
        move || frame::read(&path, scale)
    })
    .await??;
    log::info!(
        "【niri】【窗口捕获】窗口 {}，原图 {}×{}，输出比例 {}",
        target.window.id,
        capture.pixmap.width(),
        capture.pixmap.height(),
        capture.scale
    );
    Ok(capture)
}
