//! 【尺子说明测试】【绘制成本】完整标签覆盖大字号、固定分块、移位裁切与实际分配量

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};

use mark_shot::{
    features::ruler::{Measurement, RulerGeometry, RulerMode, labels::prepare_all},
    renderer::ruler_overlay,
};
use tiny_skia::{Color, Pixmap, Rect};

struct AllocationCounter;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// 【尺子说明测试】【分配记录】按size记录测量区间中的分配次数和请求字节，无返回值
fn record(size: usize) {
    if TRACK.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(size as u64, Ordering::Relaxed);
    }
}

// 1. 【尺子说明测试】【透明统计】所有内存操作交给系统分配器，计数仅在单线程性能测量区间开启
unsafe impl GlobalAlloc for AllocationCounter {
    /// 【尺子说明测试】【内存分配】按layout使用系统分配器并记录字节，返回原始地址
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }

    /// 【尺子说明测试】【内存释放】按ptr和layout释放系统分配器地址，无返回值
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    /// 【尺子说明测试】【清零分配】按layout分配清零内存并记录字节，返回原始地址
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    /// 【尺子说明测试】【内存调整】按ptr、layout和size调整系统内存并记录请求，返回新地址
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: AllocationCounter = AllocationCounter;

/// 【尺子说明测试】【标签环境】按unit返回完整测量说明，使用原版完整单行尺寸及整数像素字号
fn labels(unit: f64) -> Vec<mark_shot::features::ruler::labels::InfoLabel> {
    let mut measurement = Measurement::new(RulerMode::Length, (200.5, 200.5), 1.0).unwrap();
    measurement.readouts = mark_shot::features::ruler::Readouts::ALL;
    measurement.geometry = RulerGeometry::Length {
        start: (200.5, 200.5),
        end: (450.5, 380.5),
        rotation: 37.0,
    };
    measurement.style_scale = unit;
    prepare_all(
        std::iter::once((measurement, 3.0)),
        Rect::from_xywh(-37.0, 21.0, 1280.0, 800.0).unwrap(),
        &mut mark_shot::fonts::bundled(),
    )
}

/// 【尺子说明测试】【多行绘制】无参数，检查每一行都绘出了字形，且各行文本左对齐，无返回值
#[test]
fn multiline_card_paints_visible_text_on_every_row() {
    let labels = labels(1.0);
    let card = &labels[0];
    assert!(card.lines.len() > 1);
    let mut canvas = Pixmap::new(900, 650).unwrap();
    ruler_overlay::draw_labels(
        &mut mark_shot::renderer::Painter::new(&mut canvas).with_text(
            &mut mark_shot::fonts::bundled(),
            &mut cosmic_text::SwashCache::new(),
        ),
        &labels,
    );
    let rows = card.lines.len();
    let padding = 5.0 * card.unit;
    let stride = (card.rect.height() - 2.0 * padding + 2.0 * card.unit) / rows as f32;
    for row in 0..rows {
        let top = card.rect.top() + padding + row as f32 * stride;
        let ink = (card.rect.left().ceil() as u32..card.rect.right().floor() as u32)
            .find(|&column| {
                (top.ceil() as u32..(top + stride - 2.0 * card.unit).floor() as u32).any(|line| {
                    canvas
                        .pixel(column, line)
                        .is_some_and(|pixel| pixel.green() > 160)
                })
            })
            .expect("each readout row must contain visible glyphs");
        assert!((ink as f32 - card.rect.left() - 10.0 * card.unit).abs() < 4.0);
    }
}

