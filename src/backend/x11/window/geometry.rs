//! 【X11普通窗口】【窗口几何】优先交给窗口管理器移动缩放，无管理器时保留各方向边界语义

use super::*;

pub(super) struct Drag {
    pointer: (i16, i16),
    position: (i16, i16),
    size: (u16, u16),
    edge: Option<WindowEdge>,
}

impl Node {
    /// 【X11普通窗口】【根坐标同步】无参数，取得客户区在根窗口的位置，返回协议错误
    pub(super) fn sync_position(&mut self) -> crate::AppResult {
        let display = &self.surface.display;
        let translated = display
            .connection
            .translate_coordinates(self.surface.window, display.screen().root, 0, 0)?
            .reply()?;
        self.surface.position = (translated.dst_x, translated.dst_y);
        Ok(())
    }

    /// 【X11普通窗口】【移动缩放开始】按action请求管理器拖动，缺少协议时保存起始几何，返回协议错误
    pub(super) fn begin_drag(&mut self, action: WindowAction) -> crate::AppResult {
        self.sync_position()?;
        let display = &self.surface.display;
        let pointer = display
            .connection
            .query_pointer(display.screen().root)?
            .reply()?;
        let edge = match action {
            WindowAction::Move => None,
            WindowAction::ResizeEdge(edge) => Some(edge),
            _ => Some(WindowEdge::BottomRight),
        };
        let message = display.atom("_NET_WM_MOVERESIZE")?;
        if display
            .property(display.screen().root, "_NET_SUPPORTED")?
            .contains(&message)
        {
            let direction = match edge {
                Some(WindowEdge::TopLeft) => 0,
                Some(WindowEdge::Top) => 1,
                Some(WindowEdge::TopRight) => 2,
                Some(WindowEdge::Right) => 3,
                Some(WindowEdge::BottomRight) => 4,
                Some(WindowEdge::Bottom) => 5,
                Some(WindowEdge::BottomLeft) => 6,
                Some(WindowEdge::Left) => 7,
                None => 8,
            };
            display.connection.ungrab_pointer(x11rb::CURRENT_TIME)?;
            display.connection.send_event(
                false,
                display.screen().root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ClientMessageEvent::new(
                    32,
                    self.surface.window,
                    message,
                    [
                        pointer.root_x as u32,
                        pointer.root_y as u32,
                        direction,
                        1,
                        1,
                    ],
                ),
            )?;
            display.connection.flush()?;
        } else {
            self.drag = Some(Drag {
                pointer: (pointer.root_x, pointer.root_y),
                position: self.surface.position,
                size: self.surface.size,
                edge,
            });
        }
        Ok(())
    }

    /// 【X11普通窗口】【移动缩放推进】按position更新无管理器拖动，保持对边及最小尺寸，返回协议错误
    pub(super) fn drag_to(&mut self, position: (i16, i16)) -> crate::AppResult {
        let Some(drag) = &self.drag else {
            return Ok(());
        };
        let delta = (
            i32::from(position.0) - i32::from(drag.pointer.0),
            i32::from(position.1) - i32::from(drag.pointer.1),
        );
        let (left, top, width, height) = bounds(
            drag.position,
            drag.size,
            delta,
            drag.edge,
            self.spec.minimum,
        );
        self.surface
            .display
            .connection
            .configure_window(
                self.surface.window,
                &ConfigureWindowAux::new()
                    .x(left)
                    .y(top)
                    .width(width)
                    .height(height),
            )?
            .check()?;
        Ok(())
    }
}

/// 【X11普通窗口】【边界计算】按起点、尺寸、位移、边与最小尺寸返回客户区位置和尺寸
fn bounds(
    position: (i16, i16),
    size: (u16, u16),
    delta: (i32, i32),
    edge: Option<WindowEdge>,
    minimum: (u32, u32),
) -> (i32, i32, u32, u32) {
    let (mut left, mut top) = (i32::from(position.0), i32::from(position.1));
    let (mut width, mut height) = (u32::from(size.0), u32::from(size.1));
    let Some(edge) = edge else {
        return (left + delta.0, top + delta.1, width, height);
    };
    let from_left = matches!(
        edge,
        WindowEdge::Left | WindowEdge::TopLeft | WindowEdge::BottomLeft
    );
    let from_top = matches!(
        edge,
        WindowEdge::Top | WindowEdge::TopLeft | WindowEdge::TopRight
    );
    if !matches!(edge, WindowEdge::Top | WindowEdge::Bottom) {
        width = (size.0 as i32 + if from_left { -delta.0 } else { delta.0 })
            .clamp(minimum.0 as i32, u16::MAX as i32) as u32;
        if from_left {
            left += i32::from(size.0) - width as i32;
        }
    }
    if !matches!(edge, WindowEdge::Left | WindowEdge::Right) {
        height = (size.1 as i32 + if from_top { -delta.1 } else { delta.1 })
            .clamp(minimum.1 as i32, u16::MAX as i32) as u32;
        if from_top {
            top += i32::from(size.1) - height as i32;
        }
    }
    (left, top, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【X11普通窗口】【边界回归】验证左上角缩放保持对边，最小尺寸钳制不移动对边，无参数
    #[test]
    fn opposite_edges_remain_fixed_when_resizing_or_clamping() {
        assert_eq!(
            bounds(
                (100, 80),
                (400, 300),
                (50, 30),
                Some(WindowEdge::TopLeft),
                (100, 100)
            ),
            (150, 110, 350, 270)
        );
        assert_eq!(
            bounds(
                (100, 80),
                (400, 300),
                (500, 500),
                Some(WindowEdge::TopLeft),
                (100, 100)
            ),
            (400, 280, 100, 100)
        );
        assert_eq!(
            bounds(
                (100, 80),
                (400, 300),
                (20, 40),
                Some(WindowEdge::Right),
                (100, 100)
            ),
            (100, 80, 420, 300)
        );
        assert_eq!(
            bounds((100, 80), (400, 300), (-20, 40), None, (100, 100)),
            (80, 120, 400, 300)
        );
    }
}
