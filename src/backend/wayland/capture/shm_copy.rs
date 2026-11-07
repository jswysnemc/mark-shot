// ── screencopy & image-copy shared code ─────────────────────────────────────────

use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, poll};
use rustix::time::Timespec;
use smithay_client_toolkit::delegate_dispatch2;
use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_buffer, wl_output, wl_registry, wl_shm};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};

use crate::backend::task_wake::{Source as CancelSource, Wake};
use crate::types::{CaptureResult, MonitorFrame, Output, StreamInfo};
use crate::utils::to_rgba;

struct CancelOnDrop(Wake);

impl Drop for CancelOnDrop {
    /// 【原生捕获】【等待取消】异步请求结束或被丢弃时唤醒捕获线程，无参数、无返回值
    fn drop(&mut self) {
        self.0.notify();
    }
}

struct ProtocolOwner<P: Protocol>(Option<P>);

impl<P: Protocol> Drop for ProtocolOwner<P> {
    /// 【原生捕获】【协议回收】失败、取消和成功路径均销毁捕获协议对象，无参数、无返回值
    fn drop(&mut self) {
        if let Some(protocol) = self.0.take() {
            protocol.destroy();
        }
    }
}

struct FrameBuffer {
    pool: RawPool,
    buffer: wl_buffer::WlBuffer,
    bgr: bool,
}

impl Drop for FrameBuffer {
    /// 【原生捕获】【缓冲回收】解除服务端缓冲后由共享池释放映射，无参数、无返回值
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) trait Protocol: Sized + 'static {
    fn bind(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, String>;
    /// 【Wayland】【捕获请求】为 output 创建帧请求；idx 标识显示器，include_cursor 控制指针合成
    fn request(
        &mut self,
        output: &wl_output::WlOutput,
        idx: usize,
        qh: &QueueHandle<State>,
        include_cursor: bool,
    );
    fn copy(
        &mut self,
        idx: usize,
        buffer: &wl_buffer::WlBuffer,
        size: (u32, u32),
        qh: &QueueHandle<State>,
    );
    fn destroy(self);
}

struct Target {
    wl_output: wl_output::WlOutput,
    transform: wl_output::Transform,
    size: Option<(i32, i32)>,
    position: Option<(i32, i32)>,
}

#[derive(Default)]
pub(super) struct Slot {
    pub(super) size: Option<(u32, u32)>,
    pub(super) stride: u32,
    pub(super) formats: Vec<wl_shm::Format>,
    pub(super) constraints_done: bool,
    pub(super) transform: Option<wl_output::Transform>,
    pub(super) y_invert: bool,
    pub(super) result: Option<Result<(), String>>,
}

pub(super) struct State {
    shm: Shm,
    pub(super) slots: Vec<Slot>,
}

pub(super) fn supported(conn: &Connection, interfaces: &[&str]) -> bool {
    let Ok((globals, _)) = registry_queue_init::<State>(conn) else {
        return false;
    };
    globals.contents().with_list(|list| {
        interfaces
            .iter()
            .all(|&name| list.iter().any(|g| g.interface == name))
    })
}

pub(super) async fn capture<P: Protocol>(
    conn: &Connection,
    outputs: &[Output],
    include_cursor: bool,
) -> Result<CaptureResult, Box<dyn std::error::Error>> {
    let conn = conn.clone();
    let targets: Vec<Target> = outputs
        .iter()
        .map(|o| Target {
            wl_output: o.wl_output.clone(),
            transform: o.info.transform,
            size: o.info.logical_size,
            position: o.info.logical_position,
        })
        .collect();

    let cancel = CancelSource::new()?;
    let _cancel_on_drop = CancelOnDrop(cancel.wake());
    let frames = tokio::task::spawn_blocking(move || {
        capture_all::<P>(&conn, &targets, include_cursor, &cancel)
    })
    .await??;
    Ok(CaptureResult { frames })
}

