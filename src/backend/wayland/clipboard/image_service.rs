//! 【图片剪贴板】【协议服务】通过 ext 或 wlr 数据控制协议提供文件来源，保持 PNG 与 KDE 图片提示 MIME

use super::image_source::ImageSource;
use std::{
    collections::HashSet,
    fs::File,
    io::{self, Write},
    sync::Arc,
};
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry, wl_seat},
};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1 as ext_device, ext_data_control_manager_v1 as ext_manager,
    ext_data_control_offer_v1 as ext_offer, ext_data_control_source_v1 as ext_source,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1 as wlr_device, zwlr_data_control_manager_v1 as wlr_manager,
    zwlr_data_control_offer_v1 as wlr_offer, zwlr_data_control_source_v1 as wlr_source,
};

const PNG: &str = "image/png";
const KDE_IMAGE: &str = "x-kde-force-image-copy";

struct State {
    image: Arc<ImageSource>,
    active: HashSet<u32>,
}

/// 【图片剪贴板】【独立服务】接收标准输入 PNG 并占有全部现有座席，primary 选择主选区；返回接管结束或协议错误
pub(super) fn serve(primary: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = dirs::cache_dir()
        .ok_or("clipboard cache directory is unavailable")?
        .join("mark-shot/clipboard");
    let image = Arc::new(ImageSource::receive(io::stdin().lock(), &root)?);
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let handle = queue.handle();
    let mut state = State {
        image,
        active: HashSet::new(),
    };
    let seats: Vec<_> = globals
        .contents()
        .clone_list()
        .into_iter()
        .filter(|global| global.interface == "wl_seat")
        .collect();
    if seats.is_empty() {
        return Err("clipboard seat is unavailable".into());
    }
    // 1. 【图片剪贴板】【协议选择】优先标准 ext 数据控制，旧合成器继续使用 wlr 数据控制
    if let Ok(manager) =
        globals.bind::<ext_manager::ExtDataControlManagerV1, _, _>(&handle, 1..=1, ())
    {
        for global in seats {
            let seat = globals.registry().bind::<wl_seat::WlSeat, _, _>(
                global.name,
                global.version.min(7),
                &handle,
                (),
            );
            let device = manager.get_data_device(&seat, &handle, global.name);
            let source = manager.create_data_source(&handle, global.name);
            source.offer(PNG.into());
            source.offer(KDE_IMAGE.into());
            if primary {
                device.set_primary_selection(Some(&source));
            } else {
                device.set_selection(Some(&source));
            }
            state.active.insert(global.name);
        }
    } else {
        let version = if primary { 2..=2 } else { 1..=2 };
        let manager =
            globals.bind::<wlr_manager::ZwlrDataControlManagerV1, _, _>(&handle, version, ())?;
        for global in seats {
            let seat = globals.registry().bind::<wl_seat::WlSeat, _, _>(
                global.name,
                global.version.min(7),
                &handle,
                (),
            );
            let device = manager.get_data_device(&seat, &handle, global.name);
            let source = manager.create_data_source(&handle, global.name);
            source.offer(PNG.into());
            source.offer(KDE_IMAGE.into());
            if primary {
                device.set_primary_selection(Some(&source));
            } else {
                device.set_selection(Some(&source));
            }
            state.active.insert(global.name);
        }
    }
    queue.roundtrip(&mut state)?;
    if state.active.is_empty() {
        return Err("clipboard selection was rejected".into());
    }
    // 2. 【图片剪贴板】【交接确认】合成器处理占有请求之后才通知父进程，接管后退出并关闭匿名文件
    writeln!(io::stdout())?;
    io::stdout().flush()?;
    log::info!("【图片剪贴板】【开始服务】PNG字节数={}", state.image.bytes);
    while !state.active.is_empty() {
        queue.blocking_dispatch(&mut state)?;
    }
    Ok(())
}

macro_rules! data_control_dispatch {
    ($device:ident, $device_type:ident, $source:ident, $source_type:ident, $offer:ident, $offer_type:ident) => {
        impl Dispatch<$device::$device_type, u32> for State {
            /// 【图片剪贴板】【设备事件】根据 event 回收接收报价或已关闭的 seat，data 为当前设备；无返回值
            fn event(state: &mut Self, data: &$device::$device_type, event: $device::Event, seat: &u32, _: &Connection, _: &QueueHandle<Self>) {
                match event {
                    $device::Event::Selection { id: Some(offer) } | $device::Event::PrimarySelection { id: Some(offer) } => offer.destroy(),
                    $device::Event::Finished => { state.active.remove(seat); data.destroy(); }
                    _ => {}
                }
            }
            wayland_client::event_created_child!(State, $device::$device_type, [0 => ($offer::$offer_type, ())]);
        }

        impl Dispatch<$source::$source_type, u32> for State {
            /// 【图片剪贴板】【来源事件】按 event 独立发送文件或结束 seat，data 为当前来源；无返回值
            fn event(state: &mut Self, data: &$source::$source_type, event: $source::Event, seat: &u32, _: &Connection, _: &QueueHandle<Self>) {
                match event {
                    $source::Event::Send { mime_type, fd } if mime_type == PNG => {
                        let image = Arc::clone(&state.image);
                        let result = std::thread::Builder::new().name("clipboard-send".into()).spawn(move || {
                            if let Err(error) = image.send(File::from(fd)) {
                                log::info!("【图片剪贴板】【发送结束】{error}");
                            }
                        });
                        if let Err(error) = result { log::error!("【图片剪贴板】【发送启动】{error}"); }
                    }
                    $source::Event::Cancelled => { state.active.remove(seat); data.destroy(); }
                    _ => {}
                }
            }
        }
        delegate_noop!(State: ignore $offer::$offer_type);
    };
}

data_control_dispatch!(
    ext_device,
    ExtDataControlDeviceV1,
    ext_source,
    ExtDataControlSourceV1,
    ext_offer,
    ExtDataControlOfferV1
);
data_control_dispatch!(
    wlr_device,
    ZwlrDataControlDeviceV1,
    wlr_source,
    ZwlrDataControlSourceV1,
    wlr_offer,
    ZwlrDataControlOfferV1
);
delegate_noop!(State: ignore ext_manager::ExtDataControlManagerV1);
delegate_noop!(State: ignore wlr_manager::ZwlrDataControlManagerV1);
delegate_noop!(State: ignore wl_seat::WlSeat);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    /// 【图片剪贴板】【注册事件】初始全局表由框架维护，当前服务的关闭由数据控制设备事件处理；参数为框架事件，无返回值
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
