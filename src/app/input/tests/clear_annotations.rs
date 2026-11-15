//! 【标注清除】【画面回归】通过真实垃圾桶按钮与原生帧交付验证标注删除后的底图恢复

use crate::{
    app::{
        frame_lifecycle_overlay::Overlay,
        frame_render::{Pacing, clear_frame_state, render_dirty_monitors},
        region_rendering_test_support as support,
    },
    editor::EditorState,
    features::filter::{FilterMode, RegionFilter},
    tools::Tool,
    types::{AnnotationShape, MouseButton},
    ui::toolbar::{ToolbarButton, ToolbarItem},
};
use tiny_skia::Rect;

struct Harness {
    state: EditorState,
    overlay: Overlay,
    pacing: Pacing,
}

impl Harness {
    /// 【标注清除】【场景构造】按 region、native 与 mode 返回已绘制矩形或滤镜、工具栏可点击的测试环境
    fn new(region: bool, native: bool, mode: Option<FilterMode>) -> Self {
        // 1. 【标注清除】【标注准备】正文与工具栏分开放置，避免按钮重绘恰好覆盖旧标注
        let mut state = support::editor(1.0, region);
        state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
        state.fullscreen_scope = true;
        state.selected_tool = Tool::Pick;
        let shape = if let Some(mode) = mode {
            AnnotationShape::Redact {
                filter: RegionFilter::new((100.0, 240.0), (250.0, 330.0), 14.0, mode).unwrap(),
            }
        } else {
            AnnotationShape::Rectangle {
                start: (100.0, 240.0),
                end: (250.0, 330.0),
                filled: true,
                radius: 0.0,
                fixed_ratio: false,
            }
        };
        let mut annotation = support::annotation(1, 0.0, shape);
        annotation.frozen_bounds = state.placements[0].rect();
        state.annotations.push(annotation);
        state.annotations.push(support::annotation(
            2,
            24.0,
            AnnotationShape::Rectangle {
                start: (340.0, 260.0),
                end: (420.0, 350.0),
                filled: false,
                radius: 6.0,
                fixed_ratio: false,
            },
        ));
        // 2. 【标注清除】【面板准备】固定工具栏位置与显示进度，首次帧完成后再发送点击
        crate::app::input::refresh_panels(&mut state, &mut 0);
        state.toolbar.user_position = Some((8.0, 8.0));
        crate::app::input::refresh_panels(&mut state, &mut 0);
        state.toolbar.opacity = 1.0;
        state.toolbar.render_pos = state.toolbar.position;
        state.action_toolbar.opacity = 1.0;
        state.action_toolbar.render_pos = state.action_toolbar.position;
        // 3. 【标注清除】【交付准备】使用生产像素交付路径记录画面，不连接实际桌面
        let mut overlay = Overlay::new(&state.canvas, false);
        crate::app::frame_backdrop::configure(&mut state, &mut overlay, true);
        state.region_rendering.native_scene = native;
        let mut harness = Self {
            state,
            overlay,
            pacing: Pacing::new(1),
        };
        harness.render(1);
        harness
    }

    /// 【标注清除】【真实帧】按 dirty 提交一帧并清理帧状态，无返回值，保留未损坏区域的输出像素
    fn render(&mut self, mut dirty: u32) {
        // 1. 【标注清除】【帧交付】执行缓存重建、区域合成与后端提交
        render_dirty_monitors(
            &mut self.state,
            &mut self.overlay,
            dirty,
            false,
            &mut self.pacing,
        )
        .unwrap();
        // 2. 【标注清除】【帧清理】移除已完成的损坏记录，避免下一帧沿用首次完整重绘
        clear_frame_state(&mut self.state, &mut dirty, &self.pacing);
    }