fn capture_all<P: Protocol>(
    conn: &Connection,
    targets: &[Target],
    include_cursor: bool,
    cancel: &CancelSource,
) -> Result<Vec<MonitorFrame>, String> {
    // separate registry queue from overlay backend, but still the same connection
    let (globals, mut queue) =
        registry_queue_init::<State>(conn).map_err(|e| format!("wayland registry: {e}"))?;
    let qh = queue.handle();

    let mut owner = ProtocolOwner(Some(P::bind(&globals, &qh)?));
    let protocol = owner.0.as_mut().unwrap();
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?;

    let mut state = State {
        shm,
        slots: targets.iter().map(|_| Slot::default()).collect(),
    };
    let deadline = Instant::now() + FRAME_TIMEOUT;

    for (i, t) in targets.iter().enumerate() {
        protocol.request(&t.wl_output, i, &qh, include_cursor);
    }

    dispatch_until(&mut queue, &mut state, deadline, cancel, |s| {
        s.slots
            .iter()
            .all(|slot| slot.constraints_done || slot.result.is_some())
    })?;
    check_failed(&state)?;

    let mut pending = Vec::with_capacity(targets.len());
    for (i, slot) in state.slots.iter().enumerate() {
        let (w, h) = slot
            .size
            .ok_or_else(|| format!("output {i}: no buffer size"))?;
        let (format, bgr) = pick_format(&slot.formats)
            .ok_or_else(|| format!("output {i}: no usable shm format in {:?}", slot.formats))?;
        // one Pool per monitors, otherwise it doesn't work at least on Niri
        let mut pool = RawPool::new(slot.stride as usize * h as usize, &state.shm)
            .map_err(|e| format!("output {i}: shm pool: {e}"))?;
        let buffer = pool.create_buffer(0, w as i32, h as i32, slot.stride as i32, format, (), &qh);

        protocol.copy(i, &buffer, (w, h), &qh);
        pending.push(FrameBuffer { pool, buffer, bgr });
    }

    dispatch_until(&mut queue, &mut state, deadline, cancel, |s| {
        s.slots.iter().all(|slot| slot.result.is_some())
    })?;
    check_failed(&state)?;

    let mut frames = Vec::with_capacity(targets.len());
    for (i, (mut mapped, target)) in pending.into_iter().zip(targets).enumerate() {
        let slot = &state.slots[i];
        let (w, h) = slot.size.unwrap_or_default();

        let mut pixels = read_rows(mapped.pool.mmap(), w, h, slot.stride, slot.y_invert);
        to_rgba(&mut pixels, mapped.bgr, false);
        let transform = slot.transform.unwrap_or(target.transform);
        let (pixels, w, h) = untransform(pixels, w, h, transform);

        frames.push(MonitorFrame {
            output: i,
            pixels,
            pw_width: w,
            pw_height: h,
            pw_stride: w * 4,
            info: StreamInfo {
                node_id: 0,
                size: target.size,
                position: target.position,
            },
        });
    }

    drop(owner);
    let _ = queue.flush();

    Ok(frames)
}

fn read_rows(canvas: &[u8], w: u32, h: u32, stride: u32, y_invert: bool) -> Vec<u8> {
    let (row, h, stride) = (w as usize * 4, h as usize, stride as usize);
    let mut pixels = Vec::with_capacity(row * h);
    for y in 0..h {
        let src = if y_invert { h - 1 - y } else { y } * stride;
        pixels.extend_from_slice(&canvas[src..src + row]);
    }
    pixels
}

