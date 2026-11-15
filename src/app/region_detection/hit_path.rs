//! 【截图控件识别】【层级状态】最深组件默认优先，显式滚轮范围在兼容补充路径中保持不变

use crate::features::region_detection::PixelRect;

#[derive(Default)]
pub(super) struct HitPath {
    point: Option<(i32, i32)>,
    window: Option<usize>,
    path: Vec<PixelRect>,
    index: usize,
    explicit: bool,
    window_only: bool,
    anchor: Option<(i32, i32)>,
    wheel: f64,
}
impl HitPath {
    /// 【截图控件识别】【指针目标】按point与window更新窗口归属，返回是否需要新查询，移动超出六像素后恢复默认层级
    pub fn target(&mut self, point: (i32, i32), window: Option<(usize, PixelRect)>) -> bool {
        let changed = self.point != Some(point) || self.window != window.map(|value| value.0);
        if !changed {
            return false;
        }
        let same_window = self.window == window.map(|value| value.0)
            && self.path.last().copied() == window.map(|value| value.1);
        let leaf_hit = self.path.first().is_some_and(|rect| rect.contains(point));
        if !same_window || !leaf_hit {
            self.path = window.map_or_else(Vec::new, |value| vec![value.1]);
            self.index = 0;
            self.explicit = false;
            self.anchor = None;
            self.wheel = 0.0;
        } else if self.anchor.is_some_and(|anchor| {
            (f64::from(point.0) - f64::from(anchor.0))
                .hypot(f64::from(point.1) - f64::from(anchor.1))
                > 6.0
        }) {
            self.index = 0;
            self.explicit = false;
            self.anchor = None;
            self.wheel = 0.0;
        }
        self.point = Some(point);
        self.window = window.map(|value| value.0);
        true
    }

    /// 【截图控件识别】【结果合并】按point、path与refined接收当前目标结果，返回路径是否改变，补充路径必须包含原有祖先顺序
    pub fn apply(&mut self, point: (i32, i32), path: Vec<PixelRect>, refined: bool) -> bool {
        if self.point != Some(point) || path == self.path {
            return false;
        }
        let Some(bounds) = self.path.last().copied() else {
            return false;
        };
        if path.last() != Some(&bounds)
            || path.iter().any(|rect| {
                rect.right <= rect.left
                    || rect.bottom <= rect.top
                    || !rect.contains(point)
                    || !bounds.contains_rect(*rect)
            })
        {
            return false;
        }
        if refined {
            let mut matched = 0;
            for rect in &path {
                if self.path.get(matched) == Some(rect) {
                    matched += 1;
                }
            }
            if matched != self.path.len() {
                return false;
            }
        } else if self.path.ends_with(&path) {
            // 1. 【截图控件识别】【反向结果】同一命中位置的浅层结果不能抹除已经确认的更深路径
            return false;
        }
        let selected = self.current();
        self.path = path;
        self.index = if self.explicit {
            selected
                .and_then(|selected| self.path.iter().position(|rect| *rect == selected))
                .unwrap_or(0)
        } else {
            0
        };
        true
    }

    /// 【截图控件识别】【目标切换】无参数，切换整窗与组件模式，无返回值，整窗模式不依赖组件查询完成
    pub fn toggle(&mut self) {
        self.window_only = !self.window_only;
        self.index = 0;
        self.explicit = false;
        self.anchor = None;
        self.wheel = 0.0;
    }
    /// 【截图控件识别】【整窗模式】无参数，返回是否只选择窗口边界
    pub fn window_only(&self) -> bool {
        self.window_only
    }
    /// 【截图控件识别】【当前指针】无参数，返回当前物理目标位置，尚未悬停时返回空值
    #[cfg(target_os = "windows")]
    pub fn point(&self) -> Option<(i32, i32)> {
        self.point
    }
    /// 【截图控件识别】【原生优先】无参数，返回是否已有组件或用户显式选择了窗口，单一默认窗口允许图像回退
    pub fn native_selection(&self) -> bool {
        self.window_only || self.explicit || self.path.len() > 1
    }
    /// 【截图控件识别】【当前范围】无参数，返回当前组件或整窗，尚未命中窗口时返回空值
    pub fn current(&self) -> Option<PixelRect> {
        if self.window_only {
            self.path.last().copied()
        } else {
            self.path.get(self.index).copied()
        }
    }
    /// 【截图控件识别】【滚轮层级】按delta累积一百二十单位切换一层，返回当前范围，端点不循环，零值和整窗模式不改变目标
    pub fn step(&mut self, delta: f64) -> Option<PixelRect> {
        if !delta.is_finite() || self.window_only || self.path.is_empty() {
            return self.current();
        }
        let maximum = self.path.len() as f64 * 120.0;
        self.wheel += delta.clamp(-maximum, maximum);
        let steps = (self.wheel / 120.0).trunc();
        if steps != 0.0 {
            self.wheel %= 120.0;
            self.index =
                (self.index as f64 + steps).clamp(0.0, (self.path.len() - 1) as f64) as usize;
            self.explicit = true;
            self.anchor = self.point;
        }
        self.current()
    }
}

#[cfg(test)]
#[path = "hit_path_tests.rs"]
mod tests;
