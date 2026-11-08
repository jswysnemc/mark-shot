//! 【设置窗口】【事件状态】管理分类、搜索、选项菜单和输入，所有变更逐项保存

use super::catalog::{ENTRIES, Kind, Page};
use crate::{
    backend::window::{self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec},
    keys::Key,
    ui::{fonts::UiFonts, single_line::SingleLine},
};
use serde_json::Value;
use tiny_skia::Rect;

pub(super) struct Preferences {
    pub path: std::path::PathBuf,
    pub values: Value,
    pub page: Page,
    pub size: (u32, u32),
    pub scroll: usize,
    pub scrollbar_drag: Option<crate::ui::scrollbar::Drag>,
    pub search: SingleLine,
    pub search_focus: bool,
    pub editing: Option<(usize, SingleLine)>,
    pub choices: Option<usize>,
    pub choice_cursor: usize,
    pub focus: Option<usize>,
    pub pointer: (f64, f64),
    pub pressed: Option<Control>,
    pub fonts: UiFonts,
    pub cache: cosmic_text::SwashCache,
    pub status: String,
    pub toolbar: super::toolbar::Editor,
    ready: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Control {
    Close,
    Page(Page),
    Row(usize),
    Choice(usize),
    Search,
    Apply,
    ConfigPath,
    Scrollbar,
}

impl Preferences {
    /// 【设置窗口】【初始化】按 ready 构造轻量状态，读取当前实际配置路径，返回解析结果
    pub fn new(ready: bool) -> crate::AppResult<Self> {
        let path = crate::config::preferences::path()?;
        let values = crate::config::preferences::read(&path)?;
        let mut fonts = UiFonts::with_system_family("Sans Serif");
        let search = SingleLine::new("", &mut fonts, 128);
        let toolbar = super::toolbar::Editor::new(&values);
        Ok(Self {
            path,
            values,
            page: Page::Appearance,
            size: (900, 640),
            scroll: 0,
            scrollbar_drag: None,
            search,
            search_focus: false,
            editing: None,
            choices: None,
            choice_cursor: 0,
            focus: None,
            pointer: (-1.0, -1.0),
            pressed: None,
            fonts,
            cache: cosmic_text::SwashCache::new(),
            status: String::new(),
            toolbar,
            ready,
        })
    }

    /// 【设置窗口】【界面语言】无参数，返回当前语言选择，环境覆盖沿用应用启动规则
    pub fn chinese(&self) -> bool {
        let mode = crate::ui::language::Mode::parse(
            self.values["ui"]["language"].as_str().unwrap_or("system"),
        );
        crate::ui::language::Language::resolve(
            std::env::var("MARK_SHOT_LANG").ok().as_deref(),
            mode,
            sys_locale::get_locale().as_deref(),
        ) == crate::ui::language::Language::Chinese
    }

    /// 【工具栏设置】【页面判断】无参数，返回当前是否显示工具栏内容，搜索结果沿用通用设置列表
    pub fn toolbar_page(&self) -> bool {
        self.page == Page::Toolbar && self.search.text().is_empty()
    }

    /// 【设置窗口】【显示取值】按 index 返回当前设置文字，未覆盖快捷键显示默认组合，空选项显示首个可选值
    pub fn value(&self, index: usize) -> String {
        let entry = &ENTRIES[index];
        let value = &self.values[entry.section][entry.key];
        if entry.section == "startup" && entry.key == "selection_loupe" && value.is_null() {
            return (crate::editor::saved::get().magnifier != Some(false)).to_string();
        }
        if entry.section == "ui" && entry.key == "language" && value.as_str() == Some("") {
            return "system".into();
        }
        match value {
            Value::String(text) => text.clone(),
            Value::Bool(value) => value.to_string(),
            Value::Array(values) => values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" / "),
            _ if matches!(entry.kind, Kind::Shortcut) => crate::keys::ACTIONS
                .iter()
                .find(|(key, _, _)| *key == entry.key)
                .map_or(String::new(), |(_, _, keys)| keys.join(" / ")),
            _ => match entry.kind {
                Kind::Toggle => "false".into(),
                Kind::Choice(choices) => choices[0].into(),
                _ => String::new(),
            },
        }
    }

