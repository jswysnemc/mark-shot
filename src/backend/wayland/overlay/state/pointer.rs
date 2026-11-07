// ── pointer input handling ───────────────────────────────────────────
// pointer mouvement, actions, and shape change
// use of 'pointer_frame', to group multiple mouse events in one cycle
// instead of sending them one by one

use smithay_client_toolkit::seat::pointer::{
    AxisScroll, PointerEvent, PointerEventKind, PointerHandler,
};
use wayland_client::protocol::wl_pointer;
use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::types::{
    MouseButton, OverlayEvent, ScrollAxis, ScrollDirection, ScrollEvent, ScrollSource,
};
impl PointerHandler for OverlayState {
    /// 【Wayland覆盖层】【指针分派】按 pointer 与 events 保留真实按钮时间、序列号和坐标，回调连接与队列不参与转换，无返回值
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if self.popup_pointer(event) {
                continue;
            }
            // searching on which surface (each surface correspond to one output) it occured
            let monitor_idx = self
                .surfaces
                .iter()
                .find(|(_, sd)| sd.surface == event.surface)
                .map(|(id, _)| *id);
            let panel = monitor_idx.and_then(|index| self.surfaces.get(&index));
            let position = panel.map_or(event.position, |surface| {
                surface.canvas_point(event.position)
            });
            let scroll_window = panel.is_some_and(|surface| surface.canvas_rect.is_some());
            match event.kind {
                PointerEventKind::Enter { serial } => {
                    self.pointer_enter_serial = serial;
                    self.pointer_surface_idx = monitor_idx;

                    self.cursor_applied_since_enter = false;
                    self.apply_cursor(self.current_cursor_icon);

                    if let Some(idx) = monitor_idx {
                        self.events.push_back(OverlayEvent::PointerMove {
                            monitor_idx: idx,
                            x: position.0,
                            y: position.1,
                            history: Vec::new(),
                        });
                    }
                }

                PointerEventKind::Leave { .. } => {
                    self.pointer_surface_idx = None;
                }

                PointerEventKind::Motion { .. } => {
                    if let Some(idx) = self.pointer_surface_idx {
                        self.events.push_back(OverlayEvent::PointerMove {
                            monitor_idx: idx,
                            x: position.0,
                            y: position.1,
                            history: Vec::new(),
                        });
                    }
                }

                PointerEventKind::Press { button, time, .. }
                | PointerEventKind::Release { button, time, .. } => {
                    let pressed = matches!(event.kind, PointerEventKind::Press { .. });
                    // 3. 【滚动截图】【点击位置】布局更新后即使指针未移动，也使用本次按钮事件的真实窗口坐标
                    if scroll_window && let Some(index) = monitor_idx {
                        self.events.push_back(OverlayEvent::PointerMove {
                            monitor_idx: index,
                            x: position.0,
                            y: position.1,
                            history: Vec::new(),
                        });
                    }
                    let serial = match event.kind {
                        PointerEventKind::Press { serial, .. } => {
                            if let Some(data) = wayland_client::Proxy::data::<
                                smithay_client_toolkit::seat::pointer::PointerData<()>,
                            >(pointer)
                            {
                                self.popup_grabs.push_back((serial, data.seat().clone()));
                            }
                            Some(serial)
                        }
                        _ => None,
                    };

                    // linux input codes (BTN_LEFT=0x110, BTN_RIGHT=0x111, BTN_MIDDLE=0x112)
                    let mb = match button {
                        0x110 => MouseButton::Left,
                        0x111 => MouseButton::Right,
                        0x112 => MouseButton::Middle,
                        _ => continue,
                    };
                    // 2. 【主选择区】【释放授权】每个应用按钮保留独立座席序列，菜单 serial 仍仅限按下事件
                    if let Some(data) = wayland_client::Proxy::data::<
                        smithay_client_toolkit::seat::pointer::PointerData<()>,
                    >(pointer)
                    {
                        let serial = match event.kind {
                            PointerEventKind::Press { serial, .. }
                            | PointerEventKind::Release { serial, .. } => serial,
                            _ => unreachable!(),
                        };
                        self.primary_selection.button(data.seat().clone(), serial);
                    }
                    // 1. 【Wayland覆盖层】【按钮时间】平台时间与菜单序列号分别保存，输入处理延迟不能改变双击间隔
                    self.events.push_back(OverlayEvent::PointerButton {
                        button: mb,
                        pressed,
                        serial,
                        timestamp: Some(time),
                    });
                }

                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    source,
                    ..
                } => {
                    if scroll_window && let Some(index) = monitor_idx {
                        self.events.push_back(OverlayEvent::PointerMove {
                            monitor_idx: index,
                            x: position.0,
                            y: position.1,
                            history: Vec::new(),
                        });
                    }
                    let event = ScrollEvent {
                        horizontal: scroll_axis(horizontal, false),
                        vertical: scroll_axis(vertical, true),
                        source: source.and_then(scroll_source),
                    };
                    if !event.is_empty() {
                        self.events.push_back(OverlayEvent::Scroll(event));
                    }
                }
            }
        }
    }
}

