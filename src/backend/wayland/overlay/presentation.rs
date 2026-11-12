//! 【截图覆盖层】【表面创建】按输出身份创建截图角色及配置缓冲，独立于事件循环与结果交接

use super::{CONFIGURE_TIMEOUT, WaylandOverlay, buffer_budget, fault_in};
use crate::backend::wayland::utils::{
    scroll_popup::ScrollPopup,
    surface::{InputRegion, Role, SurfaceData},
};
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use std::time::Instant;

impl WaylandOverlay {
    /// 【截图覆盖层】【表面创建】按 targets 保留真实输出并等待配置，普通控制窗口仅申请面板大小，返回协议结果
    pub(super) fn present_surfaces(
        &mut self,
        targets: &[usize],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();

        if rt.state.outputs.is_empty() {
            return Err("compositor reported no outputs".into());
        }
        let hidpi = rt.state.viewporter.is_some();
        let scrolling = self.scrolling;
        let panel = self
            .scroll_window
            .as_ref()
            .filter(|_| scrolling.is_some())
            .map(|window| window.rect)
            .or_else(|| {
                (scrolling.is_some() && rt.state.viewporter.is_some())
                    .then_some(self.scroll_panel_rect)
                    .flatten()
            });
        let prefault = buffer_budget::size(
            targets
                .iter()
                .enumerate()
                .filter(|(index, _)| scrolling.is_none_or(|selected| *index == selected))
                .filter_map(|(_, &index)| rt.state.outputs.get(index)),
            hidpi,
            scrolling.is_none() && !self.composited_background,
            panel,
        );
        let outputs_snapshot: Vec<_> = targets
            .iter()
            .filter_map(|&i| rt.state.outputs.get(i))
            .map(|o| {
                let scale = if hidpi { o.scale() } else { 1.0 };
                (
                    o.wl_output.clone().expect("Wayland overlay output"),
                    o.info.logical_size.unwrap_or((0, 0)),
                    scale,
                )
            })
            .collect();
        self.targets = outputs_snapshot
            .iter()
            .map(|(output, _, _)| output.clone())
            .collect();

        // 1. 【截图覆盖层】【按需共享页】普通原生帧与独立冻结背景都从零页开始，避免尚未绘制的活动缓冲占有完整物理页
        let prepare_pool = scrolling.is_some()
            || (!self.composited_background && rt.state.subcompositor.is_none());
        if (self.layer || scrolling.is_some()) && prepare_pool {
            fault_in(&mut rt.state.pool, &mut self.faulted, prefault);
        }

        // for each output
        for (i, (wl_output, (w, h), scale)) in outputs_snapshot.into_iter().enumerate() {
            if self.scrolling.is_some_and(|selected| selected != i) {
                continue;
            }
            let panel_rect = self
                .scroll_window
                .as_ref()
                .filter(|_| self.scrolling.is_some())
                .map(|panel| panel.rect);
            let (w, h) = panel_rect.map_or((w as u32, h as u32), |rect| {
                (rect.width().ceil() as u32, rect.height().ceil() as u32)
            });
            let (w, h) = rt.state.image_window.unwrap_or((w, h));

            let surface = rt.state.compositor_state.create_surface(&qh);

            // scale the surface layout if the viewporter protocol is available
            let viewport = rt
                .state
                .viewporter
                .as_ref()
                .map(|viewporter| viewporter.get_viewport(&surface, &qh, ()));

            let role = match rt
                .state
                .layer_shell
                .as_ref()
                .filter(|_| self.layer || self.scrolling.is_some())
            {
                Some(shell) => {
                    // ── creating a wayland overlay on top, with screenshot itself and etc ────────────────────
                    let namespace = match std::env::var("XDG_CURRENT_DESKTOP").as_deref() {
                        // namespace utility on kde ignores both opening & closing animations
                        Ok("KDE") => "utility",
                        _ => "mark-shot",
                    };
                    let layer = shell.create_layer_surface(
                        &qh,
                        surface.clone(),
                        Layer::Overlay,
                        Some(namespace),
                        Some(&wl_output),
                    );
                    layer.set_anchor(Anchor::all());
                    layer.set_exclusive_zone(-1);
                    layer.set_keyboard_interactivity(if self.scrolling.is_some() {
                        KeyboardInteractivity::OnDemand
                    } else {
                        KeyboardInteractivity::Exclusive
                    });
                    Role::Layer(layer)
                }
                None => {
                    if panel_rect.is_some() {
                        let parent = ScrollPopup::create_parent(
                            &rt.state.compositor_state,
                            &rt.state.xdg_shell,
                            &qh,
                        );
                        let parent_surface =
                            smithay_client_toolkit::shell::WaylandSurface::wl_surface(&parent)
                                .clone();
                        let parent_viewport = rt
                            .state
                            .viewporter
                            .as_ref()
                            .map(|manager| manager.get_viewport(&parent_surface, &qh, ()));
                        parent_surface.commit();
                        rt.state.surfaces.insert(
                            i,
                            SurfaceData {
                                background: None,
                                scroll_layer: None,
                                surface: parent_surface,
                                viewport: parent_viewport,
                                role: Role::ScrollParent(parent),
                                shm_buffer: None,
                                spare_buffer: None,
                                buffer_history: Default::default(),
                                waiting_release: false,
                                mapped: false,
                                input: InputRegion::Nothing,
                                canvas_rect: None,
                                scroll_frame: None,
                                scroll_stamps: Default::default(),
                                width: 1,
                                height: 1,
                                scale: 1.0,
                                frame_sent: None,
                            },
                        );
                        let deadline = Instant::now() + CONFIGURE_TIMEOUT;
                        while rt
                            .state
                            .surfaces
                            .get(&i)
                            .is_some_and(|parent| parent.shm_buffer.is_none())
                        {
                            rt.event_queue.roundtrip(&mut rt.state)?;
                            if let Some(error) = rt.state.configure_error.take() {
                                return Err(error.into());
                            }
                            if Instant::now() >= deadline {
                                return Err("compositor did not configure the scroll parent".into());
                            }
                        }
                        // 1. 【滚动截图】【父窗口屏障】确认父缓冲映射已经到达合成器，再建立子角色
                        rt.event_queue.roundtrip(&mut rt.state)?;
                        let mut parent = rt
                            .state
                            .surfaces
                            .remove(&i)
                            .ok_or("the scroll parent is gone")?;
                        let parent_size = (parent.width, parent.height);
                        let parent_buffer = parent
                            .shm_buffer
                            .take()
                            .ok_or("the scroll parent buffer is absent")?;
                        let parent_viewport = parent.viewport.take();
                        let Role::ScrollParent(parent_window) = parent.role else {
                            return Err("invalid scroll parent role".into());
                        };
                        Role::ScrollPopup(Box::new(ScrollPopup::new(
                            (w, h),
                            surface.clone(),
                            parent_window,
                            parent_buffer,
                            parent_size,
                            parent_viewport,
                            &rt.state.xdg_shell,
                            &qh,
                        )?))
                    } else if rt.state.image_window.is_some() {
                        super::image_window::create(&rt.state, &qh, surface.clone(), (w, h))
                    } else {
                        // ── creating a forced full screen window, with screenshot itself and etc ────────────────────
                        let window = rt.state.xdg_shell.create_window(
                            surface.clone(),
                            smithay_client_toolkit::shell::xdg::window::WindowDecorations::None,
                            &qh,
                        );
                        window.set_title("mark-shot");
                        window.set_app_id("mark-shot");
                        window.set_fullscreen(Some(&wl_output));
                        Role::Window(window)
                    }
                }
            };

            // handle fractional scaling calculations for HiDPI setups
            if let Some(frac) = rt.state.frac.as_ref() {
                rt.state.frac_scale = Some(frac.get_fractional_scale(&surface, &qh, ()));
            }

            surface.commit();

            let scroll_layer =
                (scrolling.is_some() && matches!(&role, Role::Layer(_))).then(Default::default);
            rt.state.surfaces.insert(
                i,
                SurfaceData {
                    background: None,
                    scroll_layer,
                    surface,
                    viewport,
                    role,
                    shm_buffer: None,
                    spare_buffer: None,
                    buffer_history: Default::default(),
                    waiting_release: false,
                    mapped: false,
                    input: InputRegion::Full,
                    canvas_rect: panel_rect,
                    scroll_frame: None,
                    scroll_stamps: Default::default(),
                    width: w,
                    height: h,
                    scale,
                    frame_sent: None,
                },
            );
        }

        // IMPORTANT PART : 15 ms gain
        // when requesting memory, linux doesn't actually allocate physical ram, it just gives us an address space
        // physical allocation happens only on first write via page faults, which takes 15 ms on my machine
        // so instead of triggering page faults after receiving the screenshot, we do it
        // now while waiting for wayland configure anyways

        // we fill this memory with zeros to force physical page allocation before getting the screenshot
        // so when it arrives, pages are already mapped, giving us some speed gain
        rt.event_queue.flush()?;
        if !self.layer && prepare_pool {
            fault_in(&mut rt.state.pool, &mut self.faulted, prefault);
        }

        let deadline = Instant::now() + CONFIGURE_TIMEOUT;
        while rt.state.surfaces.values().any(|sd| sd.shm_buffer.is_none()) {
            // freezes untill configure_surface create necessary buffers in state/compositor_shm.rs
            // if we continue without waiting compositor response, app will crash
            rt.event_queue.roundtrip(&mut rt.state)?;
            if let Some(e) = rt.state.configure_error.take() {
                return Err(format!("can't allocate overlay buffer: {e}").into());
            }
            if Instant::now() > deadline {
                return Err("compositor didn't configure the overlay windows".into());
            }
        }

        Ok(())
    }
}
