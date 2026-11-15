//! 【截图控件识别】【键盘切换】Tab只在Windows截图选择阶段切换整窗与组件，不进入文字或拖动事务

use crate::{editor::EditorState, keys::Chord};

/// 【截图控件识别】【Tab分派】按state与chord切换原生目标，dirty返回候选损坏范围，返回是否消费按键
pub(crate) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    #[cfg(any(target_os = "windows", test))]
    {
        use crate::{
            editor::dirty::apply_damage_rects,
            keys::{Key, Mods},
        };
        if chord
            != Some(Chord {
                key: Key::Tab,
                mods: Mods::default(),
            })
            || !super::input::selecting(state)
            || state.input.mouse_down
            || state.region_detection.pressed.is_some()
            || state.settings_panel.visible
        {
            return false;
        }
        let Some(native) = &mut state.region_detection.native else {
            return false;
        };
        native.hover(state.input.pointer.global);
        let rect = native.toggle();
        super::draw::apply(state, rect);
        super::input::refresh(state, dirty);
        apply_damage_rects(state, dirty);
        true
    }
    #[cfg(not(any(target_os = "windows", test)))]
    {
        let _ = (state, chord, dirty);
        false
    }
}
