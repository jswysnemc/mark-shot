//! 【Windows贴图】【共享内容】原图独立保存，显示缩放、旋转、菜单与输出使用原生窗口契约

use crate::{
    backend::window::{
        self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowPopupAction, WindowSpec,
    },
    features::pinned_image::{
        menu::{self, PinAction},
        menu_draw::{self, Row},
        model::PinnedImage,
    },
    types::popup::{PopupEvent, PopupFrame},
};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

struct Menu {
    id: u64,
    anchor: (f32, f32),
    hovered: Option<usize>,
    armed: Option<usize>,
    frame: Option<Pixmap>,
    changed: bool,
    mode: crate::theme::Mode,
    scale: f32,
}
struct Pin {
    image: PinnedImage,
    position_offset: Option<(f64, f64)>,
    initial: Option<(i32, i32)>,
    menu: Option<Menu>,
    next_menu: u64,
    fonts: cosmic_text::FontSystem,
    cache: cosmic_text::SwashCache,
    icons: crate::renderer::Icons,
}

impl Pin {
    /// 【Windows贴图】【菜单动作】按action变换原图或交付输出，返回重绘或关闭，保存和编辑使用共用业务入口
    fn action(&mut self, action: PinAction) -> crate::AppResult<WindowAction> {
        use crate::features::pinned_image::actions;
        match action {
            PinAction::RotateLeft | PinAction::RotateRight => {
                self.image.rotate(action == PinAction::RotateRight)?
            }
            PinAction::ZoomIn | PinAction::ZoomOut | PinAction::ResetSize => {
                let zoom = if action == PinAction::ResetSize {
                    1.0
                } else {
                    self.image.zoom
                        * if action == PinAction::ZoomIn {
                            1.18
                        } else {
                            1.0 / 1.18
                        }
                };
                self.image.scale(zoom);
            }
            PinAction::AlwaysOnTop => self.image.always_on_top = !self.image.always_on_top,
            PinAction::Edit => actions::open(&self.image.png()?, false)?,
            PinAction::Copy => crate::backend::initialize_clipboard()
                .copy_image_to_clipboard(&self.image.png()?)?,
            PinAction::CopyImageText => actions::copy_text(&self.image.png()?)?,
            PinAction::SaveAs => actions::save(self.image.png()?, self.image.density)?,
            PinAction::Close => return Ok(WindowAction::Close),
            _ => return Err("this Windows pin action has not been connected yet".into()),
        }
        Ok(WindowAction::Redraw)
    }
    /// 【Windows贴图】【可用动作】按action返回当前已接入能力，未接入文字选择与翻译不显示为可执行
    fn enabled(action: PinAction) -> bool {
        matches!(
            action,
            PinAction::RotateLeft
                | PinAction::RotateRight
                | PinAction::ZoomIn
                | PinAction::ZoomOut
                | PinAction::ResetSize
                | PinAction::AlwaysOnTop
                | PinAction::Edit
                | PinAction::Copy
                | PinAction::CopyImageText
                | PinAction::SaveAs
                | PinAction::Close
        )
    }
}
impl WindowContent for Pin {
    /// 【Windows贴图】【原始位置】无参数，返回选区交接位置，缺省交给平台居中
    fn initial_position(&self) -> Option<(i32, i32)> {
        self.initial
    }
    /// 【Windows贴图】【预览尺寸】无参数，返回缩放后的逻辑尺寸，原生窗口跟随预览，原图字节保持独立
    fn preferred_size(&self) -> Option<(u32, u32)> {
        Some(self.image.size())
    }
    /// 【Windows贴图】【锚点位移】取出本轮缩放的逻辑位移，消费后不在后续重绘重复移动
    fn take_position_offset(&mut self) -> Option<(f64, f64)> {
        self.position_offset.take()
    }
    /// 【Windows贴图】【滚轮预览】按input纵轴缩放并保持原图锚点，返回是否重绘；菜单期间及纯横向输入不修改图像
    fn scroll(&mut self, input: window::ScrollInput) -> WindowAction {
        use crate::features::pinned_image::wheel;
        if self.menu.is_some() {
            return WindowAction::None;
        }
        let Some(factor) = wheel::factor(input.angle.1, -input.delta.1) else {
            return WindowAction::None;
        };
        let before = self.image.size();
        if !self.image.scale(self.image.zoom * factor) {
            return WindowAction::None;
        }
        let previous = self.position_offset.unwrap_or_default();
        // 1. 【Windows贴图】【连续消息】绘制前可能收到多个滚轮事件，扣除待应用位移后再计算当前局部锚点
        let point = (input.position.0 - previous.0, input.position.1 - previous.1);
        let shift = wheel::offset(point, before, self.image.size());
        self.position_offset = Some((previous.0 + shift.0, previous.1 + shift.1));
        WindowAction::Redraw
    }
    /// 【Windows贴图】【置顶状态】无参数，返回当前原生置顶请求
    fn always_on_top(&self) -> bool {
        self.image.always_on_top
    }
    /// 【Windows贴图】【原图绘制】按canvas和scale缩放显示原图，保留原始像素用于复制与保存
    fn draw(&mut self, canvas: &mut Pixmap, _scale: f32) {
        canvas.draw_pixmap(
            0,
            0,
            self.image.image.as_ref(),
            &PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_scale(
                canvas.width() as f32 / self.image.image.width() as f32,
                canvas.height() as f32 / self.image.image.height() as f32,
            ),
            None,
        );
    }
    /// 【Windows贴图】【内容输入】按event处理移动、菜单、缩放和快捷键，返回窗口动作，错误记录明确反馈
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        let action = match event {
            WindowEvent::PointerButton {
                button: window::LEFT_BUTTON,
                pressed: true,
                ..
            } => return WindowAction::Move,
            WindowEvent::PointerButton {
                position,
                button: window::RIGHT_BUTTON,
                pressed: true,
            } => {
                self.next_menu += 1;
                self.menu = Some(Menu {
                    id: self.next_menu,
                    anchor: (position.0 as f32, position.1 as f32),
                    hovered: None,
                    armed: None,
                    frame: None,
                    changed: true,
                    mode: crate::theme::Mode::current(),
                    scale: 0.0,
                });
                return WindowAction::Redraw;
            }
            WindowEvent::Key(input) => match input.key {
                Some(crate::keys::Key::Escape) => Some(PinAction::Close),
                Some(crate::keys::Key::Char('c')) if input.mods.ctrl => Some(PinAction::Copy),
                Some(crate::keys::Key::Char('s')) if input.mods.ctrl => Some(PinAction::SaveAs),
                Some(crate::keys::Key::Char('e')) if input.mods.ctrl => Some(PinAction::Edit),
                Some(crate::keys::Key::Char('v')) if input.mods.ctrl => {
                    if let Err(error) = crate::utils::spawn_self(&["--pin-clipboard"], &[]) {
                        log::error!("【Windows贴图】【剪贴板新窗口】{error}");
                    }
                    None
                }
                _ => None,
            },
            WindowEvent::Resize { size } => {
                self.image.set_size(size);
                None
            }
            _ => None,
        };
        match action.map(|action| self.action(action)) {
            Some(Ok(action)) => action,
            Some(Err(error)) => {
                log::error!("【Windows贴图】【菜单动作】{error}");
                WindowAction::None
            }
            None => WindowAction::None,
        }
    }
    /// 【Windows贴图】【菜单身份】无参数，返回本次存活菜单编号，旧事件不能匹配重新打开菜单
    fn popup_id(&self) -> Option<u64> {
        self.menu.as_ref().map(|menu| menu.id)
    }
    /// 【Windows贴图】【菜单像素】按scale返回共享中文菜单帧，保留悬停和按下事务，不创建系统控件
    fn popup_frame(&mut self, scale: f32) -> Option<PopupFrame<'_>> {
        let popup = self.menu.as_mut()?;
        if !popup.changed && popup.mode == crate::theme::Mode::current() && popup.scale == scale {
            return None;
        }
        popup.changed = false;
        popup.mode = crate::theme::Mode::current();
        popup.scale = scale;
        let size = (menu::WIDTH, menu::height());
        let pixel_size = (
            (size.0 as f32 * scale).ceil() as u32,
            (size.1 as f32 * scale).ceil() as u32,
        );
        let mut frame = Pixmap::new(pixel_size.0, pixel_size.1)?;
        let ui = crate::ui::language::Language::current();
        let rows = menu::ITEMS
            .iter()
            .map(|item| Row {
                label: ui.text(item.label).into(),
                enabled: item.action.is_some_and(Self::enabled),
                checked: item.action == Some(PinAction::AlwaysOnTop) && self.image.always_on_top,
                danger: item.action == Some(PinAction::Close),
                submenu: false,
            })
            .collect::<Vec<_>>();
        menu_draw::draw(
            &mut frame,
            &rows,
            popup.hovered,
            size,
            scale,
            &mut self.fonts,
            &mut self.cache,
            &self.icons,
        );
        popup.frame = Some(frame);
        Some(PopupFrame {
            id: popup.id,
            monitor_idx: 0,
            serial: None,
            anchor: popup.anchor,
            size,
            scale,
            pixels: popup.frame.as_ref()?.data(),
            pixel_size,
        })
    }
    /// 【Windows贴图】【菜单输入】按id与event更新菜单悬停和按下，返回确认动作，释放前不执行业务
    fn popup_event(&mut self, id: u64, event: PopupEvent) -> WindowPopupAction {
        let Some(popup) = self.menu.as_mut().filter(|menu| menu.id == id) else {
            return WindowPopupAction::None;
        };
        popup.changed = true;
        match event {
            PopupEvent::Pointer { x, y } => popup.hovered = menu::hit((x, y)),
            PopupEvent::Leave => popup.hovered = None,
            PopupEvent::Button {
                x,
                y,
                pressed,
                button: crate::types::MouseButton::Left,
            } => {
                let hit = menu::hit((x, y))
                    .filter(|index| menu::ITEMS[*index].action.is_some_and(Self::enabled));
                if pressed {
                    popup.armed = hit;
                } else if hit.is_some() && popup.armed.take() == hit {
                    return WindowPopupAction::Activate(hit.unwrap());
                }
            }
            PopupEvent::Closed => return WindowPopupAction::Close,
            PopupEvent::Key {
                chord: Some(chord), ..
            } => {
                use crate::{features::pinned_image::menu_navigation, keys::Key};
                // 1. 【Windows贴图】【键盘事务】键盘接管后清除未完成的鼠标按下，不允许随后释放重复交付
                popup.armed = None;
                match chord.key {
                    Key::Escape => return WindowPopupAction::Close,
                    Key::Enter | Key::Space => {
                        if let Some(index) = popup
                            .hovered
                            .filter(|index| menu_navigation::available(*index, Self::enabled))
                        {
                            return WindowPopupAction::Activate(index);
                        }
                    }
                    key => {
                        popup.hovered = menu_navigation::navigate(popup.hovered, key, Self::enabled)
                    }
                }
            }
            _ => {}
        }
        WindowPopupAction::Redraw
    }
    /// 【Windows贴图】【菜单交付】按id关闭已确认菜单并执行action，返回窗口动作，原生菜单先由运行时回收
    fn popup_closed(&mut self, id: u64, action: Option<usize>) -> WindowAction {
        if self.popup_id() != Some(id) {
            return WindowAction::None;
        }
        self.menu = None;
        if let Some(action) = action
            .and_then(|index| menu::ITEMS.get(index))
            .and_then(|item| item.action)
        {
            match self.action(action) {
                Ok(action) => action,
                Err(error) => {
                    log::error!("【Windows贴图】【菜单交付】{error}");
                    WindowAction::Redraw
                }
            }
        } else {
            WindowAction::Redraw
        }
    }
    /// 【Windows贴图】【移动光标】按position返回原图移动指针，菜单由独立表面处理
    fn cursor(&self, _position: (f64, f64)) -> WindowCursor {
        WindowCursor::Grab
    }
}
/// 【Windows贴图】【原图入口】按bytes与density启动独立原生贴图，返回窗口结果，不初始化截图后端
pub fn run(bytes: &[u8], density: f32) -> crate::AppResult {
    run_at(bytes, density, None)
}
/// 【Windows贴图】【定位入口】按bytes、density和position恢复选区位置，返回独立原生窗口结果
pub(super) fn run_at(bytes: &[u8], density: f32, position: Option<(i32, i32)>) -> crate::AppResult {
    run_at_seed(bytes, density, position, None)
}
/// 【Windows贴图】【种子入口】按bytes、density、position与seed启动原生贴图，尚未接入的翻译种子明确返回错误
pub(super) fn run_at_seed(
    bytes: &[u8],
    density: f32,
    position: Option<(i32, i32)>,
    seed: Option<crate::features::image_translation::seed::Seed>,
) -> crate::AppResult {
    if seed.is_some() {
        return Err("translated Windows pin startup is not connected yet".into());
    }
    let mut image = PinnedImage::decode(bytes, density)?;
    image.always_on_top = crate::config::get().pinned.always_on_top;
    window::run(
        WindowSpec {
            title: "Pinned Mark Shot".into(),
            app_id: "mark-shot-pin".into(),
            size: image.size(),
            minimum: (24, 24),
        },
        Pin {
            image,
            position_offset: None,
            initial: position,
            menu: None,
            next_menu: 0,
            fonts: crate::fonts::bundled(),
            cache: cosmic_text::SwashCache::new(),
            icons: crate::app::load_icons_cache(),
        },
    )
}
