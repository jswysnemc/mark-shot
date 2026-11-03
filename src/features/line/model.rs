//! 【线条】【骨架模型】保存两个端点、有限控制点和独立线型，不保存界面或撤销状态

use serde::{Deserialize, Serialize};
use tiny_skia::{LineCap, LineJoin, Stroke, StrokeDash};

pub type Point = (f32, f32);
pub const MAX_CONTROLS: usize = 128;
pub const WIDTH: (f32, f32) = (1.0, 72.0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(usize)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl LineStyle {
    pub const ALL: [Self; 3] = [Self::Solid, Self::Dashed, Self::Dotted];

    /// 【线条】【线型标识】返回当前线型的稳定配置名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Dashed => "dashed",
            Self::Dotted => "dotted",
        }
    }

    /// 【线条】【线型说明】返回当前线型的紧凑面板标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Dashed => "Dashed",
            Self::Dotted => "Dotted",
        }
    }

    /// 【线条】【线型读取】把 value 转为稳定线型，兼容原版编号；返回线型或空值
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "solid" | "0" => Some(Self::Solid),
            "dashed" | "dash" | "1" => Some(Self::Dashed),
            "dotted" | "dot" | "2" => Some(Self::Dotted),
            _ => None,
        }
    }

    /// 【线条】【描边参数】用 width 构造圆角端点和连接的描边；返回参数，线型长度随笔宽变化
    pub fn stroke(self, width: f32) -> Stroke {
        Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            dash: match self {
                Self::Solid => None,
                Self::Dashed => StrokeDash::new(vec![4.0 * width, 2.0 * width], 0.0),
                Self::Dotted => StrokeDash::new(vec![0.01 * width, 2.5 * width], 0.0),
            },
            ..Stroke::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineSkeleton {
    pub start: Point,
    pub end: Point,
    pub controls: Vec<Point>,
    pub style_scale: f32,
}

impl LineSkeleton {
    /// 【线条】【骨架创建】以 start、end 创建无控制点骨架；坐标非法时返回空值，不分配曲线缓存
    pub fn new(start: Point, end: Point) -> Option<Self> {
        let skeleton = Self {
            start,
            end,
            controls: Vec::new(),
            style_scale: 1.0,
        };
        skeleton.valid().then_some(skeleton)
    }

    /// 【线条】【骨架校验】返回端点、控制点、绘制比例及容量是否有效
    pub fn valid(&self) -> bool {
        self.style_scale.is_finite()
            && self.style_scale > 0.0
            && self.style_scale <= 1_000_000.0
            && self.controls.len() <= MAX_CONTROLS
            && self.points().all(valid_point)
    }

    /// 【线条】【点序读取】返回全部端点和控制点，端点仍占原版索引零和一
    pub fn points(&self) -> impl Iterator<Item = Point> + '_ {
        [self.start, self.end]
            .into_iter()
            .chain(self.controls.iter().copied())
    }

    /// 【线条】【定点读取】按原版 index 读取端点或控制点；越界时返回空值
    pub fn point(&self, index: usize) -> Option<Point> {
        match index {
            0 => Some(self.start),
            1 => Some(self.end),
            _ => self.controls.get(index - 2).copied(),
        }
    }

    /// 【线条】【定点更新】将原版 index 对应定点改为 point；非法坐标或索引不修改，返回是否成功
    pub fn set_point(&mut self, index: usize, point: Point) -> bool {
        if !valid_point(point) {
            return false;
        }
        let target = match index {
            0 => Some(&mut self.start),
            1 => Some(&mut self.end),
            _ => self.controls.get_mut(index - 2),
        };
        let Some(target) = target else {
            return false;
        };
        *target = point;
        true
    }

    /// 【线条】【插入控制点】在原版 index 处插入 point；容量不足、索引或坐标非法时拒绝，返回是否成功
    pub fn insert_control(&mut self, index: usize, point: Point) -> bool {
        let Some(index) = index.checked_sub(2) else {
            return false;
        };
        if self.controls.len() >= MAX_CONTROLS || index > self.controls.len() || !valid_point(point)
        {
            return false;
        }
        self.controls.insert(index, point);
        true
    }

    /// 【线条】【删除控制点】删除原版 index 指定控制点，不允许删除两个端点；返回是否成功
    pub fn remove_control(&mut self, index: usize) -> bool {
        let Some(index) = index
            .checked_sub(2)
            .filter(|index| *index < self.controls.len())
        else {
            return false;
        };
        self.controls.remove(index);
        true
    }

    /// 【线条】【坐标映射】用 map 变换全部定点；返回新骨架，不改变线型比例或原骨架
    pub fn mapped(&self, mut map: impl FnMut(Point) -> Point) -> Self {
        Self {
            start: map(self.start),
            end: map(self.end),
            controls: self.controls.iter().copied().map(map).collect(),
            style_scale: self.style_scale,
        }
    }

    /// 【线条】【原图比例】按 scale 同时缩放定点和绝对样式尺寸；返回新骨架
    pub fn scaled(&self, scale: f32) -> Self {
        let mut result = self.mapped(|point| (point.0 * scale, point.1 * scale));
        result.style_scale *= scale.abs();
        result
    }
}

/// 【线条】【坐标校验】判断 point 是否为可安全绘制的有限坐标；返回布尔值
fn valid_point(point: Point) -> bool {
    point.0.is_finite() && point.1.is_finite() && point.0.abs() <= 1e9 && point.1.abs() <= 1e9
}

/// 【线条】【笔宽约束】将 width 约束到 style_scale 对应的独立范围；返回有限笔宽，非法值使用三像素默认值
pub fn clamped_width(width: f32, style_scale: f32) -> f32 {
    let scale = if style_scale.is_finite() && style_scale > 0.0 {
        style_scale.min(1_000_000.0)
    } else {
        1.0
    };
    let width = if width.is_finite() {
        width
    } else {
        3.0 * scale
    };
    width.clamp(WIDTH.0 * scale, WIDTH.1 * scale)
}
