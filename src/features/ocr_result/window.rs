//! 【文字识别】【结果交互】独立结果窗口的页面、编辑和复制操作

use std::time::{Duration, Instant};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;

use super::{
    document::OcrDocument,
    export,
    layout::{Control, Layout, contains},
    table,
    table_view::TableView,
};
use crate::{
    AppResult,
    backend::wayland::window::{
        self, KeyInput, LEFT_BUTTON, WindowAction, WindowContent, WindowCursor, WindowEvent,
        WindowSpec,
    },
    keys::Key,
    ui::text_editor::{EditEvent, TextEditor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Text,
    Image,
    Table,
}

impl View {
    /// 【文字识别】【页面名称】返回当前页面的简短按钮名称
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Image => "Image",
            Self::Table => "Table",
        }
    }
}

pub(super) struct ResultWindow {
    pub fonts: FontSystem,
    pub cache: SwashCache,
    pub source: TextEditor,
    pub image: Pixmap,
    pub png: Vec<u8>,
    pub table: Option<TableView>,
    pub view: View,
    pub layout: Layout,
    pub hovered: Option<Control>,
    pub menu: bool,
    pub focused: bool,
    pub control_focus: Option<Control>,
    pub status: Option<String>,
    pub copied: bool,
    dragging: bool,
    system_fonts: bool,
    last_click: Option<(Instant, (f64, f64), u8)>,
}

impl ResultWindow {
    /// 【文字识别】【结果初始化】将 document 转为编辑器、原图和可选表格；仅非 ASCII 原文加载系统字体
    pub(super) fn new(document: OcrDocument) -> AppResult<Self> {
        let image = document.image()?;
        let mut fonts = crate::fonts::bundled();
        let system_fonts = !document.text.is_ascii();
        if system_fonts {
            crate::fonts::add_system(&mut fonts, crate::fonts::system());
        }
        let source = TextEditor::new(&document.text, &mut fonts);
        let table = table::infer(&document.lines).map(TableView::new);
        let layout = Layout::new((760, 460), table.is_some());
        Ok(Self {
            fonts,
            cache: SwashCache::new(),
            source,
            image,
            png: document.png,
            table,
            view: View::Text,
            layout,
            hovered: None,
            menu: false,
            focused: true,
            control_focus: None,
            status: None,
            copied: false,
            dragging: false,
            system_fonts,
            last_click: None,
        })
    }

    /// 【文字识别】【字体补充】text 首次包含非 ASCII 字符时补充系统字体；不重复扫描字体目录
    pub(super) fn ensure_fonts(&mut self, text: &str) {
        if !self.system_fonts && !text.is_ascii() {
            crate::fonts::add_system(&mut self.fonts, crate::fonts::system());
            self.system_fonts = true;
        }
    }

    /// 【文字识别】【复制反馈】将 text 复制到持久剪贴板；返回是否成功并在窗口内反馈
    fn copy_text(&mut self, text: &str) -> bool {
        match crate::utils::copy_to_clipboard(text) {
            Ok(()) => {
                self.status = Some("Copied".into());
                self.copied = true;
                true
            }
            Err(error) => {
                log::error!("【文字识别】【结果复制】文字复制失败: {error}");
                self.status = Some(format!("Copy failed: {error}"));
                self.copied = false;
                false
            }
        }
    }

    /// 【文字识别】【编辑结果】处理 event 的剪贴板与修改请求；只在复制成功后剪切选中文字
    fn edit_event(&mut self, event: EditEvent) -> WindowAction {
        match event {
            EditEvent::None => return WindowAction::None,
            EditEvent::Copy(text) => {
                self.copy_text(&text);
            }
            EditEvent::Cut(text) => {
                if self.copy_text(&text) {
                    if self.view == View::Text {
                        self.source.delete_selection(&mut self.fonts);
                    } else if let Some(table) = &mut self.table
                        && let Some(editor) = &mut table.editor
                    {
                        editor.delete_selection(&mut self.fonts);
                    }
                }
            }
            EditEvent::Paste => {
                if let Some(text) = crate::utils::paste_from_clipboard() {
                    self.ensure_fonts(&text);
                    let changed = if self.view == View::Text {
                        self.source.insert(&mut self.fonts, &text)
                    } else if let Some(table) = &mut self.table {
                        table
                            .editor
                            .as_mut()
                            .is_some_and(|editor| editor.insert(&mut self.fonts, &text))
                    } else {
                        false
                    };
                    if changed {
                        self.status = None;
                        self.copied = false;
                    }
                }
            }
            EditEvent::Changed => {
                self.status = None;
                self.copied = false;
            }
            EditEvent::Redraw => {}
        }
        WindowAction::Redraw
    }

