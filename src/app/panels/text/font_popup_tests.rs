//! 【字体弹层测试】【状态重绘】鼠标释放后立即刷新交互底板，不等待主题切换触发绘制

use super::*;
use crate::{
    fonts::font_catalog::{Family, FontCatalog},
    tools::Tool,
};

/// 【字体弹层测试】【释放刷新】无参数，滚动轨道按下与释放均登记弹层重绘，释放后不残留按下颜色，无返回值
#[test]
fn scrollbar_release_invalidates_the_popup_even_without_value_changes() {
    let mut state = crate::app::input::tests::editor();
    crate::app::input::select_tool(&mut state, Tool::Text, &mut 0);
    state.font_panel.catalog = Some(FontCatalog {
        families: (0..20)
            .map(|index| Family {
                name: format!("Sample {index}"),
                faces: Vec::new(),
            })
            .collect(),
    });
    toggle(&mut state, &mut 0);
    assert!(state.font_panel.open);
    let thumb = state.font_panel.thumb().unwrap();
    let position = state.font_panel.render_pos;
    state.input.pointer.local = (
        (position.0 + thumb.left() + 7.0) as f64,
        (position.1 + thumb.top() + 10.0) as f64,
    );
    state.input.pointer.monitor_idx = state.font_panel.monitor_idx;
    press(&mut state, &mut 0);
    assert_eq!(state.font_panel.pressed, Some(Hit::Scrollbar));
    state.font_panel.dirty = false;
    state.damage_rects.clear();
    let mut dirty = 0;
    assert!(release(&mut state, &mut dirty));
    assert!(state.font_panel.pressed.is_none());
    assert!(state.font_panel.scrollbar_drag.is_none());
    assert!(
        state.font_panel.dirty,
        "released scrollbar must discard its pressed pixels"
    );
    assert_ne!(dirty, 0);
}