/// returning pixels to its normal orientation
/// Wayland returns pixel orientated as in the settings
///
/// while it seems okay, its a problem, since everything else
/// work with the screenshot as if its not orientated
/// it wasn't a conscious choice, its implemented like that
/// because both portal and kde screenshot protocol returns
/// pixels without orientating them
fn untransform(
    pixels: Vec<u8>,
    w: u32,
    h: u32,
    transform: wl_output::Transform,
) -> (Vec<u8>, u32, u32) {
    use wl_output::Transform as T;
    let (flip, quarters) = match transform {
        T::_90 => (false, 3),
        T::_180 => (false, 2),
        T::_270 => (false, 1),
        T::Flipped => (true, 0),
        T::Flipped90 => (true, 1),
        T::Flipped180 => (true, 2),
        T::Flipped270 => (true, 3),
        _ => return (pixels, w, h),
    };

    let (w, h) = (w as usize, h as usize);
    let (dw, dh) = if quarters % 2 == 1 { (h, w) } else { (w, h) };
    let mut out = vec![0u8; pixels.len()];
    for y in 0..h {
        for x in 0..w {
            let fx = if flip { w - 1 - x } else { x };
            let (dx, dy) = match quarters {
                1 => (y, w - 1 - fx),
                2 => (w - 1 - fx, h - 1 - y),
                3 => (h - 1 - y, fx),
                _ => (fx, y),
            };
            let src = (y * w + x) * 4;
            let dst = (dy * dw + dx) * 4;
            out[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
        }
    }
    (out, dw as u32, dh as u32)
}

fn pick_format(formats: &[wl_shm::Format]) -> Option<(wl_shm::Format, bool)> {
    [
        (wl_shm::Format::Xrgb8888, true),
        (wl_shm::Format::Argb8888, true),
        (wl_shm::Format::Xbgr8888, false),
        (wl_shm::Format::Abgr8888, false),
    ]
    .into_iter()
    .find(|(f, _)| formats.contains(f))
}

fn check_failed(state: &State) -> Result<(), String> {
    for (i, slot) in state.slots.iter().enumerate() {
        if let Some(Err(e)) = &slot.result {
            return Err(format!("output {i}: {e}"));
        }
    }
    Ok(())
}

fn dispatch_until(
    queue: &mut EventQueue<State>,
    state: &mut State,
    deadline: Instant,
    cancel: &CancelSource,
    done: impl Fn(&State) -> bool,
) -> Result<(), String> {
    queue.flush().map_err(|e| e.to_string())?;
    loop {
        queue.dispatch_pending(state).map_err(|e| e.to_string())?;
        if done(state) {
            return Ok(());
        }

        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("compositor didn't deliver the capture in time".into());
        }
        let timeout = Timespec {
            tv_sec: left.as_secs() as i64,
            tv_nsec: left.subsec_nanos() as i64,
        };
        let fd = guard.connection_fd();
        let mut fds = [
            PollFd::new(&fd, PollFlags::IN),
            PollFd::new(cancel, PollFlags::IN),
        ];
        poll(&mut fds, Some(&timeout)).map_err(|e| e.to_string())?;
        if fds[1].revents().contains(PollFlags::IN) {
            return Err("native capture cancelled".into());
        }
        if fds[0].revents().contains(PollFlags::IN) {
            match guard.read() {
                Ok(_) => {}
                Err(wayland_client::backend::WaylandError::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_dispatch2!(State);
wayland_client::delegate_noop!(State: ignore wl_buffer::WlBuffer);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(ids: &[u8]) -> Vec<u8> {
        ids.iter().flat_map(|&id| [id, id, id, 255]).collect()
    }

    fn ids(pixels: &[u8]) -> Vec<u8> {
        pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect()
    }

    #[test]
    fn rotated_frames_come_back_upright() {
        use wl_output::Transform as T;

        // 2 wide, 3 tall: a square source would hide every w/h and dw/dh mix-up
        //   1 2
        //   3 4
        //   5 6
        let source = [1, 2, 3, 4, 5, 6];

        // the mapping is inverted on purpose: T::_90 means the compositor already
        // turned the content, so undoing it takes the opposite quarter-turn
        let cases = [
            (T::Normal, [1, 2, 3, 4, 5, 6], (2, 3)),
            (T::_90, [5, 3, 1, 6, 4, 2], (3, 2)),
            (T::_180, [6, 5, 4, 3, 2, 1], (2, 3)),
            (T::_270, [2, 4, 6, 1, 3, 5], (3, 2)),
            (T::Flipped, [2, 1, 4, 3, 6, 5], (2, 3)),
            (T::Flipped90, [1, 3, 5, 2, 4, 6], (3, 2)),
            (T::Flipped180, [5, 6, 3, 4, 1, 2], (2, 3)),
            (T::Flipped270, [6, 4, 2, 5, 3, 1], (3, 2)),
        ];

        for (transform, expected, size) in cases {
            let (pixels, w, h) = untransform(img(&source), 2, 3, transform);
            assert_eq!(ids(&pixels), expected, "{transform:?}");
            assert_eq!((w, h), size, "{transform:?}");
            assert_eq!(
                pixels.len(),
                source.len() * 4,
                "{transform:?}: pixels went missing"
            );
        }
    }

    #[test]
    fn the_best_shm_format_wins_over_the_advertised_order() {
        use wl_shm::Format;

        let all = [
            Format::Abgr8888,
            Format::Xbgr8888,
            Format::Argb8888,
            Format::Xrgb8888,
        ];
        assert_eq!(pick_format(&all), Some((Format::Xrgb8888, true)));

        assert_eq!(
            pick_format(&[Format::Abgr8888, Format::Argb8888]),
            Some((Format::Argb8888, true))
        );
        assert_eq!(
            pick_format(&[Format::Xbgr8888]),
            Some((Format::Xbgr8888, false))
        );
        assert_eq!(pick_format(&[]), None);
        assert_eq!(pick_format(&[Format::Rgb565]), None);
    }

    #[test]
    fn padded_and_inverted_rows_come_back_tight_and_upright() {
        let canvas: Vec<u8> = (1..=3u8)
            .flat_map(|y| [y, y, y, y, y, y, y, y, 0xEE, 0xEE, 0xEE, 0xEE])
            .collect();

        let rows = |pixels: &[u8]| -> Vec<u8> { pixels.chunks(8).map(|r| r[0]).collect() };

        let straight = read_rows(&canvas, 2, 3, 12, false);
        assert_eq!(straight.len(), 2 * 3 * 4);
        assert!(!straight.contains(&0xEE), "row padding leaked");
        assert_eq!(rows(&straight), [1, 2, 3]);

        let inverted = read_rows(&canvas, 2, 3, 12, true);
        assert_eq!(rows(&inverted), [3, 2, 1]);
    }
}
