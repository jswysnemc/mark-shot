//! 【形状标记】【曲线路径】使用三次曲线构造心形、牌面、书签和盾牌等轮廓

use tiny_skia::{Path, PathBuilder};

/// 【形状标记】【心形路径】构造顶部双瓣和底部尖角；返回闭合单位路径
pub(super) fn heart() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.50, 0.86);
    builder.cubic_to(0.18, 0.68, 0.08, 0.44, 0.20, 0.28);
    builder.cubic_to(0.28, 0.16, 0.42, 0.16, 0.50, 0.30);
    builder.cubic_to(0.58, 0.16, 0.72, 0.16, 0.80, 0.28);
    builder.cubic_to(0.92, 0.44, 0.82, 0.68, 0.50, 0.86);
    builder.close();
    builder.finish()
}

/// 【形状标记】【黑桃路径】构造两瓣、收腰和圆润底座；返回连续闭合单位路径
pub(super) fn spade() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.50, 0.05);
    builder.cubic_to(0.28, 0.18, 0.10, 0.34, 0.10, 0.50);
    builder.cubic_to(0.10, 0.64, 0.22, 0.74, 0.38, 0.68);
    builder.cubic_to(0.42, 0.74, 0.43, 0.80, 0.38, 0.86);
    builder.cubic_to(0.30, 0.90, 0.28, 0.95, 0.34, 0.97);
    builder.cubic_to(0.40, 0.99, 0.46, 0.97, 0.50, 0.93);
    builder.cubic_to(0.54, 0.97, 0.60, 0.99, 0.66, 0.97);
    builder.cubic_to(0.72, 0.95, 0.70, 0.90, 0.62, 0.86);
    builder.cubic_to(0.57, 0.80, 0.58, 0.74, 0.62, 0.68);
    builder.cubic_to(0.78, 0.74, 0.90, 0.64, 0.90, 0.50);
    builder.cubic_to(0.90, 0.34, 0.72, 0.18, 0.50, 0.05);
    builder.close();
    builder.finish()
}

/// 【形状标记】【梅花底座】构造圆润牌面底座，供多部件合并；返回闭合单位路径
pub(super) fn club_base() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.34, 0.84);
    builder.cubic_to(0.30, 0.88, 0.30, 0.94, 0.36, 0.97);
    builder.cubic_to(0.42, 0.99, 0.48, 0.99, 0.50, 0.99);
    builder.cubic_to(0.52, 0.99, 0.58, 0.99, 0.64, 0.97);
    builder.cubic_to(0.70, 0.94, 0.70, 0.88, 0.66, 0.84);
    builder.cubic_to(0.60, 0.80, 0.54, 0.80, 0.50, 0.80);
    builder.cubic_to(0.46, 0.80, 0.40, 0.80, 0.34, 0.84);
    builder.close();
    builder.finish()
}

/// 【形状标记】【书签路径】构造圆角带和底部凹口；返回闭合单位路径
pub(super) fn bookmark() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.30, 0.10);
    builder.cubic_to(0.30, 0.07, 0.33, 0.06, 0.36, 0.06);
    builder.line_to(0.64, 0.06);
    builder.cubic_to(0.67, 0.06, 0.70, 0.07, 0.70, 0.10);
    builder.line_to(0.70, 0.93);
    builder.line_to(0.50, 0.74);
    builder.line_to(0.30, 0.93);
    builder.close();
    builder.finish()
}

/// 【形状标记】【盾牌路径】构造平缓顶部、两侧收腰和底部圆尖；返回闭合单位路径
pub(super) fn shield() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.50, 0.05);
    builder.cubic_to(0.62, 0.12, 0.76, 0.15, 0.88, 0.16);
    builder.cubic_to(0.88, 0.48, 0.78, 0.76, 0.50, 0.95);
    builder.cubic_to(0.22, 0.76, 0.12, 0.48, 0.12, 0.16);
    builder.cubic_to(0.24, 0.15, 0.38, 0.12, 0.50, 0.05);
    builder.close();
    builder.finish()
}

/// 【形状标记】【感叹号竖条】构造上粗下细的圆角竖条；返回闭合单位路径
pub(super) fn exclamation_bar() -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.move_to(0.40, 0.12);
    builder.cubic_to(0.40, 0.06, 0.60, 0.06, 0.60, 0.12);
    builder.line_to(0.55, 0.60);
    builder.cubic_to(0.55, 0.66, 0.45, 0.66, 0.45, 0.60);
    builder.close();
    builder.finish()
}
