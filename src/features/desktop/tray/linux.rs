//! 【系统托盘】【Linux协议】向桌面注册真实图标和DBusMenu，桌面重启后重新登记

use super::super::{Action, Reporter};
use ksni::TrayMethods;

struct Tray {
    reporter: Reporter,
    pixels: Vec<u8>,
    menu: super::menu::State,
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
    /// 【系统托盘】【操作菜单】无参数，将共用菜单转换为宿主支持的项目、分隔线和延时子菜单
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        items(super::menu::entries(self.menu))
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

/// 【系统托盘】【菜单转换】按 entries 递归生成 DBusMenu，回调只交付动作，不在宿主线程创建窗口
fn items(entries: Vec<super::menu::Item>) -> Vec<ksni::MenuItem<Tray>> {
    entries
        .into_iter()
        .map(|entry| match entry {
            super::menu::Item::Separator => ksni::MenuItem::Separator,
            super::menu::Item::Submenu {
                label,
                items: children,
            } => ksni::menu::SubMenu {
                label,
                submenu: items(children),
                ..Default::default()
            }
            .into(),
            super::menu::Item::Action {
                label,
                action,
                enabled,
            } => ksni::menu::StandardItem {
                label,
                enabled,
                activate: Box::new(move |tray: &mut Tray| {
                    if enabled {
                        tray.reporter
                            .activated(action, ksni::current_xdg_activation_token());
                    }
                }),
                ..Default::default()
            }
            .into(),
        })
        .collect()
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
    mut menu: tokio::sync::watch::Receiver<super::menu::State>,
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
        menu: *menu.borrow_and_update(),
    }
    .assume_sni_available(true)
    // 【系统托盘】【沙箱注册】Flatpak 不能申请带进程号的固定服务名，改用连接唯一名称
    .disable_dbus_name(ashpd::is_sandboxed())
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
    loop {
        tokio::select! {
            _ = cancelled.changed() => break,
            result = menu.changed() => {
                if result.is_err() { break; }
                let state = *menu.borrow_and_update();
                if handle.update(move |tray| tray.menu = state).await.is_none() { break; }
            }
        }
    }
    handle.shutdown().await;
    Ok(())
}