    /// 【标注清除】【按钮点击】无参数，通过实际工具栏命中发送完整左键点击，返回重绘掩码
    fn clear(&mut self) -> u32 {
        // 1. 【标注清除】【命中定位】按真实布局计算垃圾桶中心，不直接调用清除动作
        let index = self
            .state
            .toolbar
            .items
            .iter()
            .position(|item| *item == ToolbarItem::Button(ToolbarButton::Clear))
            .unwrap();
        assert!(!self.state.toolbar.disabled.contains(&index));
        let cell = self.state.toolbar.cell(index).unwrap();
        let point = (
            f64::from(self.state.toolbar.render_pos.0 + cell.left() + cell.width() / 2.0),
            f64::from(self.state.toolbar.render_pos.1 + cell.top() + cell.height() / 2.0),
        );
        // 2. 【标注清除】【输入校验】完整点击必须清空标注与选中状态，同时保留截图选区
        let zone = self.state.selection.zone;
        let mut dirty = 0;
        for pressed in [true, false] {
            support::pointer(
                &mut self.state,
                point,
                Some((MouseButton::Left, pressed)),
                &mut dirty,
            );
        }
        assert!(self.state.annotations.is_empty() && self.state.pending.is_none());
        assert!(self.state.selected_annotation_ids().is_empty());
        assert_eq!(self.state.selection.zone, zone);
        dirty
    }
}

/// 【标注清除】【完整帧对照】按 actual、expected 和两者重绘掩码提交局部与完整帧，逐字节比较交付结果，无返回值
fn compare_frames(actual: &mut Harness, expected: &mut Harness, dirty: u32, expected_dirty: u32) {
    // 1. 【标注清除】【完整参考】仅为参考场景补充整屏重绘，实际场景使用清除动作给出的范围
    expected
        .state
        .damage_local(0, expected.state.placements[0].rect().unwrap());
    actual.render(dirty);
    expected.render(expected_dirty);
    // 2. 【标注清除】【像素对照】逐字节比较真实交付内容，工具栏状态相同而旧标注残留时仍会失败
    let difference = actual.overlay.frames[0]
        .iter()
        .zip(&expected.overlay.frames[0])
        .position(|(actual, expected)| actual != expected);
    assert_eq!(
        difference, None,
        "cleared annotation remains in the submitted frame"
    );
}

/// 【标注清除】【局部完整对照】无参数，普通标注、像素滤镜和两种聚焦在缓存、区域及原生场景下都必须恢复底图
#[test]
fn clear_button_restores_pixels_outside_the_toolbar() {
    for (region, native) in [(false, false), (true, false), (true, true)] {
        for mode in [
            None,
            Some(FilterMode::Pixelate),
            Some(FilterMode::Focus),
            Some(FilterMode::FocusEllipse),
        ] {
            let mut actual = Harness::new(region, native, mode);
            let mut expected = Harness::new(region, native, mode);
            let dirty = actual.clear();
            let expected_dirty = expected.clear();
            compare_frames(&mut actual, &mut expected, dirty, expected_dirty);
            assert_eq!(actual.state.filters.retained_bytes(), 0);
        }
    }
}

/// 【标注清除】【历史画面】无参数，选中标注时清除全部内容，一次撤销恢复所有标注，再次重做清空且帧像素一致
#[test]
fn clear_undo_and_redo_restore_all_annotations_and_their_pixels() {
    for native in [false, true] {
        let mut actual = Harness::new(true, native, Some(FilterMode::Focus));
        let mut expected = Harness::new(true, native, Some(FilterMode::Focus));
        for harness in [&mut actual, &mut expected] {
            harness.state.select_annotation_ids([2]);
            crate::app::input::refresh_panels(&mut harness.state, &mut 0);
            harness.render(1);
        }
        let annotations = actual.state.annotations.clone();
        let dirty = actual.clear();
        let expected_dirty = expected.clear();
        compare_frames(&mut actual, &mut expected, dirty, expected_dirty);
        for key in ["Ctrl+Z", "Ctrl+Shift+Z"] {
            let (mut dirty, mut expected_dirty) = (0, 0);
            support::key(&mut actual.state, key, &mut dirty);
            support::key(&mut expected.state, key, &mut expected_dirty);
            compare_frames(&mut actual, &mut expected, dirty, expected_dirty);
            if key == "Ctrl+Z" {
                assert_eq!(actual.state.annotations, annotations);
            } else {
                assert!(actual.state.annotations.is_empty());
            }
        }
    }
}
