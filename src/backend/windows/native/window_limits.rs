//! 【Windows窗口】【尺寸限制】最小尺寸使用逻辑单位，系统消息按当前窗口DPI返回有界物理像素

/// 【Windows窗口】【最小像素】按minimum和dpi返回向上取整的物理最小尺寸，零DPI回退96，避免整数溢出
pub(super) fn physical(minimum: (u32, u32), dpi: u32) -> (i32, i32) {
    let dpi = if dpi == 0 { 96 } else { dpi };
    let convert = |value: u32| {
        (u64::from(value.max(1)) * u64::from(dpi))
            .div_ceil(96)
            .min(i32::MAX as u64) as i32
    };
    (convert(minimum.0), convert(minimum.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【Windows窗口测试】【比例边界】无参数，标准和分数DPI保持逻辑最小尺寸，零值和极端值不会溢出，无返回值
    #[test]
    fn minimum_size_scales_up_without_rounding_below_the_contract() {
        assert_eq!(physical((380, 340), 96), (380, 340));
        assert_eq!(physical((380, 340), 144), (570, 510));
        assert_eq!(physical((381, 341), 120), (477, 427));
        assert_eq!(physical((380, 340), 192), (760, 680));
        assert_eq!(physical((0, 0), 0), (1, 1));
        assert_eq!(
            physical((u32::MAX, u32::MAX), u32::MAX),
            (i32::MAX, i32::MAX)
        );
    }

    /// 【Windows窗口测试】【原生限制消息】无参数，在私有会话创建真实窗口并查询当前DPI对应最小尺寸，无返回值，不连接用户桌面
    #[test]
    #[ignore = "requires the private Windows test runtime"]
    fn native_minimum_message_uses_current_window_dpi() {
        use crate::backend::windows::native::{
            Bounds, EventQueue, Window, WindowOptions, enable_dpi,
        };
        use windows_sys::Win32::UI::{
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{MINMAXINFO, SendMessageW, WM_GETMINMAXINFO},
        };
        assert_eq!(
            std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
            Ok("1")
        );
        enable_dpi().unwrap();
        let window = Window::new(
            WindowOptions {
                title: "Mark Shot minimum size test".into(),
                bounds: Bounds {
                    left: 100,
                    top: 100,
                    width: 760,
                    height: 680,
                },
                minimum: (380, 340),
                topmost: false,
                activate: false,
                layered: false,
                owner: 0,
            },
            EventQueue::default(),
        )
        .unwrap();
        let mut limits: MINMAXINFO = unsafe { std::mem::zeroed() };
        let dpi = unsafe { GetDpiForWindow(window.handle() as _) };
        assert!(dpi > 0);
        if let Ok(expected) = std::env::var("MARK_SHOT_TEST_EXPECTED_DPI") {
            assert_eq!(
                dpi,
                expected.parse::<u32>().unwrap(),
                "private session did not apply the requested DPI"
            );
        }
        unsafe {
            SendMessageW(
                window.handle() as _,
                WM_GETMINMAXINFO,
                0,
                &mut limits as *mut _ as isize,
            );
        }
        assert_eq!(
            (limits.ptMinTrackSize.x, limits.ptMinTrackSize.y),
            physical((380, 340), dpi)
        );
        println!(
            "{{\"dpi\":{dpi},\"minimum\":[{},{}]}}",
            limits.ptMinTrackSize.x, limits.ptMinTrackSize.y
        );
    }
}
