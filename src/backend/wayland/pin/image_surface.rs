//! 【贴图】【图像子表面】原图只上传一次，移动与缩放由合成器处理，前景保留边框、选择及提示

use super::runtime::Pin;
use crate::{
    AppResult,
    backend::wayland::utils::shm_pool::{Buffer, Slot},
    features::pinned_image::placement::Placement,
};
use smithay_client_toolkit::compositor::Region;
use wayland_client::{
    QueueHandle,
    protocol::{wl_shm, wl_subsurface::WlSubsurface, wl_surface::WlSurface},
};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;

pub(super) struct ImageSource {
    slot: Slot,
    identity: (usize, u32, u32),
}

pub(super) struct ImageSurface {
    surface: WlSurface,
    role: WlSubsurface,
    viewport: WpViewport,
    buffer: Option<Buffer>,
    identity: Option<(usize, u32, u32)>,
    visible: bool,
}

impl ImageSurface {
    /// 【贴图】【子表面创建】按 parent 创建同步且无输入的背景，qh 绑定当前贴图队列，返回角色或协议错误
    pub(super) fn new(pin: &Pin, parent: &WlSurface, qh: &QueueHandle<Pin>) -> AppResult<Self> {
        let (role, surface) = pin
            .subcompositor
            .as_ref()
            .ok_or("missing pin subcompositor")?
            .create_subsurface(parent.clone(), qh);
        role.place_below(parent);
        let region = Region::new(&pin.compositor)?;
        surface.set_input_region(Some(region.wl_region()));
        let viewport = pin
            .viewporter
            .as_ref()
            .ok_or("missing pin viewporter")?
            .get_viewport(&surface, qh, ());
        Ok(Self {
            surface,
            role,
            viewport,
            buffer: None,
            identity: None,
            visible: false,
        })
    }

    /// 【贴图】【图像变换】按 source 和 placement 同步裁切及缩放，pool 只创建共享槽位别名，返回协议结果，不复制已有图像
    pub(super) fn update(
        &mut self,
        source: &ImageSource,
        placement: Placement,
        pool: &mut crate::backend::wayland::utils::shm_pool::SlotPool,
    ) -> AppResult {
        let (_, width, height) = source.identity;
        if self.identity != Some(source.identity) {
            self.buffer = Some(pool.create_buffer_in(
                &source.slot,
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888,
            )?);
            self.identity = Some(source.identity);
            self.visible = false;
        }
        let (x, y, w, h) = source_rect((width, height), placement);
        self.viewport.set_source(x, y, w, h);
        self.viewport.set_destination(
            placement.visible.width as i32,
            placement.visible.height as i32,
        );
        // 1. 【贴图】【像素复用】已经映射的缓冲保持附着，位置及视口更新不重新上传原图
        if !self.visible {
            self.buffer.as_ref().unwrap().attach_to(&self.surface)?;
            self.surface
                .damage_buffer(0, 0, width as i32, height as i32);
            self.visible = true;
        }
        self.surface.commit();
        Ok(())
    }

    /// 【贴图】【旧画面隐藏】无参数，暂停主表面图像以交给拖动预览，不销毁原图存储，返回空值
    pub(super) fn hide(&mut self) {
        if self.visible {
            self.surface.attach(None, 0, 0);
            self.surface.commit();
            self.visible = false;
            // 1. 【贴图】【重新附着】旧缓冲可能尚未释放，下一次显示创建同槽位的新别名
            self.identity = None;
        }
    }
}

impl Drop for ImageSurface {
    /// 【贴图】【子表面释放】无参数，按视口、角色、表面顺序释放协议对象，缓冲别名随后归还，无返回值
    fn drop(&mut self) {
        self.viewport.destroy();
        self.role.destroy();
        self.surface.destroy();
    }
}

/// 【贴图】【原图裁切】按 pixels 原始尺寸与 placement 可见几何返回视口源矩形，量化到协议精度并限制映射边界
fn source_rect(pixels: (u32, u32), placement: Placement) -> (f64, f64, f64, f64) {
    let axis = |pixels: u32, full: u32, visible: u32, offset: i32| {
        let factor = f64::from(pixels) / f64::from(full.max(1));
        let start = ((f64::from(-offset) * factor).max(0.0) * 256.0).floor() / 256.0;
        let end = ((f64::from(-offset) + f64::from(visible)) * factor).min(f64::from(pixels));
        let end = (end * 256.0).floor() / 256.0;
        (start, (end - start).max(1.0 / 256.0))
    };
    let (x, w) = axis(
        pixels.0,
        placement.logical.width,
        placement.visible.width,
        placement.offset.0,
    );
    let (y, h) = axis(
        pixels.1,
        placement.logical.height,
        placement.visible.height,
        placement.offset.1,
    );
    (x, y, w, h)
}

impl Pin {
    /// 【贴图】【背景能力】无参数，返回是否可以用同步图像子表面，缺少协议时继续软件绘制
    pub(super) fn composited_image(&self) -> bool {
        self.subcompositor.is_some() && self.viewporter.is_some()
    }

    /// 【贴图】【原图上传】无参数，按原图身份更新共享图像存储，返回分配或映射错误；已有图像不复制
    pub(super) fn upload_image(&mut self) -> AppResult {
        if !self.composited_image() {
            return Ok(());
        }
        let image = &self.image.image;
        let identity = (
            image.data().as_ptr() as usize,
            image.width(),
            image.height(),
        );
        if self
            .image_source
            .as_ref()
            .is_some_and(|source| source.identity == identity)
        {
            return Ok(());
        }
        // 1. 【贴图】【独立原件】共享像素只供合成器显示，复制和保存继续读取不变的 RGBA 原图
        let slot = self.pool.new_slot(image.data().len())?;
        crate::utils::copy_swizzled(
            &mut self.pool.raw_data_mut(&slot)[..image.data().len()],
            image.data(),
        );
        self.image_source = Some(ImageSource { slot, identity });
        Ok(())
    }

    /// 【贴图】【主图同步】按 qh 映射当前主图或交给预览，返回协议结果，前景缓冲保持独立
    pub(super) fn update_main_image(&mut self, qh: &QueueHandle<Self>, hidden: bool) -> AppResult {
        if !self.composited_image() {
            return Ok(());
        }
        self.upload_image()?;
        if self.image_surface.is_none() {
            self.image_surface = Some(ImageSurface::new(self, self.shell.surface(), qh)?);
        }
        let surface = self.image_surface.as_mut().unwrap();
        if hidden {
            surface.hide();
        } else {
            surface.update(
                self.image_source.as_ref().unwrap(),
                self.placement,
                &mut self.pool,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::pinned_image::placement::{Geometry, place};

    /// 【贴图测试】【原始裁切】无参数，移动及超大缩放的源矩形不越界，并保持可见区域对应原图比例
    #[test]
    fn source_crops_preserve_original_coordinates() {
        let screen = Geometry::new((0, 0), (1280, 800));
        let p = place(Geometry::new((-100, -50), (2560, 1600)), screen);
        assert_eq!(source_rect((2560, 1600), p), (100.0, 50.0, 1280.0, 800.0));
        let p = place(Geometry::new((50, 50), (5120, 3200)), screen);
        let (x, y, w, h) = source_rect((2560, 1600), p);
        assert_eq!((x, y), (0.0, 0.0));
        assert_eq!((w, h), (615.0, 375.0));
    }
}
