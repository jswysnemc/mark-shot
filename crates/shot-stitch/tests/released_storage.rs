//! 【长图内存回归】【交接回收】隔离真实增长与逐块交付，检查小对象不能阻止长图像素页释放

#![cfg(all(target_os = "linux", target_env = "gnu"))]

use shot_image::Image;
use shot_stitch::{Config, Stitcher};
use std::{hint::black_box, process::Command};

const WORKER_ENV: &str = "MARK_SHOT_STORAGE_RELEASE_WORKER";
const WIDTH: i32 = 2048;
const HEIGHT: i32 = 256;
const STEPS: i32 = 128;
const STEP: i32 = 64;
const MAX_RETAINED_KIB: usize = 16 * 1024;

/// 【长图内存回归】【堆常驻页】无参数，从当前进程映射读取全部可写私有堆及匿名页，返回KiB，不计入程序文件或具名栈映射
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

/// 【长图内存回归】【纹理帧】按origin生成固定尺寸原始帧并返回ARGB图像，避免使用拼接结果作为期望来源
fn frame(origin: i32) -> Image {
    let pixels = (origin..origin + HEIGHT)
        .flat_map(|y| {
            (0..WIDTH).map(move |x| {
                let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                    ^ (x as u32).wrapping_mul(2_246_822_519);
                0xff00_0000 | (hash & 0x00ff_ffff)
            })
        })
        .collect();
    Image::from_pixels(WIDTH, HEIGHT, pixels).unwrap()
}

/// 【长图内存回归】【隔离场景】无参数，模拟先释放整屏缓冲再增长长图，保留交错小对象，返回交接后新增常驻页KiB
fn exercise() -> usize {
    // 1. 【长图内存回归】【分配器前序】真实捕获先释放大像素缓冲，GNU分配器可能随后将较小行块放入堆
    let mut prior = vec![0_u8; 24 * 1024 * 1024];
    prior.fill(93);
    black_box(&prior);
    drop(prior);
    let baseline = private_pages();
    let mut stitcher = Stitcher::new(Config {
        min_overlap: 64,
        min_append: 16,
        accept_diff: 0.5,
        approx_diff: 0.1,
    })
    .unwrap();
    let mut retained = Vec::new();
    // 2. 【长图内存回归】【真实增长】持续帧和界面小对象交错分配，交接结束后这些对象仍然有效
    for step in 0..=STEPS {
        stitcher.push(frame(step * STEP)).unwrap();
        retained.push(vec![step as u8; 256]);
    }
    let image = stitcher.take_image().unwrap();
    assert_eq!(
        (image.width(), image.height()),
        (WIDTH, HEIGHT + STEPS * STEP)
    );
    let mut output = vec![0; image.width() as usize * image.height() as usize * 4];
    image.write_rgba(&mut output).unwrap();
    assert_eq!(&output[..4], &[0, 0, 0, 255]);
    black_box(&output);
    drop(output);
    drop(stitcher);
    // 3. 【长图内存回归】【资源边界】原图和最终结果均已释放，保留小对象只允许少量工作页继续常驻
    black_box(&retained);
    let remaining = private_pages().saturating_sub(baseline);
    println!("retained_private_kib={remaining}, limit_kib={MAX_RETAINED_KIB}");
    remaining
}

/// 【长图内存回归】【独立进程】无参数，在新进程内执行真实生命周期，交接后私有页超预算即失败，无返回值
#[test]
fn released_pixel_blocks_do_not_remain_pinned_by_small_objects() {
    if std::env::var_os(WORKER_ENV).is_some() {
        let remaining = exercise();
        assert!(
            remaining <= MAX_RETAINED_KIB,
            "released scroll storage retained {remaining} KiB of private pages"
        );
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .env(WORKER_ENV, "1")
        .args([
            "--exact",
            "released_pixel_blocks_do_not_remain_pinned_by_small_objects",
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
