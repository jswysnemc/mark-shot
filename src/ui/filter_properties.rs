//! 【滤镜】【模式预览】复用紧凑选择组件，以实际滤镜算法绘制小型效果预览

use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

use crate::features::filter::{
    FilterMode, RegionFilter,
    raster::{FilterKey, Raster},
};
use crate::renderer::{FrozenImage, Painter};
use crate::ui::choice_popover::{ChoiceOption, ChoicePopover, draw_choice_popover};

/// 【滤镜】【面板创建】返回关闭的五模式选择面板，不预先分配界面像素缓存
pub fn popover() -> ChoicePopover {
    ChoicePopover::new(
        "Filter effect",
        FilterMode::ALL
            .map(|mode| ChoiceOption {
                label: mode.label(),
            })
            .to_vec(),
    )
}

/// 【滤镜】【真实效果预览】在 painter 的 rect 内绘制 mode 的小型图像；color 控制透明度，不使用占位字符，无返回值
pub fn preview(painter: &mut Painter<'_>, mode: FilterMode, rect: Rect, color: Color) {
    let Some(mut source) = Pixmap::new(20, 16) else {
        return;
    };
    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = if x < 10 {
                [30 + y as u8 * 6, 85 + x as u8 * 8, 90, 255]
            } else {
                [220 - y as u8 * 5, 180, 55 + (x - 10) as u8 * 7, 255]
            };
            let offset = (y as usize * source.width() as usize + x as usize) * 4;
            source.data_mut()[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
    let image = FrozenImage::image(&source, Rect::from_xywh(0.0, 0.0, 20.0, 16.0).unwrap());
    let filter = RegionFilter::new((0.0, 0.0), (20.0, 16.0), 4.0, mode).unwrap();
    let Some(key) = FilterKey::new(image, filter) else {
        return;
    };
    let Some(raster) = Raster::new(image, key) else {
        return;
    };
    let width = rect.width().min(26.0);
    let height = rect.height().min(width * 0.8);
    let start = painter.to_px(
        rect.left() + (rect.width() - width) / 2.0,
        rect.top() + (rect.height() - height) / 2.0,
    );
    let scale = painter.scale();
    painter.canvas().draw_pixmap(
        0,
        0,
        raster.pixels.as_ref(),
        &PixmapPaint {
            opacity: color.alpha(),
            quality: if mode == FilterMode::Pixelate {
                FilterQuality::Nearest
            } else {
                FilterQuality::Bilinear
            },
            ..PixmapPaint::default()
        },
        Transform::from_row(
            width * scale / 20.0,
            0.0,
            0.0,
            height * scale / 16.0,
            start.0,
            start.1,
        ),
        None,
    );
}

/// 【滤镜】【面板绘制】将 panel 绘入 painter；标签、窄窗口布局和缓存由统一组件管理，无返回值
pub fn draw_popover(painter: &mut Painter<'_>, panel: &mut ChoicePopover) {
    draw_choice_popover(painter, panel, |painter, index, rect, color| {
        if let Some(mode) = FilterMode::ALL.get(index) {
            preview(painter, *mode, rect, color);
        }
    });
}
