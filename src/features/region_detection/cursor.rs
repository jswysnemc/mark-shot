//! 【区域识别】【悬停会话】普通悬停与滚轮层级分别保留指针容差

use super::{Detector, PixelRect};

#[derive(Default)]
pub struct Cursor {
    current: Option<PixelRect>,
    anchor: Option<(i32, i32)>,
    manual_level: bool,
}

impl Cursor {
    /// 【区域识别】【悬停更新】按 detector 与 point 更新候选，返回矩形，普通移动三像素与手动层级六像素内保留原框
    pub fn hover(&mut self, detector: &Detector, point: (i32, i32)) -> PixelRect {
        let tolerance = if self.manual_level { 6.0 } else { 3.0 };
        if let (Some(current), Some(anchor)) = (self.current, self.anchor)
            && current.contains(point)
            && (point.0 as f32 - anchor.0 as f32).hypot(point.1 as f32 - anchor.1 as f32)
                <= tolerance
        {
            return current;
        }
        let rect = detector.region_at(point);
        self.current = Some(rect);
        self.anchor = Some(point);
        self.manual_level = false;
        rect
    }

    /// 【区域识别】【层级切换】按 detector、point 与 direction 切换一层，正值扩大负值缩小，返回矩形并保存手动身份
    pub fn step(&mut self, detector: &Detector, point: (i32, i32), direction: i32) -> PixelRect {
        let current = self.hover(detector, point);
        let rect = if direction > 0 {
            detector
                .adjacent(point, current, true)
                .unwrap_or(detector.bounds())
        } else if direction < 0 {
            detector.adjacent(point, current, false).unwrap_or(current)
        } else {
            current
        };
        self.current = Some(rect);
        self.anchor = Some(point);
        self.manual_level = true;
        rect
    }

    /// 【区域识别】【手动身份】无参数，返回当前候选是否来自滚轮层级切换
    pub fn manual_level(&self) -> bool {
        self.manual_level
    }

    /// 【区域识别】【当前候选】无参数，返回缓存矩形，没有悬停时返回空值
    pub fn current(&self) -> Option<PixelRect> {
        self.current
    }

    /// 【区域识别】【会话清理】无参数、无返回值，清空指针锚点、候选与手动层级
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