    /// 【文字识别】【结果快捷键】处理 input 的关闭、全文复制与编辑动作；原图页不接收文字编辑
    fn key(&mut self, input: KeyInput) -> WindowAction {
        if !self.focused {
            return WindowAction::None;
        }
        if input.key == Some(Key::Escape) {
            if self.menu {
                self.menu = false;
                return WindowAction::Redraw;
            }
            if self.view == View::Table
                && self.table.as_mut().is_some_and(|table| table.finish(false))
            {
                return WindowAction::Redraw;
            }
            return WindowAction::Close;
        }
        if let Some(control) = self.control_focus {
            if input.key == Some(Key::Tab) && !input.mods.ctrl && !input.mods.alt {
                self.control_focus = None;
                return WindowAction::Redraw;
            }
            if matches!(input.key, Some(Key::Enter | Key::Char(' ')))
                && !input.mods.ctrl
                && !input.mods.alt
            {
                return self.control(control);
            }
            if !(input.mods.ctrl && input.mods.shift && input.key == Some(Key::Char('c'))) {
                return WindowAction::None;
            }
        }
        self.ensure_fonts(&input.text);
        if input.mods.ctrl && input.mods.shift && input.key == Some(Key::Char('c')) {
            self.copy_text(&self.source.text());
            return WindowAction::Redraw;
        }
        let event = match self.view {
            View::Text => self.source.key(&mut self.fonts, &input),
            View::Table => self.table.as_mut().map_or(EditEvent::None, |table| {
                table.key(&mut self.fonts, self.layout.body, &input)
            }),
            View::Image => return WindowAction::None,
        };
        self.edit_event(event)
    }

    /// 【文字识别】【撤销可用】返回当前编辑区是否具有撤销记录；原图页不执行文字或表格撤销
    pub(super) fn can_undo(&self) -> bool {
        match self.view {
            View::Text => self.source.can_undo(),
            View::Table => self
                .table
                .as_ref()
                .and_then(|table| table.editor.as_ref())
                .is_some_and(TextEditor::can_undo),
            View::Image => false,
        }
    }

    /// 【文字识别】【控件动作】执行 control 对应页面切换、撤销、复制或关闭；保留编辑器与滚动状态
    fn control(&mut self, control: Control) -> WindowAction {
        if control == Control::Undo && !self.can_undo() {
            return WindowAction::None;
        }
        self.dragging = false;
        self.last_click = None;
        match control {
            Control::Close => return WindowAction::Close,
            Control::View(view) => {
                if self.view != view {
                    super::input_method::cancel(self);
                }
                self.control_focus = None;
                if self.view == View::Table
                    && view != View::Table
                    && let Some(table) = &mut self.table
                {
                    table.finish(true);
                }
                self.view = view;
                if view == View::Table
                    && let Some(table) = &mut self.table
                    && table.active.is_none()
                    && !table.table.rows.is_empty()
                {
                    table.active = Some((0, 0));
                }
                self.menu = false;
                self.copied = false;
                self.status = None;
            }
            Control::Undo => {
                if self.view == View::Text {
                    self.source.undo(&mut self.fonts);
                } else if let Some(table) = &mut self.table
                    && let Some(editor) = &mut table.editor
                {
                    editor.undo(&mut self.fonts);
                }
                self.copied = false;
                self.status = None;
            }
            Control::Copy => match self.view {
                View::Text => {
                    let text = self.source.text();
                    self.copy_text(&text);
                }
                View::Table => {
                    if let Some(table) = &mut self.table {
                        table.finish(true);
                    }
                    self.menu = !self.menu;
                }
                View::Image => {
                    match crate::backend::initialize_clipboard().copy_image_to_clipboard(&self.png)
                    {
                        Ok(()) => {
                            self.status = Some("Image copied".into());
                            self.copied = true;
                        }
                        Err(error) => {
                            log::error!("【文字识别】【原图复制】原图复制失败: {error}");
                            self.status = Some(format!("Copy failed: {error}"));
                            self.copied = false;
                        }
                    }
                }
            },
            Control::Format(format) => {
                if let Some(table) = &mut self.table {
                    table.finish(true);
                    let text = export::text(&table.table, format);
                    self.copy_text(&text);
                }
                self.menu = false;
            }
        }
        WindowAction::Redraw
    }

    /// 【文字识别】【连续点击】根据 position 与上次点击时间返回单击、双击或三击次数
    fn clicks(&mut self, position: (f64, f64)) -> u8 {
        let now = Instant::now();
        let count = self
            .last_click
            .filter(|(time, previous, _)| {
                now.duration_since(*time) <= Duration::from_millis(450)
                    && (previous.0 - position.0).hypot(previous.1 - position.1) < 5.0
            })
            .map_or(1, |(_, _, count)| if count == 3 { 1 } else { count + 1 });
        self.last_click = Some((now, position, count));
        count
    }

    /// 【文字识别】【正文选择】将 position 转换为当前页本地坐标；drag、clicks 控制文字选择范围
    fn select(&mut self, position: (f64, f64), drag: bool, clicks: u8) {
        match self.view {
            View::Text => {
                let rect = self.layout.editor;
                self.source
                    .resize(&mut self.fonts, rect.width(), rect.height());
                self.source.pointer(
                    &mut self.fonts,
                    (
                        position.0 - f64::from(rect.x()),
                        position.1 - f64::from(rect.y()),
                    ),
                    drag,
                    clicks,
                );
            }
            View::Table => {
                if let Some(table) = &mut self.table {
                    table.pointer(&mut self.fonts, self.layout.body, position, drag, clicks);
                }
            }
            View::Image => {}
        }
    }