/// 【Wayland】【滚轮轴转换】把 axis 转换为应用轴数据，vertical 控制纵轴一次反号；返回保留刻度、方向和停止信息的轴
fn scroll_axis(axis: AxisScroll, vertical: bool) -> ScrollAxis {
    ScrollAxis {
        pixels: if vertical {
            -axis.absolute
        } else {
            axis.absolute
        },
        discrete: if vertical {
            axis.discrete.saturating_neg()
        } else {
            axis.discrete
        },
        value120: if vertical {
            axis.value120.saturating_neg()
        } else {
            axis.value120
        },
        angle_delta: None,
        direction: match axis.relative_direction {
            Some(wl_pointer::AxisRelativeDirection::Identical) => Some(ScrollDirection::Identical),
            Some(wl_pointer::AxisRelativeDirection::Inverted) => Some(ScrollDirection::Inverted),
            _ => None,
        },
        stop: axis.stop,
    }
}

/// 【Wayland】【滚轮来源】返回 source 对应的稳定输入来源；未知协议扩展返回空值，不改变轴运动
fn scroll_source(source: wl_pointer::AxisSource) -> Option<ScrollSource> {
    match source {
        wl_pointer::AxisSource::Wheel => Some(ScrollSource::Wheel),
        wl_pointer::AxisSource::Finger => Some(ScrollSource::Finger),
        wl_pointer::AxisSource::Continuous => Some(ScrollSource::Continuous),
        wl_pointer::AxisSource::WheelTilt => Some(ScrollSource::WheelTilt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【Wayland测试】【滚轮协议】验证所有纵轴数值只反号一次、方向与停止保留，无参数、无返回值
    #[test]
    fn axis_keeps_protocol_metadata_without_double_inversion() {
        let axis = AxisScroll {
            absolute: -2.5,
            discrete: -1,
            value120: -30,
            relative_direction: Some(wl_pointer::AxisRelativeDirection::Inverted),
            stop: true,
        };
        let vertical = scroll_axis(axis, true);
        assert_eq!(
            (vertical.pixels, vertical.discrete, vertical.value120),
            (2.5, 1, 30)
        );
        assert_eq!(vertical.direction, Some(ScrollDirection::Inverted));
        assert!(vertical.stop);
        let horizontal = scroll_axis(axis, false);
        assert_eq!(
            (horizontal.pixels, horizontal.discrete, horizontal.value120),
            (-2.5, -1, -30)
        );
        assert_eq!(
            scroll_source(wl_pointer::AxisSource::Finger),
            Some(ScrollSource::Finger)
        );
        assert_eq!(
            scroll_source(wl_pointer::AxisSource::Wheel),
            Some(ScrollSource::Wheel)
        );
    }

    /// 【Wayland测试】【触控板角度】验证 Qt Wayland 对没有刻度的触控板像素合成角度，并按帧取整，无参数、无返回值
    #[test]
    fn continuous_axes_match_qt_platform_angle_synthesis() {
        for (pixels, angle) in [(-2.5, 30.0), (-0.04, 0.0), (-0.05, 1.0), (0.05, -1.0)] {
            let event = ScrollEvent {
                vertical: scroll_axis(
                    AxisScroll {
                        absolute: pixels,
                        ..Default::default()
                    },
                    true,
                ),
                source: Some(ScrollSource::Finger),
                ..Default::default()
            };
            assert_eq!(event.angle_y(), angle);
            assert_eq!(event.canvas_steps(), angle / 120.0);
        }
    }
}
