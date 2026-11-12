//! 【截图场景】【扫描行分段】边界判断按矩形段执行，透明前景块直接处理底图

use tiny_skia::IntRect;

/// 【截图场景】【固定遮罩段】按 front、background、target、shade 和 bgra 合成同一遮罩内像素，返回值为空
fn paint(front: &[u8], background: &[u8], target: &mut [u8], shade: [u8; 4], bgra: bool) {
    // 1. 【截图场景】【透明块复用】透明前景不参与逐像素叠加，选区内部直接复制原图
    for ((front, background), target) in front
        .chunks(256)
        .zip(background.chunks(256))
        .zip(target.chunks_mut(256))
    {
        let blank = front
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [0; 4]);
        if blank && shade[3] == 0 {
            if bgra {
                crate::utils::copy_swizzled(target, background);
            } else {
                target.copy_from_slice(background);
            }
            continue;
        }
        if blank {
            super::shade::paint(background, target, shade, bgra);
            continue;
        }
        for ((front, original), target) in front
            .as_chunks::<4>()
            .0
            .iter()
            .zip(background.as_chunks::<4>().0)
            .zip(target.as_chunks_mut::<4>().0)
        {
            let pixel = super::over(super::over(*front, shade), *original);
            *target = if bgra {
                [pixel[2], pixel[1], pixel[0], pixel[3]]
            } else {
                pixel
            };
        }
    }
}

/// 【截图场景】【扫描行组合】按像素切片、dirty、y、hole、hover 和颜色切分一行，bgra 指定输出通道，返回值为空
pub(super) fn row(
    front: &[u8],
    background: &[u8],
    target: &mut [u8],
    dirty: IntRect,
    y: i32,
    hole: Option<IntRect>,
    hover: Option<IntRect>,
    dim: [u8; 4],
    tint: [u8; 4],
    bgra: bool,
) {
    let mut edges = [dirty.right(); 6];
    edges[0] = dirty.left();
    let mut count = 2;
    for rectangle in [hole, hover].into_iter().flatten() {
        if y >= rectangle.top() && y < rectangle.bottom() {
            for edge in [rectangle.left(), rectangle.right()] {
                edges[count] = edge.clamp(dirty.left(), dirty.right());
                count += 1;
            }
        }
    }
    edges[..count].sort_unstable();
    // 2. 【截图场景】【段内恒定】选区和候选矩形最多产生五段，不在每个像素上重复比较坐标
    for edge in edges[..count].windows(2) {
        if edge[0] == edge[1] {
            continue;
        }
        let shade = if super::contains(hole, edge[0], y) {
            [0; 4]
        } else {
            dim
        };
        let shade = if super::contains(hover, edge[0], y) {
            super::over(tint, shade)
        } else {
            shade
        };
        let first = (edge[0] - dirty.left()) as usize * 4;
        let last = (edge[1] - dirty.left()) as usize * 4;
        paint(
            &front[first..last],
            &background[first..last],
            &mut target[first..last],
            shade,
            bgra,
        );
    }
}
