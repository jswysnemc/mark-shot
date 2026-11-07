//! 【截图绘制】【单表面背景】只在已经恢复的损坏范围合成冻结背景，避免另一份整屏原生背景

use crate::{image_source::ImageSource, types::DamageRect};
use tiny_skia::{BlendMode, IntRect, Pixmap, PixmapPaint};

#[cfg(test)]
#[path = "opaque_backdrop_tests.rs"]
mod tests;

/// 【截图绘制】【背景合成】按source、canvas和damage将完整冻结背景置于现有前景下方，不修改其他像素，无返回值
pub(crate) fn compose(source: &ImageSource, canvas: &mut Pixmap, damage: Option<DamageRect>) {
    let (x, y, width, height) = damage.unwrap_or((0, 0, canvas.width(), canvas.height()));
    let Some(region) = IntRect::from_xywh(x as i32, y as i32, width, height) else {
        return;
    };
    let Some(region) =
        region.intersect(&IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap())
    else {
        return;
    };
    let stride = canvas.width() as usize * 4;
    let needs_background = (region.top()..region.bottom()).any(|row| {
        let start = row as usize * stride + region.left() as usize * 4;
        canvas.data()[start..start + region.width() as usize * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] < 255)
    });
    // 1. 【截图绘制】【不透明前景】导航或其他完整背景已经覆盖损坏范围时，背景不会参与混合，不再读取冻结文件
    if !needs_background {
        return;
    }
    source.draw_region(
        canvas,
        &PixmapPaint {
            blend_mode: BlendMode::DestinationOver,
            ..PixmapPaint::default()
        },
        region,
    );
}
