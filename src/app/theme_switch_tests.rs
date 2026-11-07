//! 【主题刷新测试】【存活状态】验证完整输出损坏、弹层失效与正文状态保持，不修改全局主题

use super::refresh;
use crate::{editor::DamageZone, ui::font_panel::state::Field};
use tiny_skia::{Pixmap, Rect};

/// 【主题刷新测试】【局部正文】无参数，已有局部损坏不能缩小主题刷新范围，整个输出必须重绘，无返回值
#[test]
fn refresh_covers_the_whole_output_when_document_damage_is_pending() {
    let mut state = crate::app::region_rendering_test_support::editor(1.25, true);
    state.damage_rects.push(DamageZone::Local {
        monitor_idx: 0,
        rect: Rect::from_xywh(300.0, 200.0, 50.0, 30.0).unwrap(),
    });
    let mut dirty = 0;
    refresh(&mut state, &mut dirty);
    assert_eq!(dirty, 1);
    assert_eq!(
        state.monitor_dirty_rect(0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0)
    );
}

/// 【主题刷新测试】【字段保存】无参数，清理字号、字体和模型缓存但保留搜索、字库索引与用户样式，无返回值
#[test]
fn refresh_invalidates_font_settings_and_model_without_resetting_inputs() {
    let mut state = crate::app::region_rendering_test_support::editor(1.0, true);
    state.font_panel.open = true;
    state.font_panel.family = "Inter".into();
    state.font_panel.offset = 84.0;
    state.font_panel.preview.key = vec![4, 5, 6];
    state
        .font_panel
        .fields
        .values
        .insert(Field::Search, "Inter".into());
    state.font_panel.pixmap = Pixmap::new(360, 360);
    state.font_panel.dirty = false;
    state.settings_panel.panel_pixmap = Pixmap::new(400, 40);
    state.settings_panel.dirty = false;
    state.model_popover.pixmap = Pixmap::new(200, 300);
    state.model_popover.dirty = false;
    refresh(&mut state, &mut 0);
    assert!(state.font_panel.pixmap.is_none() && state.font_panel.dirty);
    assert!(state.settings_panel.panel_pixmap.is_none() && state.settings_panel.dirty);
    assert!(state.model_popover.pixmap.is_none() && state.model_popover.dirty);
    assert_eq!(state.font_panel.query(), "Inter");
    assert_eq!(state.font_panel.family, "Inter");
    assert_eq!(state.font_panel.offset, 84.0);
    assert_eq!(state.font_panel.preview.key, [4, 5, 6]);
}
