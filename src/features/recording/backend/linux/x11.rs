//! 【X11录制】【坐标解析】把单显示器相对区域转换为组件要求的根窗口坐标

use crate::{backend::output_selection, features::recording::Target, types::Output};

/// 【X11录制】【来源解析】按 target 查找实际显示器并返回区域参数，显示器移除时返回错误
pub(super) fn region(target: &Target) -> Result<String, String> {
    let display =
        crate::backend::x11::connection::Display::connect().map_err(|error| error.to_string())?;
    let outputs = display.outputs().map_err(|error| error.to_string())?;
    let output = outputs
        .iter()
        .enumerate()
        .find(|(index, output)| {
            target.display.is_empty() || output_selection::name(output, *index) == target.display
        })
        .map(|(_, output)| output)
        .ok_or("录制显示器已移除 / Display removed")?;
    geometry(output, target.crop)
}

/// 【X11录制】【根窗口区域】按 output 和可选 crop 校验范围并返回 WxH+X+Y，不允许越界或奇数编码尺寸
fn geometry(output: &Output, crop: Option<[u32; 4]>) -> Result<String, String> {
    let (width, height) = output.info.logical_size.ok_or("X11 output has no size")?;
    let (left, top) = output.info.logical_position.unwrap_or(output.info.location);
    let [x, y, w, h] = crop.unwrap_or([0, 0, width as u32 & !1, height as u32 & !1]);
    if w < 2
        || h < 2
        || w % 2 != 0
        || h % 2 != 0
        || x.checked_add(w).is_none_or(|end| end > width as u32)
        || y.checked_add(h).is_none_or(|end| end > height as u32)
    {
        return Err("录制区域无效 / Invalid recording region".into());
    }
    Ok(format!(
        "{w}x{h}+{}+{}",
        i64::from(left) + i64::from(x),
        i64::from(top) + i64::from(y)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【X11录制测试】【多屏区域】无参数，验证次屏相对坐标转换为根窗口坐标且拒绝越界
    #[test]
    fn region_uses_monitor_origin_and_validates_bounds() {
        let output = Output {
            wl_output: None,
            info: crate::types::linux_output::Info::x11(
                1,
                "second".into(),
                (1920, 100),
                (1280, 720),
            ),
        };
        assert_eq!(
            geometry(&output, Some([20, 40, 320, 240])).unwrap(),
            "320x240+1940+140"
        );
        assert_eq!(geometry(&output, None).unwrap(), "1280x720+1920+100");
        assert!(geometry(&output, Some([1200, 0, 320, 240])).is_err());
        assert!(geometry(&output, Some([0, 0, 321, 240])).is_err());
    }
}
