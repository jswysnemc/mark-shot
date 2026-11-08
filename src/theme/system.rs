//! 【界面主题】【系统偏好】读取桌面外观选择，未提供偏好时沿用浅色
use super::Mode;
use std::sync::{
    OnceLock,
    atomic::{AtomicU8, Ordering},
};

static MODE: OnceLock<AtomicU8> = OnceLock::new();

/// 【界面主题】【系统模式】无参数，首次有界查询平台偏好，后续返回监听器更新的配色
pub(super) fn current() -> Mode {
    match MODE
        .get_or_init(|| AtomicU8::new(u8::from(detect() == Mode::Dark)))
        .load(Ordering::Relaxed)
    {
        1 => Mode::Dark,
        _ => Mode::Light,
    }
}

/// 【界面主题】【系统更新】按 mode 更新缓存，返回外观是否改变，不写用户配置
pub(super) fn update(mode: Mode) -> bool {
    let next = u8::from(mode == Mode::Dark);
    MODE.get_or_init(|| AtomicU8::new(next))
        .swap(next, Ordering::Relaxed)
        != next
}

/// 【界面主题】【系统重读】无参数，供平台主题事件重新读取偏好，返回是否改变
#[cfg(target_os = "windows")]
pub(crate) fn refresh() -> bool {
    update(detect())
}

/// 【Linux主题】【外观查询】无参数，通过桌面门户读取颜色偏好，300 毫秒超时后使用 GTK 偏好或浅色
#[cfg(target_os = "linux")]
fn detect() -> Mode {
    if cfg!(test) {
        return Mode::Light;
    }
    let result = std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_millis(300), async {
                let settings = ashpd::desktop::settings::Settings::new().await.ok()?;
                let scheme = settings.color_scheme().await.ok()?;
                Some(
                    if scheme == ashpd::desktop::settings::ColorScheme::PreferDark {
                        Mode::Dark
                    } else {
                        Mode::Light
                    },
                )
            })
            .await
            .ok()
            .flatten()
        })
    })
    .join()
    .ok()
    .flatten();
    result.unwrap_or_else(|| {
        if std::env::var("GTK_THEME").is_ok_and(|theme| theme.to_ascii_lowercase().contains("dark"))
        {
            Mode::Dark
        } else {
            Mode::Light
        }
    })
}

/// 【Windows主题】【外观查询】无参数，读取当前用户应用颜色偏好，缺失或读取失败返回浅色
#[cfg(target_os = "windows")]
fn detect() -> Mode {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let mut value: u32 = 1;
    let mut length = size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            windows_sys::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            windows_sys::w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut length,
        )
    };
    if result == 0 && value == 0 {
        Mode::Dark
    } else {
        Mode::Light
    }
}
