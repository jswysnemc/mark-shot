//! 【系统托盘】【Linux协议】向桌面注册真实图标和DBusMenu，桌面重启后重新登记

use super::super::{Action, Reporter};
use ksni::TrayMethods;

struct Tray {
    reporter: Reporter,
    pixels: Vec<u8>,
}

impl ksni::Tray for Tray {
    /// 【系统托盘】【稳定身份】无参数，返回同一应用的稳定标识
    fn id(&self) -> String {
        "mark-shot".into()
    }
    /// 【系统托盘】【可见标题】无参数，返回应用名称
    fn title(&self) -> String {
        "Mark Shot".into()
    }
    /// 【系统托盘】【实际图标】无参数，返回内嵌图标的网络字节序ARGB像素
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: 32,
            height: 32,
            data: self.pixels.clone(),
        }]
    }
    /// 【系统托盘】【点击截图】按宿主坐标触发普通截图，无返回值
    fn activate(&mut self, _horizontal: i32, _vertical: i32) {
        self.reporter
            .activated(Action::Capture, ksni::current_xdg_activation_token());
    }
    /// 【系统托盘】【操作菜单】无参数，返回截图、剪贴板贴图、设置及退出动作
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        super::entries()
            .into_iter()
            .map(|(label, action)| {
                ksni::menu::StandardItem {
                    label: label.into(),
                    activate: Box::new(move |tray: &mut Self| {
                        tray.reporter
                            .activated(action, ksni::current_xdg_activation_token())
                    }),
                    ..Default::default()
                }
                .into()
            })
            .collect()
    }
    /// 【系统托盘】【宿主恢复】无参数，在托盘区域重新上线时报告注册恢复
    fn watcher_online(&self) {
        self.reporter
            .status(super::super::status::Registration::Registered(String::new()));
    }
    /// 【系统托盘】【宿主离线】按reason报告缺失托盘区域，返回保留注册以等待桌面恢复
    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        self.reporter
            .status(super::super::status::Registration::Unavailable);
        true
    }
}

/// 【系统托盘】【图标栅格】无参数，将项目已有SVG转为ARGB像素，返回解码错误
fn pixels() -> Result<Vec<u8>, String> {
    let image = super::icon()?;
    Ok(image
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.alpha(), color.red(), color.green(), color.blue()]
        })
        .collect())
}

/// 【系统托盘】【后台服务】按reporter报告状态，cancelled通知退出，返回注册错误并回收D-Bus对象
pub(super) async fn run(
    reporter: &Reporter,
    mut cancelled: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?;
    let proxy = zbus::fdo::DBusProxy::new(&connection)
        .await
        .map_err(|error| error.to_string())?;
    let registration = Tray {
        reporter: reporter.clone(),
        pixels: pixels()?,
    }
    .assume_sni_available(true)
    .spawn();
    let handle = tokio::select! {
        _ = cancelled.changed() => return Ok(()),
        result = registration => result.map_err(|error| error.to_string())?,
    };
    if proxy
        .name_has_owner("org.kde.StatusNotifierWatcher".try_into().unwrap())
        .await
        .unwrap_or(false)
    {
        reporter.status(super::super::status::Registration::Registered(String::new()));
    }
    let _ = cancelled.changed().await;
    handle.shutdown().await;
    Ok(())
}
