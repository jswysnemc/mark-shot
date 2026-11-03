// ── Wayland Screen Overlay Implementation ─────────────────────────────────────
//
// this module implements the 'ScreenOverlay' trait for wayland using SCTK
// It is responsible for creating surfaces, mapping them to the correct outputs
// via xdg windows or layer_shell, managing shm pixel buffers, and driving the event loop
//
// by default, auto decides what to use, for everything a fullscreen window
// for scrolling wm (for now only niri) layer, since creating a window there will start a clunky animation for a screenshot app
// and for COSMIC, since force fullscreen window is unstable, at least for now (focus grabbing + gray cosmic ui)
use rustix::{
    event::{PollFd, PollFlags, poll},
    time::Timespec,
};
use std::io::ErrorKind;
use std::os::unix::io::AsFd;
use std::time::{Duration, Instant};
use wayland_client::backend::WaylandError;

pub mod state;
mod visibility;

use crate::backend::ScreenOverlay;
use crate::backend::wayland::utils::shm::create_shm_buffer;
use crate::backend::wayland::utils::surface::{Background, InputRegion, Role, SurfaceData};
use crate::types::{CursorIcon, DamageRect, Output, OverlayEvent};
use log::info;
use smithay_client_toolkit::compositor::{FrameCallbackData, Region};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use smithay_client_toolkit::shm::slot::SlotPool;
use state::Probe;

const CONFIGURE_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_millis(300);

pub struct WaylandOverlay {
    runtime: state::OverlayRunTime,
    layer: bool,
    faulted: usize,
    targets: Vec<wayland_client::protocol::wl_output::WlOutput>,
    suspended: bool,
}

impl WaylandOverlay {
    pub fn new(
        connection: wayland_client::Connection,
        layer: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let rt = state::OverlayRunTime::new(&connection)?;
        let has_layer_shell = rt.state.layer_shell.is_some();
        if layer && !has_layer_shell {
            info!("compositor has no wlr-layer-shell, the overlay opens as a window");
        }

        Ok(Self {
            runtime: rt,
            layer: layer && has_layer_shell,
            faulted: 0,
            targets: Vec::new(),
            suspended: false,
        })
    }
}

