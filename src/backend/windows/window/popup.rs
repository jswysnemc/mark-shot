//! 【Windows独立弹层】【资源链】按父子定位创建自绘菜单，逆序回收后才交付业务动作

use super::{
    super::{input::Decoder, native},
    runtime::Surface,
};
use crate::{
    backend::window::{WindowEvent, WindowPopupAction},
    types::popup::PopupEvent,
};

pub(super) struct Popup {
    pub(super) id: u64,
    pub(super) window: native::Window,
    pub(super) decoder: Decoder,
}

/// 【Windows独立窗口】【弹层输入】按event返回共享弹层消息，原生关闭和无对应输入返回空值
pub(super) fn popup_event(event: WindowEvent) -> Option<PopupEvent> {
    use crate::types::MouseButton;
    Some(match event {
        WindowEvent::PointerMove { position } => PopupEvent::Pointer {
            x: position.0,
            y: position.1,
        },
        WindowEvent::PointerButton {
            position,
            button,
            pressed,
        } => PopupEvent::Button {
            x: position.0,
            y: position.1,
            button: match button {
                0x110 => MouseButton::Left,
                0x111 => MouseButton::Right,
                0x112 => MouseButton::Middle,
                _ => return None,
            },
            pressed,
        },
        WindowEvent::PointerLeave => PopupEvent::Leave,
        WindowEvent::Wheel(input) => PopupEvent::Scroll {
            delta: (-input.delta.0, -input.delta.1),
        },
        WindowEvent::Key(input) => PopupEvent::Key {
            chord: input.key.map(|key| crate::keys::Chord {
                key,
                mods: input.mods,
            }),
            text: (!input.text.is_empty()).then_some(input.text),
        },
        _ => return None,
    })
}

/// 【Windows独立窗口】【菜单同步】按surface待显示链更新popups，queue用于原生输入归属，返回系统或契约校验错误
pub(super) fn sync_popups(
    surface: &mut Surface,
    popups: &mut Vec<Popup>,
    queue: &native::EventQueue,
) -> crate::AppResult {
    let nodes = surface.content.popup_nodes();
    if !crate::types::popup_tree::valid(&nodes, surface.content.popup_id()) {
        return Err("invalid Windows popup chain".into());
    }
    let keep = popups
        .iter()
        .zip(&nodes)
        .take_while(|(popup, node)| popup.id == node.id)
        .count();
    if keep < popups.len() {
        let id = popups[keep].id;
        if finish(surface, popups, id, None) {
            return Err("Windows content closed while replacing its popup chain".into());
        }
    }
    let parent = surface.window.bounds()?;
    for (index, node) in nodes.iter().enumerate() {
        let Some(frame) = surface.content.popup_frame_for(node.id, surface.scale) else {
            continue;
        };
        crate::types::popup::validate(&frame)?;
        if index >= popups.len() {
            let origin = if index == 0 {
                parent
            } else {
                popups[index - 1].window.bounds()?
            };
            let (x, y) = match node.placement {
                crate::types::popup_tree::Placement::Point => {
                    (frame.anchor.0.floor(), frame.anchor.1.floor())
                }
                crate::types::popup_tree::Placement::BelowControl(rect) => {
                    (rect.left(), rect.bottom() + 4.0)
                }
                crate::types::popup_tree::Placement::Submenu {
                    parent_width,
                    row_top,
                    item_inset,
                } => (parent_width - 2.0, row_top - item_inset),
            };
            let mut left = origin.left + (x * surface.scale).round() as i32;
            let mut top = origin.top + (y * surface.scale).round() as i32;
            let monitors = native::monitors()?;
            if let Some(monitor) = monitors
                .iter()
                .find(|monitor| {
                    let bounds = monitor.bounds;
                    left >= bounds.left
                        && top >= bounds.top
                        && i64::from(left) < i64::from(bounds.left) + i64::from(bounds.width)
                        && i64::from(top) < i64::from(bounds.top) + i64::from(bounds.height)
                })
                .or_else(|| monitors.iter().find(|monitor| monitor.primary))
            {
                let bounds = monitor.bounds;
                let right = bounds.left.saturating_add(bounds.width as i32);
                let bottom = bounds.top.saturating_add(bounds.height as i32);
                if left.saturating_add(frame.pixel_size.0 as i32) > right {
                    left = if index == 0 {
                        right - frame.pixel_size.0 as i32
                    } else {
                        origin.left - frame.pixel_size.0 as i32 + 2
                    };
                }
                if top.saturating_add(frame.pixel_size.1 as i32) > bottom && index == 0 {
                    top = origin.top + (frame.anchor.1 * surface.scale).round() as i32
                        - frame.pixel_size.1 as i32;
                }
                left = left.clamp(
                    bounds.left,
                    (right - frame.pixel_size.0 as i32).max(bounds.left),
                );
                top = top.clamp(
                    bounds.top,
                    (bottom - frame.pixel_size.1 as i32).max(bounds.top),
                );
            }
            let owner = if index == 0 {
                surface.window.handle()
            } else {
                popups[index - 1].window.handle()
            };
            let window = native::Window::new(
                native::WindowOptions {
                    title: "Mark Shot menu".into(),
                    bounds: native::Bounds {
                        left,
                        top,
                        width: frame.pixel_size.0,
                        height: frame.pixel_size.1,
                    },
                    minimum: (1, 1),
                    topmost: true,
                    activate: false,
                    layered: true,
                    owner,
                },
                queue.clone(),
            )?;
            popups.push(Popup {
                id: node.id,
                window,
                decoder: Decoder::default(),
            });
        }
        let created = index >= keep;
        popups[index]
            .window
            .set_rgba(frame.pixel_size, frame.pixels)?;
        if created {
            let bounds = popups[index].window.bounds()?;
            surface.content.popup_event(
                node.id,
                PopupEvent::Configured {
                    position: (
                        ((bounds.left - parent.left) as f32 / surface.scale).round() as i32,
                        ((bounds.top - parent.top) as f32 / surface.scale).round() as i32,
                    ),
                },
            );
        }
    }
    Ok(())
}

