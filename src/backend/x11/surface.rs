//! 【X11窗口】【原生资源】窗口与绘图上下文按同一生命周期创建和释放

use super::{connection::Display, pixels};
use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;

pub struct Surface {
    pub display: Display,
    pub window: Window,
    pub gc: Gcontext,
    pub size: (u16, u16),
    pub position: (i16, i16),
    pub mapped: bool,
    pub pixels: Vec<u8>,
}

impl Surface {
    /// 【X11窗口】【资源创建】按display、position、size和overlay创建隐藏窗口，返回拥有资源的表面
    pub fn new(
        display: Display,
        position: (i32, i32),
        size: (u32, u32),
        overlay: bool,
    ) -> crate::AppResult<Self> {
        let size = (u16::try_from(size.0)?, u16::try_from(size.1)?);
        if size.0 == 0 || size.1 == 0 {
            return Err("empty X11 window".into());
        }
        let position = (i16::try_from(position.0)?, i16::try_from(position.1)?);
        let window = display.connection.generate_id()?;
        let gc = display.connection.generate_id()?;
        let screen = display.screen();
        let mask = EventMask::EXPOSURE
            | EventMask::STRUCTURE_NOTIFY
            | EventMask::KEY_PRESS
            | EventMask::KEY_RELEASE
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE
            | EventMask::POINTER_MOTION
            | EventMask::ENTER_WINDOW
            | EventMask::LEAVE_WINDOW
            | EventMask::FOCUS_CHANGE;
        display
            .connection
            .create_window(
                screen.root_depth,
                window,
                screen.root,
                position.0,
                position.1,
                size.0,
                size.1,
                0,
                WindowClass::INPUT_OUTPUT,
                screen.root_visual,
                &CreateWindowAux::new()
                    .override_redirect(u32::from(overlay))
                    .background_pixel(screen.black_pixel)
                    .event_mask(mask),
            )?
            .check()?;
        let surface = Self {
            display,
            window,
            gc,
            size,
            position,
            mapped: false,
            pixels: Vec::new(),
        };
        surface
            .display
            .connection
            .create_gc(gc, window, &CreateGCAux::new().graphics_exposures(0))?
            .check()?;
        surface.title("Mark Shot")?;
        surface.display.connection.change_property32(
            PropMode::REPLACE,
            window,
            surface.display.atom("_NET_WM_PID")?,
            AtomEnum::CARDINAL,
            &[std::process::id()],
        )?;
        surface.display.connection.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            b"mark-shot\0org.mark-shot.MarkShot\0",
        )?;
        let protocols = surface.display.atom("WM_PROTOCOLS")?;
        let delete = surface.display.atom("WM_DELETE_WINDOW")?;
        surface.display.connection.change_property32(
            PropMode::REPLACE,
            window,
            protocols,
            AtomEnum::ATOM,
            &[delete],
        )?;
        Ok(surface)
    }

    /// 【X11窗口】【标题同步】按title设置UTF-8窗口标题，返回协议错误
    pub fn title(&self, title: &str) -> crate::AppResult {
        self.display.connection.change_property8(
            PropMode::REPLACE,
            self.window,
            self.display.atom("_NET_WM_NAME")?,
            self.display.atom("UTF8_STRING")?,
            title.as_bytes(),
        )?;
        self.display.connection.change_property8(
            PropMode::REPLACE,
            self.window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )?;
        Ok(())
    }

    /// 【X11窗口】【绘制提交】按pixels保存当前帧并提交服务器，首次提交后映射窗口，返回协议错误
    pub fn present(&mut self, pixels: &[u8]) -> crate::AppResult {
        self.present_region(pixels, None)
    }

    /// 【X11截图场景】【持久帧合成】按 frame 更新唯一完整曝光副本并上传损坏部分，尺寸变化强制完整更新
    pub(crate) fn present_scene(
        &mut self,
        mut frame: crate::types::capture_scene::Frame<'_>,
    ) -> crate::AppResult {
        if (frame.foreground.width(), frame.foreground.height())
            != (u32::from(self.size.0), u32::from(self.size.1))
        {
            return Err("X11 scene dimensions differ from window".into());
        }
        if self.pixels.len() != frame.foreground.data().len() || !self.mapped {
            frame.damage = None;
            self.pixels.resize(frame.foreground.data().len(), 0);
        }
        crate::backend::software_scene::compose(&frame, &mut self.pixels, false)?;
        if !self.mapped {
            self.display.connection.map_window(self.window)?.check()?;
            self.mapped = true;
        }
        pixels::present_region(
            &self.display,
            self.window,
            self.gc,
            self.size,
            &self.pixels,
            frame.damage,
        )
    }

    /// 【X11窗口】【局部保存】按 pixels 和 damage 更新持久曝光帧，首次显示强制复制完整帧，返回协议结果
    pub(crate) fn present_region(
        &mut self,
        pixels: &[u8],
        damage: Option<crate::types::DamageRect>,
    ) -> crate::AppResult {
        let width = usize::from(self.size.0);
        let height = usize::from(self.size.1);
        if pixels.len() != width * height * 4 {
            return Err("X11 surface dimensions differ from pixels".into());
        }
        let damage = damage.filter(|_| self.mapped && self.pixels.len() == pixels.len());
        if let Some((x, y, w, h)) = damage {
            let right = x.saturating_add(w).min(self.size.0.into()) as usize;
            let bottom = y.saturating_add(h).min(self.size.1.into()) as usize;
            if x as usize >= right || y as usize >= bottom {
                return Ok(());
            }
            for row in y as usize..bottom {
                let start = (row * width + x as usize) * 4;
                let end = (row * width + right) * 4;
                self.pixels[start..end].copy_from_slice(&pixels[start..end]);
            }
        } else {
            self.pixels.clear();
            self.pixels.extend_from_slice(pixels);
        }
        if !self.mapped {
            self.display.connection.map_window(self.window)?.check()?;
            self.mapped = true;
        }
        pixels::present_region(
            &self.display,
            self.window,
            self.gc,
            self.size,
            &self.pixels,
            damage,
        )
    }

    /// 【X11窗口】【曝光重绘】无参数，上传保存的有效帧，返回协议错误
    pub fn repaint(&self) -> crate::AppResult {
        if self.pixels.len() == usize::from(self.size.0) * usize::from(self.size.1) * 4 {
            pixels::present(&self.display, self.window, self.gc, self.size, &self.pixels)?;
        }
        Ok(())
    }
}

impl Drop for Surface {
    /// 【X11窗口】【资源释放】无参数，销毁本表面窗口及绘图上下文，不关闭其他共享窗口
    fn drop(&mut self) {
        let _ = self.display.connection.free_gc(self.gc);
        let _ = self.display.connection.destroy_window(self.window);
        let _ = self.display.connection.flush();
    }
}
