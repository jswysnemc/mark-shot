//! 【长图拼接】【重叠证据】位移使用分散正文证据，接缝使用连续清晰段，避免局部更新截断整段匹配

use crate::{edge_occlusion, long_image::LongImage};
use shot_image::Image;
use std::ops::Range;

pub(crate) struct Overlap {
    pub clean_rows: i32,
    pub textured_rows: i32,
    pub seam: Range<i32>,
}

/// 【长图拼接】【重叠扫描】按 long、frame、position、span 和 range 返回清晰行总数、纹理证据及最长干净接缝段，不复制像素
pub(crate) fn scan(
    long: &LongImage,
    frame: &Image,
    position: i32,
    span: (i32, i32),
    range: Range<i32>,
) -> Overlap {
    let mut result = Overlap {
        clean_rows: 0,
        textured_rows: 0,
        seam: range.start..range.start,
    };
    let mut run_start = range.start;
    let mut previous_clean = false;
    // 1. 【长图拼接】【分散正文】计数或头像变化只中断当前接缝段，不清除其他位置已确认的清晰正文
    for vertical in range {
        let row = vertical - position;
        let evidence = edge_occlusion::compare(long.row(vertical), frame.row(row), span);
        if evidence.clean {
            result.clean_rows += 1;
            // 2. 【长图拼接】【空白排除】垂直纹理必须来自两个连续匹配行，不能借用上一条未匹配文字证实空白行
            if evidence.detail > 1.0
                || (previous_clean
                    && row > 0
                    && edge_occlusion::compare(frame.row(row - 1), frame.row(row), span).difference
                        > 1.0)
            {
                result.textured_rows += 1;
            }
            if vertical + 1 - run_start > result.seam.end - result.seam.start {
                result.seam = run_start..vertical + 1;
            }
        } else {
            run_start = vertical + 1;
        }
        previous_clean = evidence.clean;
    }
    result
}
