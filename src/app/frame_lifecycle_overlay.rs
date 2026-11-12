//! 【帧生命周期测试】【像素后端】复用真实前景交付及背景合成，控制原生可写状态而不连接桌面

use crate::{
    AppResult,
    backend::{ScreenOverlay, native_backdrop, wayland::overlay::frame_pixels::FramePixels},
    image_source::ImageSource,
    types::{CursorIcon, DamageRect, Output, OverlayEvent},
};
use std::time::Duration;
use tiny_skia::Pixmap;

pub(super) struct Overlay {
    pub(super) ready: bool,
    pub(super) consumed: bool,
    pub(super) frames: Vec<Vec<u8>>,
    pub(super) sizes: Vec<(u32, u32)>,
    pub(super) sent: usize,
}

impl Overlay {
    /// 【帧生命周期测试】【后端构造】按canvases创建真实尺寸的原生字节，consumed选择消费或只读路径，返回测试后端
    pub(super) fn new(canvases: &[Pixmap], consumed: bool) -> Self {
        Self {
            ready: true,
            consumed,
            frames: canvases
                .iter()
                .map(|canvas| vec![0; canvas.data().len()])
                .collect(),
            sizes: canvases
                .iter()
                .map(|canvas| (canvas.width(), canvas.height()))
                .collect(),
            sent: 0,
        }
    }

    /// 【帧生命周期测试】【实际交付】按monitor、pixels、source和damage执行生产像素路径，返回结果，保留范围外原生内容
    fn submit(
        &mut self,
        monitor: usize,
        mut pixels: FramePixels<'_>,
        source: &ImageSource,
        damage: Option<DamageRect>,
    ) -> AppResult {
        if self.consumed {
            pixels.write_composited_to(
                &mut self.frames[monitor],
                source,
                self.sizes[monitor],
                damage,
            )?;
        } else {
            pixels.write_to(&mut self.frames[monitor], self.sizes[monitor], damage)?;
            native_backdrop::compose_bgra(
                source,
                &mut self.frames[monitor],
                self.sizes[monitor],
                damage,
            )?;
        }
        self.sent += 1;
        Ok(())
    }
}

impl ScreenOverlay for Overlay {
    /// 【帧生命周期测试】【场景交付】按 monitor/frame 执行各平台共用的软件场景合成，返回实际像素结果
    fn stage_capture_scene(
        &mut self,
        monitor: usize,
        frame: crate::types::capture_scene::Frame<'_>,
    ) -> AppResult {
        crate::backend::software_scene::compose(&frame, &mut self.frames[monitor], true)?;
        self.sent += 1;
        Ok(())
    }
    /// 【帧生命周期测试】【背景能力】无参数，返回支持原始网格背景合成
    fn supports_native_frame_background(&self) -> bool {
        true
    }
    /// 【帧生命周期测试】【消费能力】无参数，返回本次测试选择的源所有权模式
    fn supports_consumed_frame_background(&self) -> bool {
        self.consumed
    }
    /// 【帧生命周期测试】【背景模式】按enabled返回测试选定模式，不改变图像
    fn set_composited_background(&mut self, enabled: bool) -> bool {
        enabled
    }
    /// 【帧生命周期测试】【只读提交】按monitor、pixels、source和damage交付实际像素，返回结果，源始终保持不变
    fn stage_frame_with_background(
        &mut self,
        monitor: usize,
        pixels: &[u8],
        source: &ImageSource,
        damage: Option<DamageRect>,
    ) -> AppResult {
        self.submit(monitor, FramePixels::Borrowed(pixels), source, damage)
    }
    /// 【帧生命周期测试】【消费提交】按monitor、pixels、source和damage交付实际像素，返回结果，已发送源区间随后透明
    fn stage_consumed_frame_with_background(
        &mut self,
        monitor: usize,
        pixels: &mut Pixmap,
        source: &ImageSource,
        damage: Option<DamageRect>,
    ) -> AppResult {
        self.submit(monitor, FramePixels::Consumed(pixels), source, damage)
    }
    /// 【帧生命周期测试】【不透明交付】校验 pixels 已包含完整背景，按 monitor 与 damage 交付真实通道转换，返回结果
    fn stage_opaque_frame(
        &mut self,
        monitor: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> AppResult {
        let (left, top, width, height) =
            damage.unwrap_or((0, 0, self.sizes[monitor].0, self.sizes[monitor].1));
        let stride = self.sizes[monitor].0 as usize * 4;
        for row in top..top + height {
            let first = row as usize * stride + left as usize * 4;
            assert!(
                pixels[first..first + width as usize * 4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| pixel[3] == 255)
            );
        }
        FramePixels::Opaque(pixels).write_to(
            &mut self.frames[monitor],
            self.sizes[monitor],
            damage,
        )?;
        self.sent += 1;
        Ok(())
    }
    /// 【帧生命周期测试】【预分配】无参数，不需要测试桌面，无返回值
    fn prefault(&mut self) {}
    /// 【帧生命周期测试】【呈现】忽略targets，测试字节在构造时建立，返回成功
    fn present(&mut self, _targets: &[usize]) -> AppResult {
        Ok(())
    }
    /// 【帧生命周期测试】【无背景拒绝】按monitor、pixels和damage拒绝错误路径，返回错误
    fn stage_frame(
        &mut self,
        _monitor: usize,
        _pixels: &[u8],
        _damage: Option<DamageRect>,
    ) -> AppResult {
        Err("the lifecycle test requires actual native background staging".into())
    }
    /// 【帧生命周期测试】【刷新】无参数，无协议队列，返回成功
    fn flush(&mut self) -> AppResult {
        Ok(())
    }
    /// 【帧生命周期测试】【事件读取】忽略timeout，返回测试时钟事件
    fn next_event(&mut self, _timeout: Option<Duration>) -> AppResult<OverlayEvent> {
        Ok(OverlayEvent::Tick)
    }
    /// 【帧生命周期测试】【输出查询】无参数，返回空协议输出，编辑器自身保存真实布局
    fn discovered_outputs(&self) -> &[Output] {
        &[]
    }
    /// 【帧生命周期测试】【缩放支持】无参数，返回按真实像素大小处理分数比例
    fn hidpi(&self) -> bool {
        true
    }
    /// 【帧生命周期测试】【可写条件】忽略monitor，返回本次模拟的帧可写状态
    fn ready(&self, _monitor: usize) -> bool {
        self.ready
    }
    /// 【帧生命周期测试】【活动输出】无参数，返回第一个测试输出
    fn active_output(&mut self) -> Option<usize> {
        Some(0)
    }
    /// 【帧生命周期测试】【输出保持】忽略keep，不移除测试布局，返回成功
    fn retain_outputs(&mut self, _keep: &[usize]) -> AppResult {
        Ok(())
    }
    /// 【帧生命周期测试】【背景设置】忽略monitor、pixels与size，背景从每次实际提交获得，返回成功
    fn set_background(
        &mut self,
        _monitor: usize,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
    ) -> AppResult {
        Ok(())
    }
    /// 【帧生命周期测试】【指针形状】忽略icon，不连接桌面，无返回值
    fn set_cursor(&mut self, _icon: CursorIcon) {}
    /// 【帧生命周期测试】【背景清理】忽略monitor，测试无额外原生表面，返回成功
    fn clear_background(&mut self, _monitor: usize) -> AppResult {
        Ok(())
    }
    /// 【帧生命周期测试】【输入范围】忽略monitor和rect，不发送桌面输入，无返回值
    fn set_input_rect(&mut self, _monitor: usize, _rect: Option<(i32, i32, i32, i32)>) {}
}