    /// 【文字识别】【鼠标按下】按 position 处理控件、标题移动、尺寸调整或正文编辑；返回窗口动作
    fn press(&mut self, position: (f64, f64)) -> WindowAction {
        if let Some(control) = self.layout.control(position, self.menu) {
            if matches!(control, Control::Copy | Control::Undo) {
                super::input_method::cancel(self);
                self.control_focus = Some(control);
            }
            return self.control(control);
        }
        if self.menu {
            self.menu = false;
            return WindowAction::Redraw;
        }
        if self.layout.resize_at(position) {
            return WindowAction::Resize;
        }
        if position.1 < 36.0 {
            return WindowAction::Move;
        }
        if contains(self.layout.body, position) {
            self.control_focus = None;
            self.dragging = self.view != View::Image;
            let clicks = self.clicks(position);
            self.select(position, false, clicks);
            return WindowAction::Redraw;
        }
        WindowAction::None
    }
}

impl WindowContent for ResultWindow {
    /// 【文字识别】【输入拥有者】返回当前真正编辑的原文或单元格身份，无参数
    fn text_input_target(&self) -> Option<u64> {
        super::input_method::identity(self)
    }

    /// 【文字识别】【协议快照】按 focus、target_id、previous、cause 返回变化后的输入快照，不复制静止正文
    fn text_input_snapshot(
        &mut self,
        focus: crate::types::text_input::Focus,
        target_id: u64,
        previous: Option<&crate::types::text_input::Snapshot>,
        cause: crate::types::text_input::ChangeCause,
    ) -> Option<crate::types::text_input::Snapshot> {
        super::input_method::snapshot(self, focus, target_id, previous, cause)
    }

    /// 【文字识别】【候选取消】清除全部显示候选并返回是否需要重绘，无参数；不修改确认正文
    fn cancel_text_input(&mut self) -> bool {
        super::input_method::cancel(self)
    }
    /// 【文字识别】【窗口就绪】首帧提交后向父进程确认就绪；父进程此后可以关闭截图覆盖层
    fn mapped(&mut self) {
        use std::io::Write;
        if let Err(error) = std::io::stdout().write_all(b"\n") {
            log::error!("【文字识别】【窗口就绪】无法确认窗口就绪: {error}");
        }
    }

    /// 【文字识别】【结果绘制】按 scale 绘制 canvas；页面共用同一窗口与编辑状态
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【文字识别】【窗口事件】处理 event；空闲、无状态变化时不安排重绘
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Resize { size } => {
                self.layout = Layout::new(size, self.table.is_some());
                WindowAction::Redraw
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.dragging = false;
                if !focused {
                    super::input_method::cancel(self);
                    if let Some(table) = &mut self.table {
                        table.finish(true);
                    }
                }
                WindowAction::Redraw
            }
            WindowEvent::TextInput(edit) => super::input_method::apply(self, edit),
            WindowEvent::Key(input) => self.key(input),
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: true,
            } => self.press(position),
            WindowEvent::PointerButton { pressed: false, .. } => {
                self.dragging = false;
                WindowAction::None
            }
            WindowEvent::PointerMove { position } => {
                let hovered = self.layout.control(position, self.menu);
                let changed = self.hovered != hovered;
                self.hovered = hovered;
                if self.dragging {
                    self.select(position, true, 1);
                }
                if changed || self.dragging {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerLeave => {
                self.hovered = None;
                self.dragging = false;
                WindowAction::Redraw
            }
            WindowEvent::Scroll { position, delta } if contains(self.layout.body, position) => {
                match self.view {
                    View::Text => self.source.scroll(&mut self.fonts, delta.1 as f32),
                    View::Table => {
                        if let Some(table) = &mut self.table {
                            table.scroll(self.layout.body, delta);
                        }
                    }
                    View::Image => return WindowAction::None,
                }
                WindowAction::Redraw
            }
            _ => WindowAction::None,
        }
    }

    /// 【文字识别】【结果指针】按 position 返回文字或调整大小指针；其他区域使用默认指针
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if self.layout.resize_at(position) {
            return WindowCursor::Resize;
        }
        let text = (self.view == View::Text && contains(self.layout.editor, position))
            || (self.view == View::Table
                && self
                    .table
                    .as_ref()
                    .is_some_and(|table| table.cell_at(self.layout.body, position).is_some()));
        if text && self.layout.control(position, self.menu).is_none() {
            WindowCursor::Text
        } else {
            WindowCursor::Default
        }
    }
}

/// 【文字识别】【结果窗口】打开 document 的可编辑普通窗口；关闭窗口后释放原图、字体和编辑状态
pub fn run(document: OcrDocument) -> AppResult {
    let content = ResultWindow::new(document)?;
    window::run(
        WindowSpec {
            title: "Mark Shot OCR".into(),
            app_id: "mark-shot-ocr-result".into(),
            size: (760, 460),
            minimum: (320, 220),
        },
        content,
    )
}
