//! 【截图前景】【驻留调度】限制透明页检查频率，空闲后归还透明页或已交付的不透明CPU画布

use std::time::{Duration, Instant};
use tiny_skia::Pixmap;

const INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy)]
struct Pending {
    deadline: Instant,
    opaque: bool,
}

/// 【截图前景】【空闲回收】按 state、pacing、dirty_mask 与 now 归还透明页或已交付CPU画布，返回已替换画布的输出索引；保留有效导航背景
pub(super) fn reclaim(
    state: &mut crate::editor::EditorState,
    pacing: &mut super::frame_render::Pacing,
    dirty_mask: u32,
    now: Instant,
) -> Vec<usize> {
    let owed = &pacing.owed;
    let reclaimed = pacing
        .foreground_residency
        .reclaim_due(&mut state.canvas, now, |monitor| {
            !crate::editor::dirty::is_dirty(dirty_mask, monitor) && owed[monitor].is_none()
        });
    // 1. 【截图前景】【恢复性能】导航背景只在视图或选区变化时失效，保留已采样块，避免空闲后重复执行大范围插值
    if !reclaimed.is_empty() {
        crate::utils::memory::release_unused();
    }
    reclaimed
}

#[derive(Default)]
pub(super) struct Residency {
    deadlines: Vec<Option<Pending>>,
    regional_deadlines: Vec<Option<Instant>>,
}

impl Residency {
    /// 【截图前景】【透明检查许可】按 monitor 和 now 返回是否允许本次区域回收，各输出独立限制检查频率
    pub(super) fn regional_due(&mut self, monitor: usize, now: Instant) -> bool {
        if self.regional_deadlines.len() <= monitor {
            self.regional_deadlines.resize(monitor + 1, None);
        }
        let deadline = &mut self.regional_deadlines[monitor];
        if deadline.is_some_and(|deadline| now < deadline) {
            return false;
        }
        *deadline = Some(now + INTERVAL);
        true
    }

    /// 【截图前景】【交付续期】按 monitor、native_scene、opaque 和 now 安排空闲回收，非原生场景取消旧期限，无返回值
    pub(super) fn touched(
        &mut self,
        monitor: usize,
        native_scene: bool,
        opaque: bool,
        now: Instant,
    ) {
        if self.deadlines.len() <= monitor {
            self.deadlines.resize(monitor + 1, None);
        }
        self.deadlines[monitor] = native_scene.then_some(Pending {
            deadline: now + INTERVAL,
            opaque,
        });
    }

    /// 【截图前景】【空闲等待】按 now 和 ready 返回最早回收时间，未交付输出不安排零期限等待
    pub(super) fn timeout(&self, now: Instant, ready: impl Fn(usize) -> bool) -> Option<Duration> {
        self.deadlines
            .iter()
            .enumerate()
            .filter_map(|(monitor, deadline)| {
                deadline
                    .filter(|_| ready(monitor))
                    .map(|pending| pending.deadline.saturating_duration_since(now))
            })
            .min()
    }

