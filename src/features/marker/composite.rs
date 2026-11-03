//! 【形状标记】【复合路径】组合多部件标记，保留空心区域和连续外轮廓

use tiny_skia::{Path, Rect};

use super::{boolean, curves, primitives as primitive};

/// 【形状标记】【交叉标记】合并两条圆角斜线；返回无重叠内部边界的单位路径
pub(super) fn cross() -> Option<Path> {
    boolean::unite(&[
        primitive::round_line(&[(0.24, 0.24), (0.76, 0.76)], 0.16)?,
        primitive::round_line(&[(0.76, 0.24), (0.24, 0.76)], 0.16)?,
    ])
}

/// 【形状标记】【梅花标记】合并三瓣、中心、圆角柄和曲线底座；返回连续单位路径
pub(super) fn club() -> Option<Path> {
    boolean::unite(&[
        primitive::circle(0.50, 0.28, 0.20)?,
        primitive::circle(0.33, 0.50, 0.20)?,
        primitive::circle(0.67, 0.50, 0.20)?,
        primitive::circle(0.50, 0.45, 0.12)?,
        primitive::round_line(&[(0.50, 0.52), (0.50, 0.80)], 0.11)?,
        curves::club_base()?,
    ])
}

/// 【形状标记】【禁止标记】组合中空圆环和圆角斜杠；保留原版描边结构，返回单位路径
pub(super) fn ban() -> Option<Path> {
    let ring = boolean::subtract(
        &primitive::circle(0.50, 0.50, 0.42)?,
        &primitive::circle(0.50, 0.50, 0.26)?,
    )?;
    let slash = primitive::round_line(&[(0.26, 0.74), (0.74, 0.26)], 0.14)?;
    boolean::join(&[ring, boolean::normalize(&slash)?])
}

/// 【形状标记】【新月标记】从大圆减去上移右移的小圆；返回单位路径
pub(super) fn crescent() -> Option<Path> {
    boolean::subtract(
        &primitive::circle(0.50, 0.50, 0.40)?,
        &primitive::circle(0.69, 0.39, 0.37)?,
    )
}

/// 【形状标记】【定位针】合并圆头和尖端再挖出中心孔；返回单位路径
pub(super) fn pin() -> Option<Path> {
    let outer = boolean::unite(&[
        primitive::circle(0.50, 0.38, 0.30)?,
        primitive::polygon(&[(0.50, 0.96), (0.27, 0.54), (0.73, 0.54)])?,
    ])?;
    boolean::subtract(&outer, &primitive::circle(0.50, 0.38, 0.115)?)
}

/// 【形状标记】【旗帜标记】合并圆角旗杆和三角旗面；返回单位路径
pub(super) fn flag() -> Option<Path> {
    boolean::unite(&[
        primitive::round_line(&[(0.24, 0.10), (0.24, 0.92)], 0.09)?,
        primitive::polygon(&[(0.28, 0.12), (0.86, 0.28), (0.28, 0.46)])?,
    ])
}

/// 【形状标记】【感叹号标记】组合渐窄竖条和圆点；返回包含两部分的单位路径
pub(super) fn exclamation() -> Option<Path> {
    boolean::unite(&[
        curves::exclamation_bar()?,
        primitive::circle(0.50, 0.84, 0.095)?,
    ])
}

/// 【形状标记】【对话气泡】合并圆角矩形和左下尾部；返回无内部重叠边界的单位路径
pub(super) fn speech_bubble() -> Option<Path> {
    boolean::unite(&[
        primitive::rounded_rectangle(Rect::from_xywh(0.08, 0.12, 0.84, 0.58)?, 0.14)?,
        primitive::polygon(&[(0.26, 0.66), (0.20, 0.92), (0.46, 0.69)])?,
    ])
}
