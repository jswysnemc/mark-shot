use tiny_skia::Rect;

pub struct SelectionState {
    pub aspect_ratio: f64,
    /// 【选区尺寸】【创建阶段】只标记范围创建手势，不把普通标注拖动当作选区创建
    pub creating: bool,
    pub zone: Option<Rect>,
    pub prev_zone: Option<Rect>,
    pub active_handle: SelectionHandle,
    pub drag_origin: Option<(f64, f64)>,
    pub selection_at_drag_start: Option<Rect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    AngleVertex,
    AngleFirstArm,
    AngleSecondArm,
    MagnifierSourceTopLeft,
    MagnifierSourceTop,
    MagnifierSourceTopRight,
    MagnifierSourceLeft,
    MagnifierSourceRight,
    MagnifierSourceBottomLeft,
    MagnifierSourceBottom,
    MagnifierSourceBottomRight,
    MagnifierSource,
    MagnifierLens,
    NumberTip,
    NumberBubble,
    /// 【线条】【骨架定点】零一起终点，二及以后为控制点
    LinePoint(usize),
    Rotate,
    Move,
    None,
}

impl SelectionHandle {
    /// 【交互控制点】【通用方向】把放大镜源框控制点映射到通用矩形方向；返回方向或移动方式，其他控制点保持不变
    pub fn generic_rect_handle(self) -> Self {
        match self {
            Self::MagnifierSourceTopLeft => Self::TopLeft,
            Self::MagnifierSourceTop => Self::Top,
            Self::MagnifierSourceTopRight => Self::TopRight,
            Self::MagnifierSourceLeft => Self::Left,
            Self::MagnifierSourceRight => Self::Right,
            Self::MagnifierSourceBottomLeft => Self::BottomLeft,
            Self::MagnifierSourceBottom => Self::Bottom,
            Self::MagnifierSourceBottomRight => Self::BottomRight,
            Self::MagnifierSource | Self::MagnifierLens => Self::Move,
            _ => self,
        }
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            aspect_ratio: 0.0,
            creating: false,
            zone: None,
            prev_zone: None,
            active_handle: SelectionHandle::None,
            drag_origin: None,
            selection_at_drag_start: None,
        }
    }
}

impl SelectionState {
    pub fn set_drag(
        &mut self,
        handle: SelectionHandle,
        origin: Option<(f64, f64)>,
        zone: Option<Rect>,
    ) {
        self.active_handle = handle;
        self.drag_origin = origin;
        self.selection_at_drag_start = zone;
    }
}

pub struct SelectionEdges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}
