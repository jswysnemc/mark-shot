//! 【原生贴图】【菜单交互】菜单绘制、语言选择和按下释放事务分别绑定稳定菜单身份

use super::*;

pub(super) struct Menu {
    id: u64,
    anchor: (f32, f32),
    language: bool,
    hovered: Option<usize>,
    armed: Option<usize>,
    frame: Option<Pixmap>,
    changed: bool,
    mode: crate::theme::Mode,
    scale: f32,
}

impl Menu {
    /// 【原生贴图】【菜单命中】按position返回当前菜单有效行，语言菜单使用共享行高与边距
    fn hit(&self, position: (f64, f64)) -> Option<usize> {
        if !self.language {
            return menu::hit(position);
        }
        if position.0 < f64::from(menu::PADDING)
            || position.0 >= f64::from(menu::WIDTH - menu::PADDING)
            || position.1 < f64::from(menu::PADDING)
        {
            return None;
        }
        let index = ((position.1 - f64::from(menu::PADDING)) / f64::from(menu::ROW)) as usize;
        (index < crate::translation::language::OPTIONS.len()).then_some(index)
    }
}

impl Pin {
    /// 【原生贴图】【菜单打开】按language选择业务菜单或语言菜单，使用新的稳定身份，无返回值
    pub(super) fn open_menu(&mut self, language: bool) {
        self.next_menu = self.next_menu.wrapping_add(1);
        self.menu = Some(Menu {
            id: self.next_menu,
            anchor: self.menu_anchor,
            language,
            hovered: None,
            armed: None,
            frame: None,
            changed: true,
            mode: crate::theme::Mode::current(),
            scale: 0.0,
        });
    }

    /// 【原生贴图】【菜单身份】无参数，返回当前菜单编号，旧事件不能匹配重新打开菜单
    pub(super) fn menu_id(&self) -> Option<u64> {
        self.menu.as_ref().map(|menu| menu.id)
    }

    /// 【原生贴图】【行可用性】按language返回每行是否可执行，不借用菜单可变状态
    fn availability(&self, language: bool) -> Vec<bool> {
        if language {
            vec![true; crate::translation::language::OPTIONS.len()]
        } else {
            menu::ITEMS
                .iter()
                .map(|item| item.action.is_some_and(|action| self.enabled(action)))
                .collect()
        }
    }