/// 【Windows独立弹层】【关闭交付】按surface、popups、id与selection先逆序销毁所需节点，再交付一次动作，返回主内容是否请求关闭
pub(super) fn finish(
    surface: &mut Surface,
    popups: &mut Vec<Popup>,
    id: u64,
    selection: Option<usize>,
) -> bool {
    let Some(index) = popups.iter().position(|popup| popup.id == id) else {
        return false;
    };
    let removed = popups.split_off(if selection.is_some() { 0 } else { index });
    let ids = removed.iter().map(|popup| popup.id).collect::<Vec<_>>();
    for popup in removed.into_iter().rev() {
        drop(popup);
    }
    let mut close = false;
    let lowering = surface.content.always_on_top() && selection.is_some();
    if lowering && let Err(error) = surface.window.set_topmost(false) {
        log::error!("【Windows独立弹层】【业务交付】无法临时降低父窗口: {error}");
    }
    for closed in ids.into_iter().rev() {
        let selected = if closed == id { selection } else { None };
        let action = surface.content.popup_closed(closed, selected);
        close |= surface.apply(action);
    }
    if lowering && let Err(error) = surface.window.set_topmost(surface.content.always_on_top()) {
        log::error!("【Windows独立弹层】【业务交付】无法恢复父窗口置顶: {error}");
    }
    close
}

/// 【Windows独立弹层】【动作分派】按surface、popups、id与action完成对应菜单链，返回内容关闭请求，重绘由内容缓存决定
pub(super) fn apply(
    surface: &mut Surface,
    popups: &mut Vec<Popup>,
    id: u64,
    action: WindowPopupAction,
) -> bool {
    match action {
        WindowPopupAction::Activate(item) => finish(surface, popups, id, Some(item)),
        WindowPopupAction::Close => finish(surface, popups, id, None),
        WindowPopupAction::CloseAll => popups
            .first()
            .map(|popup| popup.id)
            .is_some_and(|root| finish(surface, popups, root, None)),
        _ => false,
    }
}
