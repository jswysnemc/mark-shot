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
    pub source: Value,
    pub page: Page,
    pub size: (u32, u32),
    pub scroll: usize,
    pub scrollbar_drag: Option<crate::ui::scrollbar::Drag>,
    pub search: SingleLine,
    pub search_focus: bool,
    pub editing: Option<(usize, SingleLine)>,
    pub dropdown: Option<super::dropdown::Dropdown>,
    pub focus: Option<usize>,
    pub pointer: (f64, f64),
    pub pressed: Option<Control>,
    pub fonts: UiFonts,
    pub icons: crate::renderer::Icons,
    pub cache: cosmic_text::SwashCache,
    pub status: String,
    pub toolbar: super::toolbar::Editor,
    pub translation: super::translation::State,
    ready: bool,
    pub recovery: Option<String>,
    pub page_scroll: [usize; 13],
    pub dialog: Option<window::WindowDialog>,
    pub frame_preview: super::frame_preview::Preview,
    theme_watch: Option<crate::theme::SystemWatch>,
    pub resource_note: String,
    #[cfg(test)]
    test_directory: Option<tempfile::TempDir>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Control {
    Close,
    Page(Page),
    Row(usize),
    Segment(usize, usize),
    Search,
    Apply,
    ConfigPath,
    Scrollbar,
    Reset(usize),
    Clear(usize),
    Repair,
    Step(usize, i8),
}

impl Preferences {
    /// 【设置窗口】【初始化】按 ready 构造轻量状态，读取当前实际配置路径，返回解析结果
    pub fn new(ready: bool) -> crate::AppResult<Self> {
        #[cfg(test)]
        {
            let directory = tempfile::tempdir()?;
            let mut state = Self::from_path(directory.path().join("config.toml"), ready)?;
            state.test_directory = Some(directory);
            Ok(state)
        }
        #[cfg(not(test))]
        {
            let path = crate::config::preferences::path()?;
            Self::from_path(path, ready)
        }
    }

    /// 【设置窗口】【指定文件】按 path、ready 创建独立状态，配置错误保留修复入口，返回初始化结果
    pub fn from_path(path: std::path::PathBuf, ready: bool) -> crate::AppResult<Self> {
        let (values, recovery) = match crate::config::preferences::read(&path) {
            Ok(values) => (values, None),
            Err(error) => (
                serde_json::to_value(crate::config::Config::default())?,
                Some(error.to_string()),
            ),
        };
        let mut fonts = UiFonts::with_system_family("Sans Serif");
        let search = SingleLine::new("", &mut fonts, 128);
        let toolbar = super::toolbar::Editor::new(&values);
        let translation = super::translation::State::new(&values);
        Ok(Self {
            source: crate::config::settings_store::source(&path).unwrap_or_default(),
            path,
            values,
            page: Page::Appearance,
            size: (900, 640),
            scroll: 0,
            scrollbar_drag: None,
            search,
            search_focus: false,
            editing: None,
            dropdown: None,
            focus: None,
            pointer: (-1.0, -1.0),
            pressed: None,
            fonts,
            icons: super::shell::icons(),
            cache: cosmic_text::SwashCache::new(),
            status: String::new(),
            toolbar,
            translation,
            ready,
            recovery,
            page_scroll: [0; 13],
            dialog: None,
            frame_preview: Default::default(),
            theme_watch: None,
            resource_note: String::new(),
            #[cfg(test)]
            test_directory: None,
        })
    }

