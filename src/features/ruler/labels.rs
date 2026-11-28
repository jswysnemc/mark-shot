//! 【尺子】【说明入口】组合完整说明请求和窗口布局，公共数据与几何由对应模块提供

use super::Measurement;
use crate::editor::canvas_navigation::transform::ViewTransform;
use cosmic_text::FontSystem;
use tiny_skia::Rect;

pub use super::geometry::angle_arc;
pub use super::label_model::{InfoLabel, LabelRequest};

/// 【尺子】【说明准备】按 measurement 和 width 返回排版请求，bounds 留给布局阶段，度量使用定向尺子字体
pub fn prepare(
    measurement: Measurement,
    width: f32,
    _bounds: Rect,
    _fonts: &mut FontSystem,
) -> Option<LabelRequest> {
    super::label_model::prepare(measurement, width, Default::default())
}

/// 【尺子】【浮动范围平移】按rect、bounds和gap返回原版完整范围，只平移位置，不改变标签尺寸
pub fn clamped(rect: Rect, bounds: Rect, gap: f32) -> Rect {
    super::label_geometry::clamped(rect, bounds, gap)
}

/// 【尺子】【全部说明布局】按 requests 和 bounds 返回角度优先、长度尺内优先的完整说明
pub fn layout(requests: &[LabelRequest], bounds: Rect) -> Vec<InfoLabel> {
    super::label_geometry::layout(requests, bounds, std::iter::empty())
}

/// 【尺子】【导出说明集合】按rulers与bounds生成当前绘制坐标中的完整说明，fonts保留字体上下文，返回布局列表
pub fn prepare_all(
    rulers: impl Iterator<Item = (Measurement, f32)>,
    bounds: Rect,
    _fonts: &mut FontSystem,
) -> Vec<InfoLabel> {
    prepare_in_projection(rulers, bounds, Default::default())
}

/// 【尺子】【窗口说明集合】按rulers、bounds和projection生成当前窗口中的说明，再返回稳定坐标，不修改源图测量
pub(crate) fn prepare_in_projection(
    rulers: impl Iterator<Item = (Measurement, f32)>,
    bounds: Rect,
    projection: ViewTransform,
) -> Vec<InfoLabel> {
    let mut requests = Vec::new();
    let mut unlabelled = Vec::new();
    for (measurement, width) in rulers {
        if measurement.mode() == super::RulerMode::Length
            && measurement.style == super::RulerStyle::Standard
            && measurement.readouts == Default::default()
        {
            if measurement.measures() {
                unlabelled.push((measurement, width));
            }
        } else if let Some(request) = super::label_model::prepare(measurement, width, projection) {
            requests.push(request);
        }
    }
    // 1. 【尺子】【刻度避让】未启用说明的尺身仍参与避让，仅在存在可见长度说明时计算其刻度范围
    let obstacles = unlabelled.into_iter().filter_map(|(measurement, width)| {
        super::label_geometry::scale(measurement, width, bounds, projection)
    });
    super::label_geometry::layout(&requests, bounds, obstacles)
}

/// 【尺子】【数字边界约束】按rect、tick和bounds返回范围内刻度的完整数字范围，范围外刻度保持位置
pub fn number_rect(rect: Rect, tick: super::Point, bounds: Option<Rect>) -> Rect {
    match bounds {
        Some(bounds) if super::geometry::contains(bounds, tick) => clamped(rect, bounds, 0.0),
        _ => rect,
    }
}
