//! 【工具栏设置】【跨栏拖动】统一工具池、主栏和侧栏的拖动预览与一次性提交

use super::super::window::{Preferences, contains};
use super::{input, layout::Layout};
use crate::{
    config::toolbar_layout::{Profile, ToolbarLayout},
    ui::toolbar::ToolbarItem,
};
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Source {
    Pool(ToolbarItem),
    Bar(Profile, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Target {
    Bar(Profile, usize),
    Pool,
}

pub(in crate::features::preferences) struct Drag {
    pub(super) source: Source,
    pub(super) start: (f64, f64),
    pub(super) target: Option<Target>,
    pub active: bool,
}

impl Drag {
    /// 【工具栏设置】【拖动项目】从 config 读取 source 对应项目，返回按钮或分隔线
    pub fn item(&self, config: &ToolbarLayout) -> ToolbarItem {
        match self.source {
            Source::Pool(item) => item,
            Source::Bar(profile, index) => profile.items(config)[index],
        }
    }
}

/// 【工具栏设置】【栏内编辑】按 profile 返回 config 的可修改项目，图片场景沿用主栏
pub(super) fn items(config: &mut ToolbarLayout, profile: Profile) -> &mut Vec<ToolbarItem> {
    match profile {
        Profile::Actions => &mut config.actions,
        _ => &mut config.main,
    }
}

/// 【工具栏设置】【避让位置】按 state 与 layout 返回各原始项目的目标矩形，拖动项目由浮动预览绘制
pub(super) fn positions(state: &Preferences, layout: &Layout) -> Vec<(Profile, usize, Rect)> {
    let drag = state.toolbar.drag.as_ref().filter(|drag| drag.active);
    layout
        .bars
        .iter()
        .flat_map(|bar| {
            (0..bar.cells.len()).map(move |index| {
                let mut slot = index;
                if let Some(drag) = drag {
                    if let Source::Bar(profile, source) = drag.source
                        && profile == bar.profile
                        && index > source
                    {
                        slot -= 1;
                    }
                    if let Some(Target::Bar(profile, target)) = drag.target
                        && profile == bar.profile
                        && slot >= target
                    {
                        slot += 1;
                    }
                }
                (bar.profile, index, bar.slot(slot))
            })
        })
        .collect()
}

/// 【工具栏设置】【拖动更新】按 position 更新 state 的目标位置并启动避让动画，无返回值
pub(super) fn update(state: &mut Preferences, position: (f64, f64)) {
    let old = positions(state, &Layout::new(state));
    let layout = Layout::new(state);
    let Some(drag) = &mut state.toolbar.drag else {
        return;
    };
    drag.active |= (position.0 - drag.start.0).hypot(position.1 - drag.start.1) >= 4.0;
    drag.target = Profile::ALL
        .into_iter()
        .find_map(|profile| {
            layout.insertion(profile, position).map(|index| {
                let index = if let Source::Bar(source, original) = drag.source {
                    index.saturating_sub(usize::from(source == profile && original < index))
                } else {
                    index
                };
                Target::Bar(profile, index)
            })
        })
        .or_else(|| {
            (contains(layout.viewport, position) && contains(layout.pool, position))
                .then_some(Target::Pool)
        });
    state.pointer = position;
    let next = positions(state, &Layout::new(state));
    state.toolbar.animation.retarget(&old, next);
}

/// 【工具栏设置】【释放提交】完成 state 的当前拖动，返回是否实际拖动；范围外取消且不改写配置
pub(super) fn finish(state: &mut Preferences, position: (f64, f64)) -> bool {
    update(state, position);
    let Some(drag) = state.toolbar.drag.take() else {
        return false;
    };
    state.toolbar.animation = Default::default();
    if !drag.active {
        return false;
    }
    state.toolbar.pressed = None;
    let Some(target) = drag.target else {
        return true;
    };
    if matches!((drag.source, target), (Source::Pool(_), Target::Pool)) {
        return true;
    }
    let item = drag.item(&state.toolbar.config);
    let mut config = state.toolbar.config.clone();
    if let Source::Bar(profile, index) = drag.source {
        items(&mut config, profile).remove(index);
    }
    let previous_profile = state.toolbar.profile;
    let selected = match target {
        Target::Pool => None,
        Target::Bar(profile, index) => {
            let entries = items(&mut config, profile);
            if item != ToolbarItem::Seperator && entries.contains(&item) {
                return true;
            }
            let index = index.min(entries.len());
            entries.insert(index, item);
            state.toolbar.profile = profile;
            Some(index)
        }
    };
    if !input::save_layout(state, config, selected) {
        state.toolbar.profile = previous_profile;
    }
    true
}
