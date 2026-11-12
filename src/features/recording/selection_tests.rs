//! 【录制选区回归】【多屏映射】通过生产坐标换算验证负原点、非主屏、混合缩放及跨屏拒绝

use super::region_crop;
use crate::types::Placement;
use tiny_skia::Rect;

/// 【录制选区回归】【输出布局】无参数，返回左侧负原点和右侧分数缩放显示器
fn outputs() -> [Placement; 2] {
    [
        Placement {
            position: (-1280, -100),
            size: (1280, 720),
            scale: 1.5,
        },
        Placement {
            position: (0, 0),
            size: (1536, 864),
            scale: 1.25,
        },
    ]
}

/// 【录制选区回归】【像素来源】无参数，选中哪个显示器就返回哪个表面，不将非主屏裁剪到原点屏
#[test]
fn negative_origins_and_mixed_scales_keep_surface_identity() {
    let placements = outputs();
    for (region, expected) in [
        ((-1200.0, -60.0, 400.0, 200.0), (0, [120, 60, 600, 300])),
        ((80.0, 40.0, 400.0, 200.0), (1, [100, 50, 500, 250])),
        ((0.0, 0.0, 1536.0, 864.0), (1, [0, 0, 1920, 1080])),
    ] {
        let rect = Rect::from_xywh(region.0, region.1, region.2, region.3).unwrap();
        assert_eq!(region_crop(rect, &placements).unwrap(), expected);
    }
    let only_secondary = &placements[1..];
    assert_eq!(
        region_crop(
            Rect::from_xywh(80.0, 40.0, 400.0, 200.0).unwrap(),
            only_secondary
        )
        .unwrap(),
        (0, [100, 50, 500, 250])
    );
}

/// 【录制选区回归】【边界拒绝】无参数，跨显示器或过小区域返回错误，奇数像素只向内对齐不越界
#[test]
fn crossing_outputs_is_rejected_and_video_dimensions_round_inward() {
    let placements = outputs();
    assert!(
        region_crop(
            Rect::from_xywh(-10.0, 10.0, 20.0, 20.0).unwrap(),
            &placements
        )
        .is_err()
    );
    assert!(region_crop(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(), &placements).is_err());
    assert_eq!(
        region_crop(Rect::from_xywh(0.0, 0.0, 99.0, 101.0).unwrap(), &placements).unwrap(),
        (1, [0, 0, 124, 126])
    );
}