    /// 【截图前景】【画布归还】按 canvases、now 和 ready 回收空闲前景，返回已替换的不透明画布索引；透明场景保留非零像素，分配失败保留原画布
    pub(super) fn reclaim_due(
        &mut self,
        canvases: &mut [Pixmap],
        now: Instant,
        ready: impl Fn(usize) -> bool,
    ) -> Vec<usize> {
        let mut reclaimed = Vec::new();
        self.regional_deadlines.resize(self.deadlines.len(), None);
        for (monitor, deadline) in self.deadlines.iter_mut().enumerate() {
            if !deadline.is_some_and(|pending| now >= pending.deadline) || !ready(monitor) {
                continue;
            }
            let pending = deadline.take().unwrap();
            let Some(canvas) = canvases.get_mut(monitor) else {
                continue;
            };
            if !pending.opaque {
                // 1. 【截图前景】【空闲透明页】连续输入只定期检查局部范围，空闲后检查全部自有页并保留仍然可见的前景
                let area =
                    tiny_skia::IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
                crate::utils::transparent_pixmap::reclaim_region(canvas, area);
                self.regional_deadlines[monitor] = Some(now + INTERVAL);
                continue;
            }
            let Some(mut blank) = Pixmap::new(canvas.width(), canvas.height()) else {
                continue;
            };
            // 2. 【截图前景】【独立所有权】原生帧已保存完整像素，CPU画布没有外部借用，替换后释放旧分配
            crate::utils::transparent_pixmap::prepare_sparse(&mut blank);
            #[cfg(target_os = "linux")]
            crate::utils::transparent_pixmap::clear(&mut blank);
            *canvas = blank;
            reclaimed.push(monitor);
        }
        reclaimed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图前景测试】【检查频率】连续帧每250毫秒至多获得一次检查许可，其他输出不受当前输出时间影响
    #[test]
    fn regional_reclamation_waits_for_the_interval_and_keeps_outputs_independent() {
        let now = Instant::now();
        let mut residency = Residency::default();
        assert!(residency.regional_due(0, now));
        assert!(!residency.regional_due(0, now + Duration::from_millis(249)));
        assert!(residency.regional_due(1, now + Duration::from_millis(20)));
        assert!(residency.regional_due(0, now + Duration::from_millis(250)));
        assert!(!residency.regional_due(1, now + Duration::from_millis(250)));
        assert!(residency.regional_due(1, now + Duration::from_millis(270)));
    }

    /// 【截图前景测试】【空闲透明页】空闲后实际归还透明自有页并保留前景，未提交输出不变，下一帧不重复查询
    #[test]
    fn idle_sparse_reclamation_keeps_visible_and_pending_pixels_and_stops_repeated_waits() {
        let now = Instant::now();
        let mut canvases = vec![
            Pixmap::new(513, 257).unwrap(),
            Pixmap::new(513, 257).unwrap(),
        ];
        // 1. 【截图前景测试】【独占透明页】显式写入零值建立真实自有页，首个不透明像素验证回收不丢失前景
        canvases[0].data_mut().fill(0);
        canvases[0].data_mut()[..4].copy_from_slice(&[91, 110, 20, 255]);
        canvases[1].data_mut().fill(137);
        let foreground = canvases[0].data().to_vec();
        let address = canvases[0].data().as_ptr();
        #[cfg(target_os = "linux")]
        let private = {
            use std::os::unix::fs::FileExt;
            let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
            let position = (address as usize).div_ceil(page) + 1;
            let entries = std::fs::File::open("/proc/self/pagemap").unwrap();
            move || {
                let mut value = [0; 8];
                entries
                    .read_exact_at(&mut value, (position * 8) as u64)
                    .unwrap();
                let value = u64::from_ne_bytes(value);
                value & ((1 << 63) | (1 << 56)) == ((1 << 63) | (1 << 56))
            }
        };
        #[cfg(target_os = "linux")]
        assert!(private());
        let mut residency = Residency::default();
        for monitor in 0..2 {
            residency.touched(monitor, true, false, now);
        }
        // 2. 【截图前景测试】【输出边界】到期只处理已交付输出，透明画布地址与非零字节保持原样
        assert!(
            residency
                .reclaim_due(&mut canvases, now + INTERVAL, |monitor| monitor == 0)
                .is_empty()
        );
        #[cfg(target_os = "linux")]
        assert!(!private());
        assert_eq!(canvases[0].data().as_ptr(), address);
        assert!(
            canvases[0].data() == foreground,
            "visible foreground changed"
        );
        assert!(canvases[1].data().iter().all(|byte| *byte == 137));
        // 3. 【截图前景测试】【等待取消】完成最终清理后不继续唤醒，下一帧在区域检查间隔内复用当前映射
        assert!(
            residency
                .timeout(now + INTERVAL, |monitor| monitor == 0)
                .is_none()
        );
        assert!(!residency.regional_due(0, now + INTERVAL + Duration::from_millis(1)));
    }

    /// 【截图前景测试】【未交付保留】到期只回收已提交输出，保持尺寸并停止重复唤醒，未交付像素完整保留
    #[test]
    fn idle_replacement_keeps_pending_outputs_and_resets_committed_pixels() {
        let now = Instant::now();
        let mut canvases = vec![
            Pixmap::new(513, 257).unwrap(),
            Pixmap::new(513, 257).unwrap(),
        ];
        for canvas in &mut canvases {
            canvas.data_mut().fill(137)
        }
        let mut residency = Residency::default();
        for monitor in 0..2 {
            residency.touched(monitor, true, true, now)
        }
        assert!(
            residency
                .reclaim_due(&mut canvases, now + Duration::from_millis(249), |_| true)
                .is_empty()
        );
        assert_eq!(
            residency.reclaim_due(
                &mut canvases,
                now + Duration::from_millis(250),
                |monitor| monitor == 0
            ),
            vec![0]
        );
        assert!(canvases[0].data().iter().all(|byte| *byte == 0));
        assert!(canvases[1].data().iter().all(|byte| *byte == 137));
        assert_eq!((canvases[0].width(), canvases[0].height()), (513, 257));
        assert!(
            residency
                .timeout(now + Duration::from_millis(300), |monitor| monitor == 0)
                .is_none()
        );
    }
}
