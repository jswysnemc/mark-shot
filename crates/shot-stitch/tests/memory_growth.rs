//! 【长图内存回归】【真实增长】通过拼接入口核对双轴分配上限和完整原始像素

use shot_image::Image;
use shot_stitch::{Axis, Config, Status, Stitcher};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 256;
const STEPS: i32 = 48;
const STEP: i32 = 64;
const MAX_BLOCK_BYTES: usize = 4 * 1024 * 1024;

/// 【长图内存回归】【固定纹理】按origin、width和height返回独立整数纹理，像素与原生页面公式一致
fn frame(origin: i32, width: i32, height: i32) -> Image {
    let pixels = (origin..origin + height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                    ^ (x as u32).wrapping_mul(2_246_822_519);
                0xff00_0000 | (hash & 0x00ff_ffff)
            })
        })
        .collect();
    Image::from_pixels(width, height, pixels).unwrap()
}

/// 【长图内存回归】【增长边界】无参数，检查真实拼接的最大单次分配保持有界，双轴长图每个像素必须一致，无返回值
#[test]
fn growing_in_both_axes_keeps_individual_pixel_allocations_bounded() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let mut stitcher = Stitcher::new(Config {
            min_overlap: 64,
            min_append: 16,
            accept_diff: 0.5,
            approx_diff: 0.1,
        })
        .unwrap();
        stitcher.set_axis(axis).unwrap();
        for step in 0..=STEPS {
            let image = frame(step * STEP, WIDTH, HEIGHT);
            let image = if axis == Axis::Horizontal {
                image.transpose().unwrap()
            } else {
                image
            };
            let status = stitcher.push(image).unwrap().status;
            assert!(matches!(status, Status::FirstFrame | Status::Appended));
            let storage = stitcher.pixel_storage();
            assert!(
                storage.largest_allocation_bytes <= MAX_BLOCK_BYTES,
                "axis={axis:?}, step={step}, storage={storage:?}: growth still replaces a complete long buffer"
            );
            assert!(storage.allocated_bytes <= storage.used_bytes + MAX_BLOCK_BYTES * 2);
        }
        let expected = frame(0, WIDTH, HEIGHT + STEPS * STEP);
        let expected = if axis == Axis::Horizontal {
            expected.transpose().unwrap()
        } else {
            expected
        };
        let view = stitcher.image_view().unwrap();
        assert_eq!(
            (view.width(), view.height()),
            (expected.width(), expected.height())
        );
        for y in 0..view.height() {
            for x in 0..view.width() {
                assert_eq!(
                    view.pixel(x, y),
                    expected.row(y)[x as usize],
                    "axis={axis:?}, x={x}, y={y}"
                );
            }
        }
    }
}
