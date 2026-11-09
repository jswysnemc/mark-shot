#[cfg(any(target_os = "linux", test))]
mod capture_chain;
pub mod capture_session;
pub mod notify;
pub mod output_selection;
mod platform;
pub mod session;
pub mod task_wake;
#[cfg(target_os = "linux")]
pub mod wayland;
pub mod window;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod x11;
pub use platform::{
    initialize_capture, initialize_capture_with_cursor, initialize_clipboard, initialize_notifier,
    initialize_overlay, initialize_window_capture,
};
pub mod window_detection;

use crate::types::{Capture, CaptureResult, CursorIcon, DamageRect, Output, OverlayEvent};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const FRAME_FALLBACK: Duration = Duration::from_millis(150);

#[async_trait]
pub trait CaptureMethod: Send + Sync {
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>>;

    async fn prepare(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// 【持续捕获】【会话创建】按 outputs 创建固定目标的捕获会话，不支持时明确返回错误
    async fn open_session(
        &self,
        _outputs: &[Output],
    ) -> capture_session::Result<Box<dyn capture_session::CaptureSession>> {
        Err("this capture backend does not support continuous sessions".into())
    }

    /// 【Wayland】【窗口捕获】后端未提供精确窗口捕获时返回错误，不替换为整屏图片
    async fn capture_active_window(&self) -> Result<Capture, Box<dyn std::error::Error>> {
        Err("this backend does not support active-window capture".into())
    }
}

pub trait ClipboardProvider {
    fn copy_image_to_clipboard(&self, png_data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}

#[cfg(target_os = "linux")]
pub(crate) mod composited_pixels;
#[cfg(any(target_os = "windows", test))]
pub(crate) mod native_backdrop;

pub trait ScreenOverlay {
    /// 【图片编辑】【窗口角色】按 size 请求普通可调整窗口，返回后端是否支持；必须在 present 前调用
    fn configure_image_window(&mut self, _size: (u32, u32)) -> bool {
        false
    }
    /// 【图片编辑】【实际窗口】返回合成器确认的逻辑尺寸，非图片窗口返回空值
    fn image_window_size(&self) -> Option<(u32, u32)> {
        None
    }
    /// 【截图绘制】【原生背景能力】无参数，返回后端是否能够在原生缓冲中合成同网格冻结背景
    fn supports_native_frame_background(&self) -> bool {
        false
    }
    /// 【截图绘制】【前景消费能力】无参数，返回后端是否支持复制已交付字节后归还CPU前景，未交付字节必须保留
    fn supports_consumed_frame_background(&self) -> bool {
        false
    }
    /// 【截图绘制】【消费背景提交】按monitor_idx、pixels、background和damage交付并消费CPU前景，返回协议结果；调用方在下一帧按损坏区域重建前景
    fn stage_consumed_frame_with_background(
        &mut self,
        _monitor_idx: usize,
        _pixels: &mut tiny_skia::Pixmap,
        _background: &crate::image_source::ImageSource,
        _damage: Option<DamageRect>,
    ) -> crate::AppResult {
        Err("consumed native foreground staging is unavailable".into())
    }
    /// 【截图绘制】【原生背景提交】按monitor_idx、pixels、background和damage提交原始前景及冻结背景，返回协议结果；不支持的后端由调用方使用既有合成路径
    fn stage_frame_with_background(
        &mut self,
        _monitor_idx: usize,
        _pixels: &[u8],
        _background: &crate::image_source::ImageSource,
        _damage: Option<DamageRect>,
    ) -> crate::AppResult {
        Err("native frozen background staging is unavailable".into())
    }
    /// 【截图绘制】【背景模式】按enabled选择由调用方合成完整背景，返回实际启用状态，默认后端继续接收独立背景
    fn set_composited_background(&mut self, _enabled: bool) -> bool {
        false
    }
    /// 【主选择区】【协议启用】按需启用当前截图连接的主选择支持，无参数，返回是否具备协议
    fn enable_primary_selection(&mut self) -> bool {
        false
    }
    /// 【主选择区】【正文拥有】用当前按钮授权拥有 text，返回是否成功，不启动后台剪贴板进程
    fn set_primary_text(&mut self, _text: &str) -> bool {
        false
    }
    /// 【主选择区】【正文接收】按 id 与 limit 请求有界正文，无返回值，完成后投递 PrimaryText
    fn request_primary_text(&mut self, _id: u64, _limit: usize) {}
    /// 【主选择区】【接收取消】取消 id 对应过期请求，无返回值，不影响来源拥有权
    fn cancel_primary_text(&mut self, _id: u64) {}
    /// 【截图覆盖层】【任务通知】按需返回后台完成句柄，无参数；不支持通知的后端明确返回错误，不增加周期轮询
    fn task_wake(&mut self) -> crate::AppResult<task_wake::Wake> {
        Err("overlay task wake is unavailable".into())
    }
    /// 【输入法】【状态同步】把 snapshot 交给原生协议，空值停用正文输入；默认保留普通键盘后端，返回协议结果
    fn set_text_input(
        &mut self,
        _snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        Ok(())
    }
    /// 【原生弹层】【帧同步】按 frame 打开或更新菜单，空值关闭；返回是否提供原生弹层，默认实现供无表面的测试后端使用
    fn set_popup(
        &mut self,
        _frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
    /// 【截图覆盖层】【暂停显示】隐藏全部截图表面并释放输入抓取；保留原图和输出绑定，返回协议执行结果
    fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("overlay suspension is unavailable".into())
    }
    /// 【截图覆盖层】【恢复显示】在原输出重新创建表面；调用方恢复冻结背景，返回协议执行结果
    fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("overlay resumption is unavailable".into())
    }
    /// 【滚动截图】【输入角色】为 monitor_idx 释放独占键盘并移除其他输出表面；返回协议执行结果
    fn begin_scroll(&mut self, _monitor_idx: usize) -> crate::AppResult {
        Err("scroll capture requires an overlay with on-demand keyboard focus".into())
    }
    /// 【滚动截图】【面板配置】按 rect 和 paused 同步控制布局；普通窗口后端可以独立于输出大小呈现，无返回值
    fn configure_scroll_panel(
        &mut self,
        _rect: tiny_skia::Rect,
        _paused: bool,
    ) -> crate::AppResult {
        Ok(())
    }
    /// 【滚动截图】【窗口角色】返回当前控制面板是否使用普通窗口，无参数，不根据配置猜测协议能力
    fn is_scroll_window(&self) -> bool {
        false
    }
    /// 【滚动截图】【面板边界】返回 monitor_idx 控制表面的合成器尺寸限制，无限制时返回空值
    fn scroll_panel_limits(&self, _monitor_idx: usize) -> Option<(u32, u32)> {
        None
    }
    /// 【滚动截图】【帧隔离】核对 frame 与 region，必要时隐藏重叠控制窗口；返回是否可以接受该帧
    fn allow_scroll_frame(
        &mut self,
        _frame: &crate::types::MonitorFrame,
        _region: tiny_skia::Rect,
    ) -> crate::AppResult<bool> {
        Ok(true)
    }
    /// 【滚动截图】【控制可见性】返回是否因捕获区域重叠暂时隐藏了控制窗口，无参数
    fn scroll_controls_hidden(&self) -> bool {
        false
    }
    /// 【滚动截图】【输出绑定】按表面 monitor_idx 返回原输出身份；索引超界时返回空值
    fn surface_output(&self, monitor_idx: usize) -> Option<&Output> {
        self.discovered_outputs().get(monitor_idx)
    }
    fn prefault(&mut self);
    fn present(&mut self, targets: &[usize]) -> Result<(), Box<dyn std::error::Error>>;
    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// 【原生帧】【不透明提交】按 monitor_idx、pixels 和 damage 提交已包含背景的完整像素；默认沿用普通交付，返回协议结果
    fn stage_opaque_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        self.stage_frame(monitor_idx, pixels, damage)
    }
    /// 【滚动截图】【局部面板】按monitor_idx和frame提交控制像素与独立边线，返回协议结果，不要求调用方保留整屏画布
    fn stage_scroll_panel(
        &mut self,
        _monitor_idx: usize,
        _frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        Err("the overlay cannot present a local scroll panel".into())
    }
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    /// `None` waits for the compositor indefinitely, a duration wakes up with
    /// `OverlayEvent::Tick` once it runs out.
    fn next_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<OverlayEvent, Box<dyn std::error::Error>>;
    fn discovered_outputs(&self) -> &[Output];
    fn hidpi(&self) -> bool;
    /// true once compositor has shown the last frame
    /// or after FRAME_FALLBACK without an answer, to avoid monitor freezing
    fn ready(&self, monitor_idx: usize) -> bool;
    fn active_output(&mut self) -> Option<usize>;
    fn retain_outputs(&mut self, keep: &[usize]) -> Result<(), Box<dyn std::error::Error>>;
    fn set_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>>;
    /// 【冻结背景】【来源交付】按monitor_idx和source安装冻结背景，返回协议结果；默认后端兼容已有完整像素接口
    fn set_background_source(
        &mut self,
        monitor_idx: usize,
        source: &crate::image_source::ImageSource,
    ) -> crate::AppResult {
        self.set_background(monitor_idx, source.data(), source.width(), source.height())
    }
    fn set_cursor(&mut self, icon: CursorIcon);
    /// Drops the static snapshot subsurface on `monitor_idx` to expose the live screen beneath.
    fn clear_background(&mut self, monitor_idx: usize) -> Result<(), Box<dyn std::error::Error>>;
    /// Sets the input region on `monitor_idx` to `rect`, or allows all pointer events
    /// to fall through to windows beneath when `None`.
    fn set_input_rect(&mut self, monitor_idx: usize, rect: Option<(i32, i32, i32, i32)>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayShell {
    Auto,
    Window,
    Layer,
}

/// 【系统通知】【平台契约】交付通知内容并返回系统服务结果，与截图表面协议选择分离
#[async_trait]
pub trait Notifier {
    async fn notify(
        &self,
        n: &notify::Notification<'_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(target_os = "linux")]
pub use wayland::{clipboard, pin};
#[cfg(target_os = "windows")]
pub use windows::{clipboard, pin};