impl ScreenOverlay for WaylandOverlay {
    /// 【输入法】【原生同步】同步 snapshot 后刷新连接，空值停用正文输入；不依赖画布重绘，返回协议结果
    fn set_text_input(
        &mut self,
        snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        self.runtime.state.text_input.sync(snapshot);
        self.runtime.event_queue.flush()?;
        Ok(())
    }
    /// 【截图弹层】【原生同步】将 frame 交给真实 Wayland 弹出表面；空值释放菜单抓取，返回原生支持状态
    fn set_popup(
        &mut self,
        frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match frame {
            Some(frame) => self
                .runtime
                .state
                .sync_popup(frame, &self.runtime.event_queue.handle())?,
            None if self.runtime.state.popup.is_none() => return Ok(true),
            None => self.runtime.state.popup = None,
        }
        self.runtime.event_queue.flush()?;
        Ok(true)
    }
    /// 【截图覆盖层】【暂停显示】销毁当前表面以露出真实桌面，保留目标输出；返回暂停结果
    fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.suspend_surfaces()
    }

    /// 【截图覆盖层】【恢复显示】按保留的输出身份恢复表面，避免依赖可能变化的全局索引；返回恢复结果
    fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.resume_surfaces()
    }
    /// Preloads memory for all monitors
    /// this is done in advance so as not to waste extra time on it later.
    ///
    /// function is called before capturing
    /// At this point, we can't know which monitor is active, so it's called for literally all of them.
    ///
    /// TODO: on -m, instead of preloading memory for all amonitors, preloading max possible size of one monitor
    fn prefault(&mut self) {
        let state = &mut self.runtime.state;
        let hidpi = state.viewporter.is_some();
        let size = prefault_size(state.outputs.iter(), hidpi);
        fault_in(&mut state.pool, &mut self.faulted, size);
    }

    fn present(&mut self, targets: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();

        if rt.state.outputs.is_empty() {
            return Err("compositor reported no outputs".into());
        }
        let hidpi = rt.state.viewporter.is_some();
        let prefault = prefault_size(
            targets.iter().filter_map(|&i| rt.state.outputs.get(i)),
            hidpi,
        );
        let outputs_snapshot: Vec<_> = targets
            .iter()
            .filter_map(|&i| rt.state.outputs.get(i))
            .map(|o| {
                let scale = if hidpi { o.scale() } else { 1.0 };
                (
                    o.wl_output.clone(),
                    o.info.logical_size.unwrap_or((0, 0)),
                    scale,
                )
            })
            .collect();
        self.targets = outputs_snapshot
            .iter()
            .map(|(output, _, _)| output.clone())
            .collect();

        if self.layer {
            fault_in(&mut rt.state.pool, &mut self.faulted, prefault);
        }

        // for each output
        for (i, (wl_output, (w, h), scale)) in outputs_snapshot.into_iter().enumerate() {
            let (w, h) = (w as u32, h as u32);

            let surface = rt.state.compositor_state.create_surface(&qh);

            // scale the surface layout if the viewporter protocol is available
            let viewport = rt
                .state
                .viewporter
                .as_ref()
                .map(|viewporter| viewporter.get_viewport(&surface, &qh, ()));

            let role = match rt.state.layer_shell.as_ref().filter(|_| self.layer) {
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
                    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                    Role::Layer(layer)
                }
                None => {
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
            };

            // handle fractional scaling calculations for HiDPI setups
            if let Some(frac) = rt.state.frac.as_ref() {
                rt.state.frac_scale = Some(frac.get_fractional_scale(&surface, &qh, ()));
            }

            surface.commit();

            rt.state.surfaces.insert(
                i,
                SurfaceData {
                    background: None,
                    surface,
                    viewport,
                    role,
                    shm_buffer: None,
                    mapped: false,
                    input: InputRegion::Full,
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
        if !self.layer {
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

    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();
        let sd = rt
            .state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("surface not found")?;

        let pool = &mut rt.state.pool;
        let (bw, bh) = sd.buffer_size();

        let buffer = sd
            .shm_buffer
            .as_mut()
            .ok_or("SHM buffer is not configured yet")?;

        // optimization: only upload and redraw the modified area
        if let Some((x, y, w, h)) = damage {
            if x == 0 && y == 0 && w == bw && h == bh {
                buffer.write_pixels(pool, pixels);
                sd.surface.attach(Some(buffer.wl_buffer()), 0, 0);
                sd.surface.damage_buffer(0, 0, bw as i32, bh as i32);
            } else {
                buffer.write_pixels_rect(pool, pixels, bw, (x, y, w, h));
                sd.surface.attach(Some(buffer.wl_buffer()), 0, 0);
                sd.surface
                    .damage_buffer(x as i32, y as i32, w as i32, h as i32);
            }
        } else {
            // fallback to full frame redraw
            buffer.write_pixels(pool, pixels);
            sd.surface.attach(Some(buffer.wl_buffer()), 0, 0);
            sd.surface.damage_buffer(0, 0, bw as i32, bh as i32);
        }

        // next frame of this surface waits until the compositor has shown this one
        sd.surface.frame(&qh, FrameCallbackData(sd.surface.clone()));
        sd.frame_sent = Some(Instant::now());
        sd.surface.commit();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.event_queue.flush()?;
        Ok(())
    }

    fn next_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<OverlayEvent, Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;

        loop {
            // prepare the wayland connection socket for reading incoming server events
            if let Some(guard) = rt.event_queue.prepare_read() {
                match guard.read() {
                    Ok(_) => {}
                    Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.into()),
                }
            }
            rt.event_queue.dispatch_pending(&mut rt.state)?;

            if rt.state.pending_flush {
                rt.state.pending_flush = false;
                rt.event_queue.flush()?;
            }

            if let Some(mut ev) = rt.state.events.pop_front() {
                if let OverlayEvent::TextInput(update) = &mut ev {
                    rt.state.text_input.validate(update);
                }
                // 1. 【截图弹层】【序列号保留】只清理已经分派的旧按下记录，未来排队输入不能挤掉当前菜单的抓取身份
                if let OverlayEvent::PointerButton {
                    serial: Some(serial),
                    ..
                } = &ev
                {
                    crate::backend::wayland::overlay_popup::retain_current_press(
                        &mut rt.state.popup_grabs,
                        *serial,
                    );
                }
                // optimization: coalesce sequential mouse move events.
                // we only care about the latest mouse coordinate in the event queue per frame
                if let OverlayEvent::PointerMove { .. } = ev {
                    let mut latest_move = ev;
                    while let Some(
                        OverlayEvent::PointerMove { .. } | OverlayEvent::FrameDone { .. },
                    ) = rt.state.events.front()
                    {
                        let ev = rt.state.events.pop_front().unwrap();
                        if let OverlayEvent::PointerMove { .. } = ev {
                            latest_move = ev;
                        }
                    }
                    return Ok(latest_move);
                }
                if let OverlayEvent::FrameDone { .. } = ev
                    && !rt.state.events.is_empty()
                {
                    continue;
                }
                return Ok(ev);
            }

            // block and wait until the Wayland connection file descriptor has data available (poll)
            let fd = rt.event_queue.as_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            let deadline = timeout.map(|t| Timespec {
                tv_sec: t.as_secs() as i64,
                tv_nsec: t.subsec_nanos() as i64,
            });

            poll(&mut fds, deadline.as_ref())?;

            if fds[0].revents().intersects(PollFlags::ERR | PollFlags::HUP) {
                return Err("Wayland connection closed".into());
            }
            // but if poll timed out without events, emit a Tick event to drive internal ui animations
            if !fds[0].revents().contains(PollFlags::IN) {
                return Ok(OverlayEvent::Tick);
            }
        }
    }

    fn discovered_outputs(&self) -> &[Output] {
        &self.runtime.state.outputs
    }

    fn hidpi(&self) -> bool {
        self.runtime.state.viewporter.is_some()
    }

    fn ready(&self, monitor_idx: usize) -> bool {
        self.runtime
            .state
            .surfaces
            .get(&monitor_idx)
            .and_then(|sd| sd.frame_sent)
            .is_none_or(|sent| sent.elapsed() >= crate::backend::FRAME_FALLBACK)
    }

    fn active_output(&mut self) -> Option<usize> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();
        let shell = rt.state.layer_shell.as_ref()?;

        let surface = rt.state.compositor_state.create_surface(&qh);
        let layer =
            shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("lumine-probe"), None);
        layer.set_size(1, 1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        if let Ok(region) = Region::new(&rt.state.compositor_state) {
            layer
                .wl_surface()
                .set_input_region(Some(region.wl_region()));
        }
        layer.commit();
        rt.state.probe = Some(Probe {
            layer,
            buffer: None,
            output: None,
        });

        let deadline = Instant::now() + PROBE_TIMEOUT;
        while rt.state.probe.as_ref().is_some_and(|p| p.output.is_none())
            && Instant::now() < deadline
        {
            if rt.event_queue.roundtrip(&mut rt.state).is_err() {
                break;
            }
            if rt.state.probe.as_ref().is_some_and(|p| p.output.is_none()) {
                std::thread::sleep(Duration::from_millis(2));
            }
        }

        let output = rt.state.probe.take()?.output?;
        let _ = rt.event_queue.flush();
        rt.state.outputs.iter().position(|o| o.wl_output == output)
    }

    fn retain_outputs(&mut self, keep: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        self.targets = keep
            .iter()
            .filter_map(|&index| self.targets.get(index).cloned())
            .collect();
        let rt = &mut self.runtime;
        let remap = |idx: usize| keep.iter().position(|&k| k == idx);

        for (idx, sd) in std::mem::take(&mut rt.state.surfaces) {
            if let Some(new_idx) = remap(idx) {
                rt.state.surfaces.insert(new_idx, sd);
            }
        }

        rt.state.pointer_surface_idx = rt.state.pointer_surface_idx.and_then(remap);
        rt.state.events.retain_mut(|ev| match ev {
            OverlayEvent::PointerMove { monitor_idx, .. }
            | OverlayEvent::Focus { monitor_idx }
            | OverlayEvent::FocusLost { monitor_idx }
            | OverlayEvent::FrameDone { monitor_idx } => match remap(*monitor_idx) {
                Some(idx) => {
                    *monitor_idx = idx;
                    true
                }
                None => false,
            },
            _ => true,
        });

        rt.event_queue.flush()?;
        Ok(())
    }

    fn set_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();
        let state = &mut rt.state;

        let subcompositor = state
            .subcompositor
            .as_ref()
            .ok_or("compositor has no wl_subcompositor")?;
        let sd = state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("surface not found")?;
        if state.viewporter.is_none() && (width, height) != (sd.width, sd.height) {
            return Err("can't scale the background without wp_viewporter".into());
        }

        let (subsurface, surface) = subcompositor.create_subsurface(sd.surface.clone(), &qh);
        subsurface.place_below(&sd.surface);
        let viewport = state.viewporter.as_ref().map(|viewporter| {
            let viewport = viewporter.get_viewport(&surface, &qh, ());
            viewport.set_destination(sd.width as i32, sd.height as i32);
            viewport
        });
        if let Ok(region) = Region::new(&state.compositor_state) {
            surface.set_input_region(Some(region.wl_region()));
        }

        let mut buffer = create_shm_buffer(&mut state.pool, width, height)?;
        buffer.write_pixels(&mut state.pool, pixels);
        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();

        sd.background = Some(Background {
            subsurface,
            surface,
            viewport,
            buffer,
        });
        Ok(())
    }

    fn set_cursor(&mut self, icon: CursorIcon) {
        self.runtime.state.apply_cursor(icon);
    }

    fn clear_background(&mut self, monitor_idx: usize) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let Some(sd) = rt.state.surfaces.get_mut(&monitor_idx) else {
            return Err("surface not found".into());
        };
        // detach buffer from background subsurface so live content reveals
        if let Some(background) = sd.background.take() {
            background.surface.attach(None, 0, 0);
            background.surface.commit();
        }
        rt.event_queue.flush()?;
        Ok(())
    }

    fn set_input_rect(&mut self, monitor_idx: usize, rect: Option<(i32, i32, i32, i32)>) {
        let rt = &mut self.runtime;
        let Some(sd) = rt.state.surfaces.get_mut(&monitor_idx) else {
            return;
        };
        // empty input region lets clicks and wheel pass through to the window below
        sd.input = match rect {
            Some((x, y, w, h)) => InputRegion::Rect(x, y, w, h),
            None => InputRegion::Nothing,
        };

        // unmapped surfaces cannot commit state; defer until configure
        if !sd.mapped {
            return;
        }
        sd.apply_input(&rt.state.compositor_state);
        sd.surface.commit();
        let _ = rt.event_queue.flush();
    }
}

fn prefault_size<'a>(outputs: impl Iterator<Item = &'a Output>, hidpi: bool) -> usize {
    outputs
        .flat_map(|o| {
            let (w, h) = o.info.logical_size.unwrap_or((0, 0));
            let scale = if hidpi { o.scale() } else { 1.0 };
            let (pw, ph) = crate::types::px_size((w.max(0) as u32, h.max(0) as u32), scale);
            let top = (pw as i32, ph as i32);
            let background = o
                .info
                .modes
                .iter()
                .find(|m| m.current)
                .map_or((0, 0), |m| m.dimensions);
            [top, background]
        })
        // every buffer get rounded to 64 byte, since when working with uneven resolution
        // it would cause to use realloc which cause bugs and overall not effecient
        .map(|(w, h)| (w.max(0) as usize * h.max(0) as usize * 4).next_multiple_of(64))
        .sum()
}

fn fault_in(pool: &mut SlotPool, faulted: &mut usize, size: usize) {
    if size <= *faulted {
        return;
    }
    if let Ok(slot) = pool.new_slot(size) {
        pool.raw_data_mut(&slot).fill(0);
        *faulted = size;
    }
}
