//! 【多选区截图】【编辑事务】草稿始终从已确认区域重新计算，取消拖动不改变选区

pub(crate) mod export;
pub(crate) mod geometry;

use geometry::Operation;
use tiny_skia::Rect;

type Point = (f32, f32);

struct Drag {
    operation: Operation,
    start: Point,
    rect: Option<Rect>,
    preview: Vec<Rect>,
}

#[derive(Default)]
pub(crate) struct Session {
    pub(crate) confirmed: Vec<Rect>,
    drag: Option<Drag>,
    undo: Vec<Vec<Rect>>,
    redo: Vec<Vec<Rect>>,
}

impl Session {
    /// 【多选区截图】【当前遮罩】返回当前预览，静止时直接借用已确认区域
    pub(crate) fn visible(&self) -> &[Rect] {
        self.drag
            .as_ref()
            .map_or(&self.confirmed, |drag| &drag.preview)
    }

    /// 【多选区截图】【操作范围】返回当前拖选矩形，供局部重绘与边线绘制使用
    pub(crate) fn marquee(&self) -> Option<Rect> {
        self.drag.as_ref()?.rect
    }

    /// 【多选区截图】【手势归属】返回当前拖动的操作，非起始按钮释放不能提交草稿
    pub(crate) fn operation(&self) -> Option<Operation> {
        self.drag.as_ref().map(|drag| drag.operation)
    }

    /// 【多选区截图】【开始拖动】按 operation 和桌面坐标 start 建立手势，不修改已确认区域
    pub(crate) fn begin(&mut self, operation: Operation, start: Point) {
        self.drag = Some(Drag {
            operation,
            start,
            rect: None,
            preview: self.confirmed.clone(),
        });
    }

    /// 【多选区截图】【拖动预览】按桌面坐标 point 和允许范围 workspace 更新草稿；返回预览是否变化
    pub(crate) fn motion(&mut self, point: Point, workspace: Rect) -> bool {
        let Some(drag) = self.drag.as_mut() else {
            return false;
        };
        let (left, top) = drag.start;
        let rect = Rect::from_ltrb(
            left.min(point.0),
            top.min(point.1),
            left.max(point.0),
            top.max(point.1),
        )
        .and_then(|area| area.intersect(&workspace))
        .filter(|area| !area.is_empty());
        if rect == drag.rect {
            return false;
        }
        drag.rect = rect;
        drag.preview = rect.map_or_else(
            || self.confirmed.clone(),
            |area| geometry::apply(&self.confirmed, drag.operation, area),
        );
        true
    }

    /// 【多选区截图】【确认区域】提交当前预览，只有实际变化才增加一次撤销记录
    pub(crate) fn commit(&mut self) {
        if let Some(drag) = self.drag.take()
            && drag.preview != self.confirmed
        {
            self.undo
                .push(std::mem::replace(&mut self.confirmed, drag.preview));
            self.redo.clear();
        }
    }

    /// 【多选区截图】【放弃草稿】取消当前拖动，返回此前是否存在草稿
    pub(crate) fn cancel_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// 【多选区截图】【恢复操作】按 redo 选择重做或撤销，恢复上一次完整遮罩
    pub(crate) fn history(&mut self, redo: bool) {
        self.cancel_drag();
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        if let Some(regions) = from.pop() {
            to.push(std::mem::replace(&mut self.confirmed, regions));
        }
    }
}