/// 【尺子说明测试】【大字号局部帧】原版整数字号和非整数原点下，局部重绘保持完整帧的每个像素
#[test]
fn full_and_partial_labels_match_at_small_and_large_integer_fonts() {
    let origin = (-37.0, 21.0);
    let scale = 1.2;
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    for unit in [0.125, 1.0, 14.0] {
        let labels = labels(unit);
        let mut base = Pixmap::new(1536, 960).unwrap();
        base.fill(Color::from_rgba8(70, 80, 90, 255));
        let mut full = base.clone();
        ruler_overlay::draw_clipped(
            &mut full, &labels, None, origin, scale, None, &mut fonts, &mut cache,
        );
        // 1. 【尺子说明测试】【优化前后像素】显式指定目录时保存完整帧，供分配优化前后的独立指纹比较
        if let Ok(root) = std::env::var("MARK_SHOT_RULER_LABEL_COST_FRAMES") {
            full.save_png(std::path::Path::new(&root).join(format!("unit-{unit}.png")))
                .unwrap();
        }
        let rect = labels[0].rect;
        for shift in [(0.0, 0.0), (23.0, 17.0), (93.0, 3.0)] {
            let damage = Rect::from_xywh(
                rect.left() - origin.0 + shift.0,
                rect.top() - origin.1 + shift.1,
                151.0,
                43.0,
            )
            .unwrap();
            let pixels = mark_shot::renderer::scale_rect(&damage, scale)
                .unwrap()
                .round_out()
                .unwrap();
            let Some(blank) = base.clone_rect(pixels) else {
                continue;
            };
            let mut partial = full.clone();
            mark_shot::renderer::paste(&mut partial, &blank, pixels.x(), pixels.y());
            ruler_overlay::draw_clipped(
                &mut partial,
                &labels,
                None,
                origin,
                scale,
                Some(&damage),
                &mut fonts,
                &mut cache,
            );
            assert!(
                partial.data() == full.data(),
                "unit={unit} shift={shift:?} first mismatch={:?}",
                partial
                    .pixels()
                    .iter()
                    .zip(full.pixels())
                    .position(|(a, b)| a != b)
            );
        }
    }
}

/// 【尺子说明测试】【分块成本】按原版小、中、大字号测量完整叠加与96乘48窗口损坏范围，输出耗时及请求分配量
#[test]
#[ignore = "requires isolated release performance run with one test thread"]
fn measure_ruler_label_render_cost() {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    for scale in [1.0, 1.2] {
        for unit in [0.125, 1.0, 14.0] {
            let labels = labels(unit);
            for partial in [false, true] {
                let mut canvas =
                    Pixmap::new((1280.0 * scale) as u32, (800.0 * scale) as u32).unwrap();
                let damage = Rect::from_xywh(
                    labels[0].rect.left() + 32.0,
                    labels[0].rect.top(),
                    96.0,
                    48.0,
                )
                .unwrap();
                let dirty = partial.then_some(&damage);
                // 1. 【尺子说明测试】【预热】先初始化相同字形缓存，测量区间只覆盖稳定绘制
                ruler_overlay::draw_clipped(
                    &mut canvas,
                    &labels,
                    None,
                    (0.0, 0.0),
                    scale,
                    dirty,
                    &mut fonts,
                    &mut cache,
                );
                let mut times = Vec::with_capacity(40);
                ALLOCATIONS.store(0, Ordering::Relaxed);
                BYTES.store(0, Ordering::Relaxed);
                TRACK.store(true, Ordering::Relaxed);
                for _ in 0..40 {
                    let start = Instant::now();
                    ruler_overlay::draw_clipped(
                        &mut canvas,
                        &labels,
                        None,
                        (0.0, 0.0),
                        scale,
                        dirty,
                        &mut fonts,
                        &mut cache,
                    );
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                TRACK.store(false, Ordering::Relaxed);
                times.sort_by(f64::total_cmp);
                println!(
                    "{{\"scale\":{scale},\"unit\":{unit},\"partial\":{partial},\"median_ms\":{},\"p95_ms\":{},\"allocations_per_frame\":{},\"requested_bytes_per_frame\":{}}}",
                    times[20],
                    times[38],
                    ALLOCATIONS.load(Ordering::Relaxed) / 40,
                    BYTES.load(Ordering::Relaxed) / 40
                );
            }
        }
    }
}