    /// 【设置窗口】【界面语言】无参数，返回当前语言选择，环境覆盖沿用应用启动规则
    pub fn chinese(&self) -> bool {
        let mode = crate::ui::language::Mode::parse(
            self.values["ui"]["language"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| self.values["language"].as_str())
                .unwrap_or("system"),
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

    /// 【翻译设置】【页面判断】无参数，返回当前是否显示独立翻译配置页面
    pub fn translation_page(&self) -> bool {
        self.page == Page::Translation && self.search.text().is_empty()
    }

    /// 【设置窗口】【显示取值】按 index 返回当前设置文字，未覆盖快捷键显示默认组合，空选项显示首个可选值
    pub fn value(&self, index: usize) -> String {
        ENTRIES[index].display(&self.values)
    }

    /// 【设置窗口】【可见目录】无参数，返回当前分类或搜索匹配的设置编号，搜索同时支持中文、英文与配置键
    pub fn visible(&self) -> Vec<usize> {
        let query = self.search.text().to_lowercase();
        let mut visible: Vec<_> = ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if query.is_empty() {
                    entry.page == self.page
                        && !(entry.section == "ocr"
                            && ((entry.key == "engine" && !cfg!(target_os = "windows"))
                                || (self.values["ocr"]["engine"] == "system"
                                    && matches!(entry.key, "mode" | "device"))))
                } else {
                    entry.search_text().to_lowercase().contains(&query)
                }
            })
            .map(|(index, _)| index)
            .collect();
        if query.is_empty() {
            visible.sort_by_key(|index| super::sections::group(&ENTRIES[*index]).0);
        }
        visible
    }

    /// 【设置窗口】【控件命中】按 point 返回可点击控件，菜单打开时优先命中菜单项
    pub fn hit(&self, point: (f64, f64)) -> Option<Control> {
        if contains(super::layout::close_rect(self), point) {
            return Some(Control::Close);
        }
        if contains(super::layout::search_rect(self), point) {
            return Some(Control::Search);
        }
        if self
            .scrollbar()
            .is_some_and(|bar| contains(bar.track, point))
        {
            return Some(Control::Scrollbar);
        }
        for (index, page) in Page::ALL.into_iter().enumerate() {
            if contains(super::layout::nav_rect(self, index), point) {
                return Some(Control::Page(page));
            }
        }
        if self.recovery.is_some() && contains(super::recovery::button(self), point) {
            return Some(Control::Repair);
        }
        if contains(super::layout::path_rect(self), point) {
            return Some(Control::ConfigPath);
        }
        if self.recovery.is_some() {
            return None;
        }
        super::layout::Layout::new(self).hit(self, point)
    }

    /// 【设置窗口】【字段提交】按 index 保存 value，成功后刷新有效配置，失败在当前窗口显示状态并保留编辑内容
    pub fn save(&mut self, index: usize, value: Value) -> bool {
        let entry = &ENTRIES[index];
        self.save_field(entry.section, entry.key, value)
    }

    /// 【设置窗口】【共用保存】将 section/key 的 value 校验并保存，返回成功状态；自定义工具栏与列表共用此事务
    pub fn save_field(&mut self, section: &str, key: &str, value: Value) -> bool {
        let proposed = if section == "keys" && value.is_null() {
            super::shortcuts::defaults(key).into()
        } else {
            value.clone()
        };
        if section == "keys"
            && let Some(key) = super::shortcuts::conflict(&self.values, key, &proposed)
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
        if let Err(error) = crate::config::preferences::write(&self.path, section, key, value) {
            self.status = format!(
                "{}: {error}",
                if self.chinese() {
                    "保存失败"
                } else {
                    "Save failed"
                }
            );
            return false;
        }
        let applied = self.publish();
        let result = crate::config::preferences::read(&self.path);
        match result {
            Ok(values) => {
                self.values = values;
                self.source = crate::config::settings_store::source(&self.path).unwrap_or_default();
                if !cfg!(test) {
                    crate::theme::apply_saved(crate::theme::Mode::parse(
                        self.values["ui"]["theme"].as_str().unwrap_or("light"),
                    ));
                }
                self.status = ENTRIES
                    .iter()
                    .find(|entry| entry.section == section && entry.key == key)
                    .map_or(if self.chinese() { "已保存" } else { "Saved" }, |entry| {
                        entry.effect(self.chinese())
                    })
                    .into();
                if let Err(error) = applied {
                    self.status = format!(
                        "{}: {error}",
                        if self.chinese() {
                            "已保存，应用失败；下次启动将重新读取"
                        } else {
                            "Saved; apply failed, will reload next launch"
                        }
                    );
                }
                true
            }
            Err(error) => {
                self.status = format!(
                    "{}: {error}",
                    if self.chinese() {
                        "已保存，重新读取失败"
                    } else {
                        "Saved; reload failed"
                    }
                );
                true
            }
        }
    }