    /// 【设置窗口】【可见目录】无参数，返回当前分类或搜索匹配的设置编号，搜索同时支持中文、英文与配置键
    pub fn visible(&self) -> Vec<usize> {
        let query = self.search.text().to_lowercase();
        ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if query.is_empty() {
                    entry.page == self.page
                } else {
                    format!("{} {} {}", entry.labels.0, entry.labels.1, entry.key)
                        .to_lowercase()
                        .contains(&query)
                }
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// 【设置窗口】【容纳行数】无参数，返回当前窗口可完整显示的设置行数
    pub fn capacity(&self) -> usize {
        ((self.size.1.saturating_sub(190)) / 68).max(1) as usize
    }

    /// 【设置窗口】【控件命中】按 point 返回可点击控件，菜单打开时优先命中菜单项
    pub fn hit(&self, point: (f64, f64)) -> Option<Control> {
        if let Some(index) = self.choices
            && let Kind::Choice(choices) = ENTRIES[index].kind
        {
            for choice in 0..choices.len() {
                if contains(super::draw::choice_rect(self, choice, choices.len()), point) {
                    return Some(Control::Choice(choice));
                }
            }
            return None;
        }
        if contains(super::draw::close_rect(self), point) {
            return Some(Control::Close);
        }
        if contains(super::draw::search_rect(self), point) {
            return Some(Control::Search);
        }
        if self
            .scrollbar()
            .is_some_and(|bar| contains(bar.track, point))
        {
            return Some(Control::Scrollbar);
        }
        for (index, page) in Page::ALL.into_iter().enumerate() {
            if contains(super::draw::nav_rect(self, index), point) {
                return Some(Control::Page(page));
            }
        }
        for (row, index) in self
            .visible()
            .into_iter()
            .skip(self.scroll)
            .take(self.capacity())
            .enumerate()
        {
            if self
                .editing
                .as_ref()
                .is_some_and(|(editing, _)| *editing == index)
                && contains(super::draw::apply_rect(self, row), point)
            {
                return Some(Control::Apply);
            }
            if contains(super::draw::value_rect(self, row), point) {
                return Some(Control::Row(index));
            }
        }
        if self.page == Page::Advanced && contains(super::draw::path_rect(self), point) {
            return Some(Control::ConfigPath);
        }
        None
    }

    /// 【设置窗口】【字段提交】按 index 保存 value，成功后刷新有效配置，失败在当前窗口显示状态并保留编辑内容
    pub fn save(&mut self, index: usize, value: Value) -> bool {
        let entry = &ENTRIES[index];
        self.save_field(entry.section, entry.key, value)
    }

    /// 【设置窗口】【共用保存】将 section/key 的 value 校验并保存，返回成功状态；自定义工具栏与列表共用此事务
    pub fn save_field(&mut self, section: &str, key: &str, value: Value) -> bool {
        if section == "keys"
            && let Some(key) = super::shortcuts::conflict(&self.values, key, &value)
        {
            let label = ENTRIES
                .iter()
                .find(|entry| entry.section == "keys" && entry.key == key)
                .map_or(key, |entry| entry.label(self.chinese()));
            self.status = format!(
                "{}: {label}",
                if self.chinese() {
                    "快捷键已用于"
                } else {
                    "Shortcut already used by"
                }
            );
            return false;
        }
        let result = crate::config::preferences::write(&self.path, section, key, value)
            .and_then(|()| crate::config::preferences::reload(&self.path))
            .and_then(|()| crate::config::preferences::read(&self.path));
        match result {
            Ok(values) => {
                self.values = values;
                crate::theme::apply_saved(crate::theme::Mode::parse(
                    self.values["ui"]["theme"].as_str().unwrap_or("light"),
                ));
                self.status = if self.chinese() {
                    "已保存；捕获与引擎启动选项在下次截图生效"
                } else {
                    "Saved. Capture and engine startup options apply next capture."
                }
                .into();
                true
            }
            Err(error) => {
                self.status = format!(
                    "{}: {error}",
                    if self.chinese() {
                        "保存失败"
                    } else {
                        "Save failed"
                    }
                );
                false
            }
        }
    }

    /// 【设置窗口】【输入确认】无参数，保存当前文本字段，成功才结束编辑，错误保留输入供修正
    fn apply(&mut self) {
        if let Some((index, field)) = &self.editing {
            let (index, text) = (*index, field.text());
            if self.save(index, text.into()) {
                self.editing = None;
            }
        }
    }

    /// 【设置窗口】【控件激活】按 control 切换分类、修改开关或进入文字编辑，无返回值
    fn activate(&mut self, control: Control) {
        match control {
            Control::Close | Control::Scrollbar => {}
            Control::Page(page) => {
                self.toolbar.cancel();
                self.toolbar.config = serde_json::from_value(self.values["toolbar_layout"].clone())
                    .unwrap_or_default();
                self.page = page;
                self.scroll = 0;
                self.focus = None;
                self.editing = None;
                self.search_focus = false;
                self.search.set("", &mut self.fonts, false);
            }
            Control::Search => {
                self.search_focus = true;
                self.editing = None;
            }
            Control::Apply => self.apply(),
            Control::Row(index) => {
                self.focus = Some(index);
                self.search_focus = false;
                match ENTRIES[index].kind {
                    Kind::Toggle => {
                        self.save(index, (self.value(index) != "true").into());
                    }
                    Kind::Choice(choices) => {
                        self.editing = None;
                        self.choice_cursor = choices
                            .iter()
                            .position(|value| *value == self.value(index))
                            .unwrap_or(0);
                        self.choices = Some(index);
                    }
                    Kind::Text | Kind::Shortcut => {
                        let text = self.value(index);
                        let mut field = SingleLine::new(&text, &mut self.fonts, 2048);
                        field.select_all(&mut self.fonts);
                        self.editing = Some((index, field));
                    }
                }
            }
            Control::Choice(choice) => {
                if let Some(index) = self.choices.take()
                    && let Kind::Choice(choices) = ENTRIES[index].kind
                {
                    self.save(index, choices[choice].into());
                }
            }
            Control::ConfigPath => {
                self.status = match crate::utils::copy_to_clipboard(&self.path.to_string_lossy()) {
                    Ok(()) => if self.chinese() {
                        "配置路径已复制"
                    } else {
                        "Configuration path copied"
                    }
                    .into(),
                    Err(error) => error.to_string(),
                };
            }
        }
    }
}

/// 【设置窗口】【矩形命中】按 rect 和 point 判断控件内部，返回布尔值
pub(super) fn contains(rect: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(rect.left())
        && point.0 < f64::from(rect.right())
        && point.1 >= f64::from(rect.top())
        && point.1 < f64::from(rect.bottom())
}

impl WindowContent for Preferences {
    /// 【设置窗口】【首帧确认】无参数，按启动约定通知父进程窗口已经映射，无返回值
    fn mapped(&mut self) {
        if self.ready {
            use std::io::Write;
            let _ = std::io::stdout().write_all(b"\n");
            self.ready = false;
        }
    }
    /// 【设置窗口】【画面输出】以 scale 将当前状态绘制到 canvas，静止时不发起周期绘制
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }
    /// 【设置窗口】【输入路由】按 event 更新当前控件和配置，返回必要的重绘或关闭动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if self.toolbar_page()
            && let Some(action) = super::toolbar::input::event(self, &event)
        {
            return action;
        }
        match event {
            WindowEvent::Resize { size } => {
                self.size = size;
                self.toolbar.scroll = self
                    .toolbar
                    .scroll
                    .min(super::toolbar::layout::Layout::new(self).maximum());
                self.scroll = self
                    .scroll
                    .min(self.visible().len().saturating_sub(self.capacity()));
            }
            WindowEvent::PointerMove { position } => {
                if let Some(drag) = self.scrollbar_drag
                    && let Some(bar) = self.scrollbar()
                {
                    self.scroll = drag.value(position, &bar);
                    self.pointer = position;
                    return WindowAction::Redraw;
                }
                let before = self.hit(self.pointer);
                self.pointer = position;
                if before == self.hit(position) {
                    return WindowAction::None;
                }
            }
            WindowEvent::PointerButton {
                position,
                button: window::LEFT_BUTTON,
                pressed,
            } => {
                let hit = self.hit(position);
                if !pressed && self.scrollbar_drag.take().is_some() {
                    self.pressed = None;
                    return WindowAction::Redraw;
                }
                if pressed {
                    if self.choices.is_none() && self.press_scrollbar(position) {
                        return WindowAction::Redraw;
                    }
                    if self.choices.is_none()
                        && position.0 >= self.size.0 as f64 - 18.0
                        && position.1 >= self.size.1 as f64 - 18.0
                    {
                        return WindowAction::Resize;
                    }
                    if hit.is_none() && self.choices.is_none() && position.1 < 78.0 {
                        return WindowAction::Move;
                    }
                    self.pressed = hit;
                } else if let Some(control) =
                    self.pressed.take().filter(|control| Some(*control) == hit)
                {
                    if control == Control::Close {
                        return WindowAction::Close;
                    }
                    self.activate(control);
                } else {
                    self.choices = None;
                }
            }
            WindowEvent::Scroll { delta, .. } => {
                self.scroll_rows(if delta.1 > 0.0 {
                    1
                } else if delta.1 < 0.0 {
                    -1
                } else {
                    0
                });
            }
            WindowEvent::Key(input) => {
                if let Some(index) = self.choices
                    && let Kind::Choice(choices) = ENTRIES[index].kind
                {
                    match input.key {
                        Some(Key::Escape) => self.choices = None,
                        Some(Key::Up) => self.choice_cursor = self.choice_cursor.saturating_sub(1),
                        Some(Key::Down) => {
                            self.choice_cursor = (self.choice_cursor + 1).min(choices.len() - 1)
                        }
                        Some(Key::Home) => self.choice_cursor = 0,
                        Some(Key::End) => self.choice_cursor = choices.len() - 1,
                        Some(Key::Enter | Key::Space) => {
                            self.activate(Control::Choice(self.choice_cursor))
                        }
                        _ => {}
                    }
                    return WindowAction::Redraw;
                }
                if input.key == Some(Key::Escape) {
                    if self.choices.take().is_none()
                        && self.editing.take().is_none()
                        && !self.search_focus
                    {
                        return WindowAction::Close;
                    }
                    self.search_focus = false;
                } else if input.mods.ctrl && input.key == Some(Key::Char('f')) {
                    self.search_focus = true;
                    self.editing = None;
                    self.search.select_all(&mut self.fonts);
                } else if self.search_focus {
                    self.search.key(&mut self.fonts, &input);
                    self.scroll = 0;
                } else if input.key == Some(Key::Enter) && self.editing.is_some() {
                    self.apply();
                } else if let Some((index, field)) = &mut self.editing {
                    if matches!(ENTRIES[*index].kind, Kind::Shortcut) {
                        if let Some(chord) = super::shortcuts::chord(&input) {
                            let index = *index;
                            if crate::keys::parse(&chord).is_ok() && self.save(index, chord.into())
                            {
                                self.editing = None;
                            }
                        }
                    } else {
                        field.key(&mut self.fonts, &input);
                    }
                } else if self.scroll_key(input.key) {
                    self.focus = None;
                } else if input.key == Some(Key::Tab) {
                    let visible = self.visible();
                    if !visible.is_empty() {
                        let next = self
                            .focus
                            .and_then(|focus| visible.iter().position(|index| *index == focus))
                            .map_or(0, |index| (index + 1) % visible.len());
                        self.focus = Some(visible[next]);
                        self.scroll = (next / self.capacity()) * self.capacity();
                    }
                } else if input.key == Some(Key::Enter)
                    && let Some(index) = self.focus
                {
                    self.activate(Control::Row(index));
                }
            }
            WindowEvent::Focused(false) => {
                self.pressed = None;
                self.scrollbar_drag = None;
            }
            _ => return WindowAction::None,
        }
        WindowAction::Redraw
    }
    /// 【设置窗口】【指针反馈】按 position 返回文本或按钮光标，无状态修改
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if self.toolbar_page() {
            if self.toolbar.drag.as_ref().is_some_and(|drag| drag.active) {
                return WindowCursor::Grabbing;
            }
            if let Some(hit) = super::toolbar::layout::Layout::new(self).hit(position) {
                return if matches!(hit, super::toolbar::layout::Hit::Item(_)) {
                    WindowCursor::Grab
                } else {
                    WindowCursor::Pointer
                };
            }
        }
        if position.0 >= self.size.0 as f64 - 18.0 && position.1 >= self.size.1 as f64 - 18.0 {
            return WindowCursor::Resize;
        }
        match self.hit(position) {
            Some(Control::Search) => WindowCursor::Text,
            Some(_) => WindowCursor::Pointer,
            None => WindowCursor::Default,
        }
    }
}

/// 【设置窗口】【窗口执行】按 ready 创建普通原生窗口，返回生命周期结果
pub(super) fn run(ready: bool) -> crate::AppResult {
    window::run(
        WindowSpec {
            title: "Mark Shot — Settings".into(),
            app_id: "mark-shot-settings".into(),
            size: (900, 640),
            minimum: (580, 420),
        },
        Preferences::new(ready)?,
    )
}
