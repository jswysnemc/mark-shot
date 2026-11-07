//! 【贴图】【动作执行】菜单关闭后交付图像变换、输出和独立编辑任务

use super::runtime::{Pin, Shell};
use crate::{
    AppResult,
    features::pinned_image::{actions, menu::PinAction},
};
use smithay_client_toolkit::{
    compositor::Region,
    shell::wlr_layer::{KeyboardInteractivity, Layer},
};
use wayland_client::QueueHandle;

impl Pin {
    /// 【贴图】【首次模型引导】临时交还输入和置顶层级，返回明确选择的模型文件，取消后恢复当前贴图
    pub fn model_guide(&mut self) -> AppResult<Option<crate::ocr::models::ModelFiles>> {
        let region = Region::new(&self.compositor)?;
        self.shell
            .surface()
            .set_input_region(Some(region.wl_region()));
        if let Shell::Layer(layer) = &self.shell {
            layer.set_layer(Layer::Bottom);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        }
        self.shell.surface().commit();
        let result = (|| {
            self.connection.flush()?;
            crate::features::ocr_model_guide::run()
        })();
        self.shell.surface().set_input_region(None);
        if let Shell::Layer(layer) = &self.shell {
            layer.set_layer(Layer::Top);
            layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        }
        self.shell.surface().commit();
        let restored = self.connection.flush();
        let selected = result?;
        restored?;
        Ok(selected)
    }
    /// 【贴图】【模态保存】另存为期间降低置顶层级并关闭输入；返回保存结果，取消或失败后恢复原角色
    fn save_as(&mut self) -> AppResult {
        let png = self.image.png()?;
        let region = Region::new(&self.compositor)?;
        self.shell
            .surface()
            .set_input_region(Some(region.wl_region()));
        if let Shell::Layer(layer) = &self.shell {
            layer.set_layer(Layer::Bottom);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        }
        self.shell.surface().commit();
        let result = (|| {
            self.connection.flush()?;
            actions::save(png, self.image.density)
        })();
        // 1. 【贴图】【模态结束】恢复置顶和输入，不改变原图、倍率或文字选择
        self.shell.surface().set_input_region(None);
        if let Shell::Layer(layer) = &self.shell {
            layer.set_layer(Layer::Top);
            layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        }
        self.shell.surface().commit();
        let restored = self.connection.flush();
        result?;
        restored?;
        Ok(())
    }
    /// 【贴图】【错误反馈】记录 result 中的失败并发送桌面通知；成功不增加无用提示
    pub fn report(&self, result: AppResult) {
        if let Err(error) = result {
            log::error!("【贴图】【菜单动作】动作失败: {error}");
            crate::backend::notify::send_blocking(crate::backend::notify::Notice::Failed(
                error.to_string(),
            ));
        }
    }

    /// 【贴图】【菜单执行】关闭菜单后执行 action；qh 用于重建置顶角色，不销毁原图
    pub fn action(&mut self, action: PinAction, qh: &QueueHandle<Self>) {
        if !self.menu_enabled(action) {
            return;
        }
        self.popup = None;
        let result: AppResult = (|| {
            // 1. 【贴图】【抓取释放】嵌套保存窗口进入阻塞循环前必须发送旧菜单销毁请求
            self.connection.flush()?;
            match action {
                PinAction::RotateLeft | PinAction::RotateRight => {
                    self.clear_ocr();
                    self.image.rotate(action == PinAction::RotateRight)?;
                    self.resize(true, qh);
                    if self.text_selection {
                        self.start_ocr()?;
                    }
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
                    self.resize(true, qh);
                }
                PinAction::AlwaysOnTop => {
                    self.image.always_on_top = !self.image.always_on_top;
                    self.recreate(qh);
                    self.resize(false, qh);
                }
                PinAction::Edit => actions::open(&self.image.png()?, false)?,
                PinAction::Copy => crate::backend::initialize_clipboard()
                    .copy_image_to_clipboard(&self.image.png()?)?,
                PinAction::CopySelectedText => {
                    crate::utils::copy_to_clipboard(&self.text.line_text(true))?
                }
                PinAction::CopyImageText => {
                    if self.text.is_active() {
                        crate::utils::copy_to_clipboard(&self.text.line_text(false))?;
                    } else {
                        self.copy_after_ocr = true;
                        if !self.start_ocr()? {
                            self.copy_after_ocr = false;
                        }
                    }
                }
                PinAction::TextSelection => {
                    self.text_selection = !self.text_selection;
                    if self.text_selection {
                        self.text_selection = if self.text.is_active() {
                            true
                        } else {
                            self.start_ocr()?
                        };
                    } else {
                        self.text.deselect();
                        self.selecting = false;
                    }
                    self.dirty = true;
                }
                PinAction::SaveAs => self.save_as()?,
                PinAction::Close => self.exit = true,
                PinAction::Translate => self.request_translation()?,
                PinAction::ToggleTranslation => {
                    self.set_translation_active(!self.translation.active)
                }
                _ => {}
            }
            Ok(())
        })();
        self.report(result);
    }
}