    /// 【设置窗口】【文本定位】按 point 和 drag 更新已激活搜索框或文本选区，返回是否消费输入
    fn edit_pointer(&mut self, point: (f64, f64), drag: bool) -> bool {
        let search = super::layout::search_rect(self);
        if self.search_focus
            && (drag && self.pressed == Some(Control::Search) || contains(search, point))
        {
            self.search.pointer(
                &mut self.fonts,
                (
                    point.0 - f64::from(search.left() + 31.0),
                    point.1 - f64::from(search.top() + 3.0),
                ),
                drag,
            );
            return true;
        }
        let layout = super::layout::Layout::new(self);
        let Some((index, editor)) = &mut self.editing else {
            return false;
        };
        if !matches!(ENTRIES[*index].kind, Kind::Text) {
            return false;
        }
        let Some(row) = layout.rows.iter().find(|row| row.index == *index) else {
            return false;
        };
        let area = super::layout::editor_rect(row);
        if !(drag && self.pressed == Some(Control::Row(*index)))
            && (!contains(area, point) || !contains(layout.viewport, point))
        {
            return false;
        }
        editor.pointer(
            &mut self.fonts,
            (
                point.0 - f64::from(area.left()),
                point.1 - f64::from(area.top()),
            ),
            drag,
        );
        true
    }

