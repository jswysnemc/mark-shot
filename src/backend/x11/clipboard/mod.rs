//! 【X11剪贴板】【选择区服务】持有PNG或UTF-8原始数据，支持ICCCM分块传输

mod read;
mod serve;
mod transfer;
pub use read::{read, targets};
pub use serve::serve;

pub const LIMIT: usize = 128 * 1024 * 1024;

/// 【Linux剪贴板】【协议选择】无参数，根据真实会话及数据控制能力选择X11，返回探测错误
pub fn use_x11() -> crate::AppResult<bool> {
    match wayland_controlled()? {
        None => Ok(true),
        Some(true) => Ok(false),
        Some(false) if std::env::var_os("DISPLAY").is_some() => Ok(true),
        Some(false) => Err(
            "this Wayland compositor has no data-control clipboard or X11 clipboard reader".into(),
        ),
    }
}

/// 【Linux剪贴板】【复制协议】无参数，按真实会话选择复制服务，返回协议结果；Wayland 缺少数据控制时禁止无焦点 X11 写入
pub fn use_x11_for_copy() -> crate::AppResult<bool> {
    match wayland_controlled()? {
        None => Ok(true),
        Some(true) => Ok(false),
        Some(false) => Err("this Wayland compositor has no data-control clipboard; copy from a focused Mark Shot window using the core Wayland clipboard protocol".into()),
    }
}

/// 【Linux剪贴板】【能力探测】无参数，返回 Wayland 数据控制能力；X11 会话返回空值，连接失败返回错误
fn wayland_controlled() -> crate::AppResult<Option<bool>> {
    use crate::backend::session::Session;
    let Session::Wayland(connection) = Session::connect_to_env()? else {
        return Ok(None);
    };
    let (globals, _) = wayland_client::globals::registry_queue_init::<Probe>(&connection)?;
    let controlled = globals.contents().with_list(|globals| {
        globals.iter().any(|global| {
            matches!(
                global.interface.as_str(),
                "ext_data_control_manager_v1" | "zwlr_data_control_manager_v1"
            )
        })
    });
    Ok(Some(controlled))
}

struct Probe;

impl
    wayland_client::Dispatch<
        wayland_client::protocol::wl_registry::WlRegistry,
        wayland_client::globals::GlobalListContents,
    > for Probe
{
    /// 【Linux剪贴板】【注册表探测】接收registry和event等协议参数，静态能力探测无需更新状态，无返回值
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_registry::WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &wayland_client::globals::GlobalListContents,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
