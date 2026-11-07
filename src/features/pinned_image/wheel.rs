//! 【贴图】【滚轮缩放】复用原版刻度倍率并计算保持局部锚点的窗口位移，不读取或改写原图

/// 【贴图】【滚轮倍率】按angle与pixels的纵轴分量返回1.08倍刻度倍率，角度优先，零值或无效输入返回空值
pub fn factor(angle: f64, pixels: f64) -> Option<f32> {
    let steps = if angle.is_finite() && angle.abs() > 1e-12 {
        angle / 120.0
    } else if pixels.is_finite() && pixels.abs() > 1e-12 {
        pixels / 80.0
    } else {
        return None;
    };
    Some(1.08_f64.powf(steps.clamp(-100.0, 100.0)) as f32)
}

/// 【贴图】【锚点位移】按point、before和after返回逻辑窗口位移，尺寸改变后保持同一原图位置，空尺寸和无效坐标返回零位移
pub fn offset(point: (f64, f64), before: (u32, u32), after: (u32, u32)) -> (f64, f64) {
    if before.0 == 0 || before.1 == 0 || !point.0.is_finite() || !point.1.is_finite() {
        return (0.0, 0.0);
    }
    (
        point.0 * (1.0 - f64::from(after.0) / f64::from(before.0)),
        point.1 * (1.0 - f64::from(after.1) / f64::from(before.1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【贴图测试】【原版刻度】无参数，角度与像素符合原版倍率且忽略水平或无效输入，无返回值
    #[test]
    fn angle_and_pixel_steps_match_reference() {
        assert_eq!(factor(120.0, 0.0), Some(1.08));
        assert_eq!(factor(0.0, 80.0), Some(1.08));
        assert_eq!(factor(120.0, -80.0), Some(1.08));
        assert!((factor(-120.0, 0.0).unwrap() - 1.0 / 1.08).abs() < 1e-6);
        assert_eq!(factor(0.0, 0.0), None);
        assert_eq!(factor(f64::NAN, f64::NAN), None);
    }

    /// 【贴图测试】【锚点不变】无参数，缩放前后同一原图点全局位置保持不变，最小尺寸及异常输入安全，无返回值
    #[test]
    fn offset_preserves_the_same_source_point() {
        let before = (320, 200);
        let after = (346, 216);
        let point = (80.0, 50.0);
        let shift = offset(point, before, after);
        assert!((shift.0 + point.0 * 346.0 / 320.0 - point.0).abs() < 1e-9);
        assert!((shift.1 + point.1 * 216.0 / 200.0 - point.1).abs() < 1e-9);
        assert_eq!(offset(point, before, before), (0.0, 0.0));
        assert_eq!(offset(point, (0, 0), after), (0.0, 0.0));
        assert_eq!(offset((f64::NAN, 1.0), before, after), (0.0, 0.0));
    }
}