    /// 【设置窗口】【控件激活】按 control 切换分类、修改开关或进入文字编辑，无返回值
    pub(super) fn activate(&mut self, control: Control) {
        if !matches!(
            control,
            Control::Apply | Control::Reset(_) | Control::Clear(_)
        ) && !self
            .editing
            .as_ref()
            .is_some_and(|(index, _)| control == Control::Row(*index))
            && !self.commit_pending()
        {
            return;
        }
        match control {
            Control::Close | Control::Scrollbar => {}
            Control::Page(page) => {
                self.translation.cancel();
                self.toolbar.cancel();
                self.toolbar.config = serde_json::from_value(self.values["toolbar_layout"].clone())
                    .unwrap_or_default();
                self.page_scroll[self.page as usize] = self.scroll;
                self.page = page;
                if page == Page::Ocr {
                    self.refresh_resources();
                }
                self.scroll = self.page_scroll[page as usize];
                self.focus = None;
                self.editing = None;
                self.search_focus = false;
                self.search.set("", &mut self.fonts, false);
            }
            Control::Search => {
                self.search_focus = true;
                self.editing = None;
            }
            Control::Apply => {
                self.apply();
            }
            Control::Reset(index) => {
                if self.save(index, Value::Null) {
                    self.editing = None;
                }
            }
            Control::Clear(index) => {
                if self.save(index, serde_json::json!([])) {
                    self.editing = None;
                }
            }
            Control::Repair => self.repair(),
            Control::Step(index, direction) => self.step(index, direction),
            Control::Segment(index, choice) => {
                self.editing = None;
                self.search_focus = false;
                if let Kind::Segments(choices) = ENTRIES[index].kind {
                    self.save(index, choices[choice].into());
                }
            }
            Control::Row(index) => {
                self.focus = Some(index);
                self.search_focus = false;
                self.reveal_row(index);
                match ENTRIES[index].kind {
                    Kind::Link(page) => {
                        if ENTRIES[index].key == "resources" {
                            self.dialog = Some(crate::features::ocr_model_guide::dialog());
                        } else {
                            self.activate(Control::Page(page));
                            if page == Page::Translation
                                && let Some(selected) =
                                    crate::config::settings::translation::SERVICES
                                        .iter()
                                        .position(|service| service.vendor == ENTRIES[index].key)
                            {
                                let _ = self.translation.select(&self.path, selected);
                            }
                        }
                    }
                    Kind::Toggle => {
                        self.save(index, (self.value(index) != "true").into());
                    }
                    Kind::Choice(_) | Kind::Segments(_) => {
                        self.editing = None;
                        super::menus::setting(self, index);
                    }
                    Kind::Text | Kind::Shortcut => {
                        if self
                            .editing
                            .as_ref()
                            .is_some_and(|(active, _)| *active == index)
                        {
                            return;
                        }
                        let text = self.value(index);
                        let mut field = SingleLine::new(&text, &mut self.fonts, 2048);
                        field.select_all(&mut self.fonts);
                        self.editing = Some((index, field));
                    }
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
    /// 【设置窗口】【系统关闭】无参数，关闭前提交输入，失败保留窗口和草稿，返回是否允许关闭
    fn close_requested(&mut self) -> bool {
        self.commit_pending()
    }
    /// 【设置窗口】【资源对话框】无参数，取出待打开的 OCR 资源管理窗口，返回可选对话框
    fn take_dialog(&mut self) -> Option<window::WindowDialog> {
        self.dialog.take()
    }
    /// 【设置窗口】【资源返回】按对话框关闭事件刷新安装摘要，返回重绘动作，不启动下载
    fn dialog_closed(&mut self, _id: u64, _accepted: bool) -> WindowAction {
        self.refresh_resources();
        WindowAction::Redraw
    }
    /// 【设置窗口】【动画调度】无参数，返回工具栏避让动画下一帧期限，静止时没有周期工作
    fn next_deadline(&self) -> Option<std::time::Instant> {
        self.toolbar.animation.deadline()
    }
    /// 【翻译设置】【完成通知】保存 wake 用于后台目录查询完成后唤醒窗口，无返回值
    fn connected(&mut self, wake: window::WindowWake) {
        self.theme_watch = Some(crate::theme::watch_system(wake.clone()));
        self.translation.wake = Some(wake);
    }
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
        super::dropdown::draw::draw(self, canvas, scale);
    }
    /// 【设置窗口】【输入路由】按 event 更新当前控件和配置，返回必要的重绘或关闭动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if let WindowEvent::PointerButton {
            position,
            button: window::LEFT_BUTTON,
            pressed: true,
        } = event
            && let Some(edge) = super::layout::resize_edge(self.size, position)
        {
            return WindowAction::ResizeEdge(edge);
        }
        if matches!(event, WindowEvent::Wake) {
            super::translation::input::event(self, &event);
            return WindowAction::Redraw;
        }
        if let Some(action) = super::dropdown::input::event(self, &event) {
            return action;
        }
        let shell_hover_changed = match &event {
            WindowEvent::PointerMove { position } => self.hit(self.pointer) != self.hit(*position),
            _ => false,
        };
        if (self.translation_page() || matches!(event, WindowEvent::Wake))
            && let Some(action) = super::translation::input::event(self, &event)
        {
            return if shell_hover_changed && matches!(action, WindowAction::None) {
                WindowAction::Redraw
            } else {
                action
            };
        }
        if self.toolbar_page()
            && let Some(action) = super::toolbar::input::event(self, &event)
        {
            return if shell_hover_changed && matches!(action, WindowAction::None) {
                WindowAction::Redraw
            } else {
                action
            };
        }
        match event {
            WindowEvent::Resize { size } => {
                self.size = size;
                self.toolbar.scroll = self
                    .toolbar
                    .scroll
                    .min(super::toolbar::layout::Layout::new(self).maximum());
                self.scroll = self.scroll.min(super::layout::Layout::new(self).maximum);
            }
            WindowEvent::PointerMove { position } => {
                if self.pressed.is_some() && self.edit_pointer(position, true) {
                    self.pointer = position;
                    return WindowAction::Redraw;
                }
                if let Some(drag) = self.scrollbar_drag
                    && let Some(bar) = self.scrollbar()
                {
                    self.scroll = drag.value(position, &bar);
                    self.pointer = position;
                    return WindowAction::Redraw;
                }
                let before = self.hit(self.pointer);
                self.pointer = position;
                if !shell_hover_changed && before == self.hit(position) {
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
                    self.edit_pointer(position, false);
                    if self.press_scrollbar(position) {
                        return WindowAction::Redraw;
                    }
                    if position.0 >= self.size.0 as f64 - 18.0
                        && position.1 >= self.size.1 as f64 - 18.0
                    {
                        return WindowAction::Resize;
                    }
                    if hit.is_none() && position.1 < 48.0 {
                        return WindowAction::Move;
                    }
                    self.pressed = hit;
                } else if let Some(control) =
                    self.pressed.take().filter(|control| Some(*control) == hit)
                {
                    if control == Control::Close {
                        return if self.commit_pending() {
                            WindowAction::Close
                        } else {
                            WindowAction::Redraw
                        };
                    }
                    self.activate(control);
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
                if input.key != Some(Key::Escape)
                    && let Some((index, _)) = &self.editing
                    && matches!(ENTRIES[*index].kind, Kind::Shortcut)
                {
                    self.record_shortcut(*index, &input);
                    return WindowAction::Redraw;
                }
                if input.key == Some(Key::Escape) {
                    if self.editing.take().is_none() && !self.search_focus {
                        return if self.commit_pending() {
                            WindowAction::Close
                        } else {
                            WindowAction::Redraw
                        };
                    }
                    self.search_focus = false;
                } else if input.mods.ctrl && input.key == Some(Key::Char('f')) {
                    if !self.commit_pending() {
                        return WindowAction::Redraw;
                    }
                    self.search_focus = true;
                    self.editing = None;
                    self.search.select_all(&mut self.fonts);
                } else if self.search_focus {
                    self.search.key(&mut self.fonts, &input);
                    self.scroll = 0;
                } else if input.key == Some(Key::Tab) {
                    if self.commit_pending() {
                        self.advance_focus(input.mods.shift);
                    }
                } else if input.key == Some(Key::Enter) && self.editing.is_some() {
                    self.apply();
                } else if let Some((index, field)) = &mut self.editing {
                    if matches!(ENTRIES[*index].kind, Kind::Shortcut) {
                        let index = *index;
                        self.record_shortcut(index, &input);
                    } else {
                        field.key(&mut self.fonts, &input);
                    }
                } else if self.scroll_key(input.key) {
                    self.focus = None;
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
        if let Some(edge) = super::layout::resize_edge(self.size, position) {
            use window::WindowEdge;
            return match edge {
                WindowEdge::Top | WindowEdge::Bottom => WindowCursor::VerticalResize,
                WindowEdge::Left | WindowEdge::Right => WindowCursor::HorizontalResize,
                _ => WindowCursor::Resize,
            };
        }
        if let Some(menu) = &self.dropdown {
            return match super::dropdown::layout::Layout::new(menu, self.size).hit(position) {
                Some(super::dropdown::layout::Hit::Search) => WindowCursor::Text,
                Some(_) => WindowCursor::Pointer,
                None => WindowCursor::Default,
            };
        }
        if self.translation_page()
            && super::translation::layout::Layout::new(self)
                .hit(position)
                .is_some()
        {
            return WindowCursor::Pointer;
        }
        if self.toolbar_page() {
            if self.toolbar.drag.as_ref().is_some_and(|drag| drag.active) {
                return WindowCursor::Grabbing;
            }
            if let Some(hit) = super::toolbar::layout::Layout::new(self).hit(position) {
                return if matches!(hit, super::toolbar::layout::Hit::Item(..)) {
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
