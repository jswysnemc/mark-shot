//! 【多选区截图】【区域运算】使用互不重叠的矩形表示选区并集，保留区域间的空白和内部孔洞

use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Operation {
    Add,
    Subtract,
}

/// 【多选区截图】【矩形差集】从 source 去除 cut，向 result 追加仍有面积的部分
fn subtract(source: Rect, cut: Rect, result: &mut Vec<Rect>) {
    let Some(hit) = source.intersect(&cut).filter(|rect| !rect.is_empty()) else {
        result.push(source);
        return;
    };
    for (left, top, right, bottom) in [
        (source.left(), source.top(), source.right(), hit.top()),
        (source.left(), hit.bottom(), source.right(), source.bottom()),
        (source.left(), hit.top(), hit.left(), hit.bottom()),
        (hit.right(), hit.top(), source.right(), hit.bottom()),
    ] {
        if let Some(piece) =
            Rect::from_ltrb(left, top, right, bottom).filter(|rect| !rect.is_empty())
        {
            result.push(piece);
        }
    }
}

/// 【多选区截图】【选区合成】按 operation 对已有区域和 rect 求并集或差集，返回新的不重叠矩形集合
pub(super) fn apply(regions: &[Rect], operation: Operation, rect: Rect) -> Vec<Rect> {
    if operation == Operation::Add {
        let mut additions = vec![rect];
        for &existing in regions {
            let mut next = Vec::new();
            for piece in additions {
                subtract(piece, existing, &mut next);
            }
            additions = next;
            if additions.is_empty() {
                break;
            }
        }
        let mut result = regions.to_vec();
        result.extend(additions);
        return result;
    }
    let mut result = Vec::new();
    for &source in regions {
        subtract(source, rect, &mut result);
    }
    result
}

/// 【多选区截图】【取景范围】返回覆盖 regions 的最小矩形，空选区不产生导出范围
pub(crate) fn bounds(regions: &[Rect]) -> Option<Rect> {
    regions
        .iter()
        .copied()
        .reduce(|first, next| first.join(&next).unwrap_or(first))
}
