//! 【内存生命周期回归】【捕获线程】大临时帧释放后不能继续保留在已经结束的线程内存区

#![cfg(all(target_os = "linux", target_env = "gnu"))]

use std::{hint::black_box, process::Command, sync::Arc, sync::Barrier};

const WORKER_ENV: &str = "MARK_SHOT_CAPTURE_ALLOCATION_WORKER";
const FRAME_BYTES: usize = 3840 * 2160 * 4;
const MAX_RETAINED_KIB: usize = 16 * 1024;

/// 【内存生命周期回归】【私有常驻页】无参数，统计当前进程可写匿名映射和堆的RSS，返回KiB，不计入程序文件与具名栈映射
fn private_pages() -> usize {
    let maps = std::fs::read_to_string("/proc/self/smaps").unwrap();
    let mut included = false;
    let mut resident = 0;
    for line in maps.lines() {
        let columns: Vec<_> = line.split_whitespace().collect();
        if columns.first().is_some_and(|value| value.contains('-')) {
            included = columns.get(1) == Some(&"rw-p")
                && (columns.len() == 5 || columns.get(5) == Some(&"[heap]"));
        } else if included && columns.first() == Some(&"Rss:") {
            resident += columns[1].parse::<usize>().unwrap();
        }
    }
    resident
}

/// 【内存生命周期回归】【线程帧释放】无参数，复现首帧释放后的动态阈值与两个捕获线程，返回线程结束后的新增常驻页KiB
fn exercise() -> usize {
    super::configure();
    // 1. 【内存生命周期回归】【前序捕获】先释放一份真实4K大小的帧，复现GNU分配器随后提高映射阈值的行为
    let mut prior = vec![0_u8; FRAME_BYTES];
    prior.fill(91);
    black_box(&prior);
    drop(prior);
    let baseline = private_pages();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let retained = vec![index as u8; 512];
                let mut pixels = vec![0_u8; FRAME_BYTES];
                pixels.fill(127);
                black_box(&pixels);
                barrier.wait();
                drop(pixels);
                retained
            })
        })
        .collect();
    // 2. 【内存生命周期回归】【捕获关闭】工作线程全部结束，继续保留少量真实回调大小的对象，并执行现有空闲页回收
    let retained: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    super::release_unused();
    black_box(&retained);
    let remaining = private_pages().saturating_sub(baseline);
    println!("retained_capture_private_kib={remaining}, limit_kib={MAX_RETAINED_KIB}");
    remaining
}

/// 【内存生命周期回归】【独立进程】无参数，在新进程验证捕获线程关闭后的实际页释放，超过预算时失败，无返回值
#[test]
fn closed_capture_threads_release_large_frame_pages() {
    if std::env::var_os(WORKER_ENV).is_some() {
        let remaining = exercise();
        assert!(
            remaining <= MAX_RETAINED_KIB,
            "closed capture threads retained {remaining} KiB"
        );
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .env(WORKER_ENV, "1")
        .env_remove("GLIBC_TUNABLES")
        .env_remove("MALLOC_MMAP_THRESHOLD_")
        .args([
            "--exact",
            "utils::memory::tests::closed_capture_threads_release_large_frame_pages",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
