//! 【选区尺寸】【像素几何】使用双精度图像坐标约束拖动和精确尺寸，窗口坐标换算由应用负责

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// 【选区尺寸】【端点构造】按有限端点 a、b 返回归一化矩形，无效端点返回空矩形
    pub fn from_corners(a: (f64, f64), b: (f64, f64)) -> Self {
        if ![a.0, a.1, b.0, b.1].into_iter().all(f64::is_finite) {
            return Self::default();
        }
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            width: (a.0 - b.0).abs(),
            height: (a.1 - b.1).abs(),
        }
    }

    /// 【选区尺寸】【端点读取】返回当前矩形右下端点，无参数，不修正负尺寸
    pub fn bottom_right(self) -> (f64, f64) {
        (self.x + self.width, self.y + self.height)
    }

    /// 【选区尺寸】【有限检查】返回左上与右下端点是否有限，无参数
    pub fn finite(self) -> bool {
        let end = self.bottom_right();
        [self.x, self.y, end.0, end.1]
            .into_iter()
            .all(f64::is_finite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Other,
}

/// 【选区尺寸】【比例创建】按 anchor、pointer、ratio 和 image 创建图像内选区，非法比例返回自由矩形
pub fn create(anchor: (f64, f64), pointer: (f64, f64), ratio: f64, image: (i32, i32)) -> Rect {
    if ![anchor.0, anchor.1, pointer.0, pointer.1]
        .into_iter()
        .all(f64::is_finite)
    {
        return Rect::default();
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        return Rect::from_corners(anchor, pointer);
    }
    // 1. 【选区尺寸】【拖动方向】取两轴方向，并用较大投影决定初始宽高
    let sx = if pointer.0 >= anchor.0 { 1.0 } else { -1.0 };
    let sy = if pointer.1 >= anchor.1 { 1.0 } else { -1.0 };
    let mut width = (pointer.0 - anchor.0)
        .abs()
        .max((pointer.1 - anchor.1).abs() * ratio);
    let mut height = width / ratio;
    let available_width = if sx > 0.0 {
        f64::from(image.0) - anchor.0
    } else {
        anchor.0
    };
    let available_height = if sy > 0.0 {
        f64::from(image.1) - anchor.1
    } else {
        anchor.1
    };
    // 2. 【选区尺寸】【边界约束】依次限制宽高，每次都按当前比例联动另一侧
    if width > available_width {
        width = available_width.max(0.0);
        height = width / ratio;
    }
    if height > available_height {
        height = available_height.max(0.0);
        width = height * ratio;
    }
    Rect::from_corners(anchor, (anchor.0 + sx * width, anchor.1 + sy * height))
}

/// 【选区尺寸】【比例调整】按 handle、adjusted、ratio 和 image 约束 before，结果不足八像素时返回 before
pub fn adjust(before: Rect, handle: Handle, adjusted: Rect, ratio: f64, image: (i32, i32)) -> Rect {
    if !before.finite() || !adjusted.finite() || image.0 <= 0 || image.1 <= 0 {
        return before;
    }
    if !ratio.is_finite() || ratio <= 0.0 || handle == Handle::Other {
        return adjusted;
    }
    let br = before.bottom_right();
    let ar = adjusted.bottom_right();
    // 1. 【选区尺寸】【角点约束】固定对角端点，复用创建选区时的边界规则
    let corners = match handle {
        Handle::TopLeft => Some((br, (adjusted.x, adjusted.y))),
        Handle::TopRight => Some(((before.x, br.1), (ar.0, adjusted.y))),
        Handle::BottomLeft => Some(((br.0, before.y), (adjusted.x, ar.1))),
        Handle::BottomRight => Some(((before.x, before.y), ar)),
        _ => None,
    };
    let result = if let Some((anchor, pointer)) = corners {
        create(anchor, pointer, ratio, image)
    } else if matches!(handle, Handle::Left | Handle::Right) {
        // 2. 【选区尺寸】【水平边约束】保持另一侧水平端点，并围绕原竖向中心联动高度
        let mut width = adjusted.width;
        let mut height = width / ratio;
        if height > f64::from(image.1) {
            height = f64::from(image.1);
            width = height * ratio;
        }
        Rect {
            x: if handle == Handle::Right {
                before.x
            } else {
                br.0 - width
            },
            y: (before.y + before.height / 2.0 - height / 2.0)
                .clamp(0.0, (f64::from(image.1) - height).max(0.0)),
            width,
            height,
        }
    } else {
        // 3. 【选区尺寸】【竖向边约束】保持另一侧竖向端点，并围绕原水平中心联动宽度
        let mut height = adjusted.height;
        let mut width = height * ratio;
        if width > f64::from(image.0) {
            width = f64::from(image.0);
            height = width / ratio;
        }
        Rect {
            x: (before.x + before.width / 2.0 - width / 2.0)
                .clamp(0.0, (f64::from(image.0) - width).max(0.0)),
            y: if handle == Handle::Bottom {
                before.y
            } else {
                br.1 - height
            },
            width,
            height,
        }
    };
    if !result.finite() || result.width < 8.0 || result.height < 8.0 {
        before
    } else {
        result
    }
}

/// 【选区尺寸】【精确尺寸】按 requested 在 image 中调整 current，优先保持左上角，越界时平移，图像无效返回空矩形
pub fn resize(current: Rect, requested: (i32, i32), image: (i32, i32)) -> Rect {
    if !current.finite() || image.0 <= 0 || image.1 <= 0 {
        return Rect::default();
    }
    let width = f64::from(requested.0.clamp(1, image.0));
    let height = f64::from(requested.1.clamp(1, image.1));
    let current = Rect::from_corners((current.x, current.y), current.bottom_right());
    Rect {
        x: current.x.clamp(0.0, f64::from(image.0) - width),
        y: current.y.clamp(0.0, f64::from(image.1) - height),
        width,
        height,
    }
}