    /// 【原生贴图】【菜单像素】按scale返回共享菜单像素，静止菜单不重复分配
    pub(super) fn menu_frame(&mut self, scale: f32) -> Option<PopupFrame<'_>> {
        let available = self.availability(self.menu.as_ref()?.language);
        let popup = self.menu.as_mut()?;
        if !popup.changed && popup.mode == crate::theme::Mode::current() && popup.scale == scale {
            return None;
        }
        popup.changed = false;
        popup.mode = crate::theme::Mode::current();
        popup.scale = scale;
        let size = (
            menu::WIDTH,
            if popup.language {
                menu::PADDING * 2 + menu::ROW * crate::translation::language::OPTIONS.len() as u32
            } else {
                menu::height()
            },
        );
        let pixel_size = (
            (size.0 as f32 * scale).ceil() as u32,
            (size.1 as f32 * scale).ceil() as u32,
        );
        let mut frame = Pixmap::new(pixel_size.0, pixel_size.1)?;
        let language = crate::ui::language::Language::current();
        let rows = if popup.language {
            crate::translation::language::labels(language)
                .iter()
                .enumerate()
                .map(|(index, label)| Row {
                    label: (*label).into(),
                    enabled: true,
                    checked: crate::translation::language::OPTIONS[index]
                        == self.translation.target,
                    danger: false,
                    submenu: false,
                })
                .collect::<Vec<_>>()
        } else {
            menu::ITEMS
                .iter()
                .enumerate()
                .map(|(index, item)| Row {
                    label: language.text(item.label).into(),
                    enabled: available[index],
                    checked: match item.action {
                        Some(PinAction::AlwaysOnTop) => {
                            self.top_supported && self.image.always_on_top
                        }
                        Some(PinAction::TextSelection) => self.text_selection,
                        Some(PinAction::ToggleTranslation) => self.translation.active,
                        _ => false,
                    },
                    danger: item.action == Some(PinAction::Close),
                    submenu: item.action == Some(PinAction::TargetLanguage),
                })
                .collect::<Vec<_>>()
        };
        menu_draw::draw(
            &mut frame,
            &rows,
            popup.hovered,
            size,
            scale,
            &mut self.fonts,
            &mut self.cache,
            &self.icons,
        );
        popup.frame = Some(frame);
        Some(PopupFrame {
            id: popup.id,
            monitor_idx: 0,
            serial: None,
            anchor: popup.anchor,
            size,
            scale,
            pixels: popup.frame.as_ref()?.data(),
            pixel_size,
        })
    }

    /// 【原生贴图】【菜单输入】按id与event处理完整菜单事务，释放前不修改业务状态
    pub(super) fn menu_event(&mut self, id: u64, event: PopupEvent) -> WindowPopupAction {
        let Some(language) = self
            .menu
            .as_ref()
            .filter(|menu| menu.id == id)
            .map(|menu| menu.language)
        else {
            return WindowPopupAction::None;
        };
        let available = self.availability(language);
        let popup = self.menu.as_mut().unwrap();
        popup.changed = true;
        match event {
            PopupEvent::Pointer { x, y } => popup.hovered = popup.hit((x, y)),
            PopupEvent::Leave => popup.hovered = None,
            PopupEvent::Button {
                x,
                y,
                pressed,
                button: crate::types::MouseButton::Left,
            } => {
                let hit = popup.hit((x, y)).filter(|index| available[*index]);
                if pressed {
                    popup.armed = hit;
                } else if hit.is_some() && popup.armed.take() == hit {
                    return WindowPopupAction::Activate(hit.unwrap());
                }
            }
            PopupEvent::Closed => return WindowPopupAction::Close,
            PopupEvent::Key {
                chord: Some(chord), ..
            } => {
                use crate::keys::Key;
                popup.armed = None;
                let choices = available
                    .iter()
                    .enumerate()
                    .filter_map(|(index, enabled)| enabled.then_some(index))
                    .collect::<Vec<_>>();
                match chord.key {
                    Key::Escape | Key::Left if language => return WindowPopupAction::Close,
                    Key::Escape => return WindowPopupAction::Close,
                    Key::Enter | Key::Space | Key::Right => {
                        if let Some(index) = popup.hovered.filter(|index| available[*index]) {
                            return WindowPopupAction::Activate(index);
                        }
                    }
                    Key::Home => popup.hovered = choices.first().copied(),
                    Key::End => popup.hovered = choices.last().copied(),
                    Key::Up | Key::Down if !choices.is_empty() => {
                        let current = popup
                            .hovered
                            .and_then(|hovered| choices.iter().position(|index| *index == hovered));
                        let index = if chord.key == Key::Down {
                            current.map_or(0, |index| (index + 1) % choices.len())
                        } else {
                            current.map_or(choices.len() - 1, |index| {
                                (index + choices.len() - 1) % choices.len()
                            })
                        };
                        popup.hovered = Some(choices[index]);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        WindowPopupAction::Redraw
    }

    /// 【原生贴图】【菜单交付】按id和index关闭原生菜单后提交动作，返回窗口动作，失败使用统一通知
    pub(super) fn menu_closed(&mut self, id: u64, index: Option<usize>) -> WindowAction {
        if self.menu_id() != Some(id) {
            return WindowAction::None;
        }
        let popup = self.menu.take().unwrap();
        let Some(index) = index else {
            return WindowAction::Redraw;
        };
        let result = if popup.language {
            self.choose_language(index).map(|_| WindowAction::Redraw)
        } else if let Some(action) = menu::ITEMS.get(index).and_then(|item| item.action) {
            self.action(action)
        } else {
            Ok(WindowAction::None)
        };
        match result {
            Ok(action) => action,
            Err(error) => {
                self.report(Err(error));
                WindowAction::Redraw
            }
        }
    }
}
