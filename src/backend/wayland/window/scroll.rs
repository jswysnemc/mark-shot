//! 【原生窗口】【滚动信息】保留协议滚轮刻度、来源和修饰键，默认窗口继续接收原有像素位移

use smithay_client_toolkit::seat::pointer::AxisScroll;
use wayland_client::protocol::wl_pointer::AxisSource;

use crate::keys::Mods;

pub use crate::backend::window::{ScrollInput, ScrollSource};

impl ScrollInput {
    /// 【原生窗口】【协议转换】用 position、两方向 axis、source、mods 返回完整滚动信息，高分辨率刻度优先于旧离散刻度，不重复累计
    pub(crate) fn from_axes(
        position: (f64, f64),
        horizontal: AxisScroll,
        vertical: AxisScroll,
        source: Option<AxisSource>,
        mods: Mods,
    ) -> Self {
        Self {
            position,
            delta: (horizontal.absolute, vertical.absolute),
            angle: (angle(horizontal), angle(vertical)),
            source: match source {
                Some(AxisSource::Wheel) => ScrollSource::Wheel,
                Some(AxisSource::Finger) => ScrollSource::Finger,
                Some(AxisSource::Continuous) => ScrollSource::Continuous,
                Some(AxisSource::WheelTilt) => ScrollSource::WheelTilt,
                _ => ScrollSource::Unknown,
            },
            mods,
        }
    }
}

/// 【原生窗口】【方向刻度】按 axis 返回 Qt 方向的角度刻度，高分辨率和离散刻度同时出现时只使用前者
fn angle(axis: AxisScroll) -> f64 {
    if axis.value120 != 0 {
        -f64::from(axis.value120)
    } else if axis.discrete != 0 {
        -f64::from(axis.discrete) * 120.0
    } else {
        -axis.absolute * 12.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【滚动信息测试】【高分辨率优先】协议两种刻度不得重复计算，连续位移、来源与修饰键保留不变，无参数、无返回值
    #[test]
    fn resolution_metadata_does_not_double_count_and_legacy_delta_survives() {
        let mods = Mods {
            shift: true,
            ..Mods::default()
        };
        let input = ScrollInput::from_axes(
            (3.0, 7.0),
            AxisScroll {
                absolute: 15.0,
                discrete: 1,
                value120: 60,
                ..AxisScroll::default()
            },
            AxisScroll {
                absolute: -2.5,
                ..AxisScroll::default()
            },
            Some(AxisSource::Finger),
            mods,
        );
        assert_eq!(input.delta, (15.0, -2.5));
        assert_eq!(input.angle, (-60.0, 30.0));
        assert_eq!(input.source, ScrollSource::Finger);
        assert_eq!(input.mods, mods);
        assert_eq!(
            angle(AxisScroll {
                discrete: -2,
                ..AxisScroll::default()
            }),
            240.0
        );
        assert_eq!(
            angle(AxisScroll {
                stop: true,
                ..AxisScroll::default()
            }),
            0.0
        );
    }

    /// 【滚动信息测试】【真实 Qt 协议】逐项比较私有 Wayland 向原版交付的离散与无刻度来源，角度转换不能统一套用八倍系数，无参数、无返回值
    #[test]
    fn angles_match_actual_qt_wayland_wheel_finger_and_continuous_events() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/qt-ocr-table-scroll-wayland.json"
        ))
        .unwrap();
        for request in fixture["requests"].as_array().unwrap() {
            let source = match request["source"].as_u64().unwrap() {
                0 => AxisSource::Wheel,
                1 => AxisSource::Finger,
                2 => AxisSource::Continuous,
                _ => AxisSource::WheelTilt,
            };
            let input = ScrollInput::from_axes(
                (0.0, 0.0),
                AxisScroll::default(),
                AxisScroll {
                    absolute: request["absolute"].as_f64().unwrap(),
                    discrete: request["discrete"].as_i64().unwrap() as i32,
                    ..AxisScroll::default()
                },
                Some(source),
                Mods::default(),
            );
            let actual = request["qt_events"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|event| event["angle"][1] != 0)
                .unwrap();
            assert_eq!(
                input.angle.1,
                actual["angle"][1].as_f64().unwrap(),
                "{}",
                request["name"]
            );
            assert_eq!(
                (-input.delta.1).round(),
                actual["pixel"][1].as_f64().unwrap()
            );
        }
    }
}
