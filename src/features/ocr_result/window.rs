//! 【文字识别】【结果交互】独立结果窗口的页面、编辑和复制操作

use std::time::Instant;

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;

use super::{
    document::OcrDocument,
    feedback::{Feedback, Pane},
    layout::{Control, Layout, contains},
    table,
    table_view::TableView,
};
use crate::{
    AppResult,
    backend::window::{self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec},
    ui::text_editor::{EditEvent, EditorStyle, TextEditor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Text,
    Image,
    Table,
}

pub(super) struct ResultWindow {
    pub language: crate::ui::language::Language,
    pub fonts: FontSystem,
    pub cache: SwashCache,
    pub icons: crate::renderer::Icons,
    pub source: TextEditor,
    pub(super) scrollbars: super::scrollbars::State,
    pub(super) translation: super::translation::Translation,
    pub(super) translate_on_connect: bool,
    pub(super) stacking: super::stacking::State,
    pub(super) text_focus: super::control_input::FocusTarget,
    pub image: Pixmap,
    pub png: Vec<u8>,
    pub table: Option<TableView>,
    pub view: View,
    pub layout: Layout,
    pub hovered: Option<Control>,
    pub(super) pointer: Option<(f64, f64)>,
    pub source_menu: Option<crate::ui::context_menu::ContextMenu>,
    pub(super) popup_kind: super::source_menu::Kind,
    pub(super) popup_editor_id: Option<(super::translation::input::Field, u64)>,
    pub(super) preview: super::source_preview::SourcePreview,
    pub focused: bool,
    pub(super) focus_target: super::control_input::FocusTarget,
    pub(super) control_press: Option<super::control_input::Press>,
    pub status: Option<String>,
    pub(super) feedback: Feedback,
    pub(super) more_menu: super::more_menu::MoreMenu,
    pub(super) dragging: bool,
    pub(super) last_click: Option<(Instant, (f64, f64), u8)>,
}

impl Drop for ResultWindow {
    /// 【OCR 翻译】【窗口最终回收】事件循环结束后等待当前及已取消任务退出，无参数、无返回值，避免应用退出先于命令进程组清理
    fn drop(&mut self) {
        if let Some(mut task) = self.translation.task.take() {
            task.shutdown();
        }
        for mut task in self.translation.retired.drain(..) {
            task.shutdown();
        }
    }
}

impl ResultWindow {
    /// 【文字识别】【结果初始化】将 document 转为编辑器、原图和可选表格，返回独立窗口；界面和正文只加载必要字体
    pub(super) fn new(document: OcrDocument) -> AppResult<Self> {
        #[cfg(test)]
        let language = crate::ui::language::Language::English;
        #[cfg(not(test))]
        let language = crate::ui::language::Language::current();
        let image = document.image()?;
        let mut fonts = crate::fonts::ocr_ui::new();
        crate::fonts::ocr_ui::ensure_text(&mut fonts, language.font_sample());
        crate::fonts::ocr_ui::ensure_text(&mut fonts, &document.text);
        let colors = super::palette::Palette::current();
        let source = TextEditor::new_with_style(
            &document.text,
            &mut fonts,
            EditorStyle {
                size: super::labels::BODY_FONT,
                line_height: super::labels::BODY_LINE_HEIGHT,
                colors: Some([colors.text, colors.text, colors.accent, colors.on_accent]),
            },
        );
        let table = table::infer(&document.lines).map(|table| TableView::new(table, &mut fonts));
        #[cfg(test)]
        let translation_config = crate::config::Translation::default();
        #[cfg(not(test))]
        let translation_config = crate::config::get().translation.clone();
        let translation =
            super::translation::Translation::localized(&mut fonts, translation_config, language);
        let layout = Layout::localized(
            super::layout::INITIAL_SIZE,
            table.is_some(),
            true,
            View::Text,
            "Copy",
            "Copy",
            false,
            false,
            language,
        );
        Ok(Self {
            language,
            fonts,
            cache: SwashCache::new(),
            icons: super::titlebar_draw::icons(),
            source,
            scrollbars: Default::default(),
            translation,
            translate_on_connect: false,
            stacking: super::stacking::State::new(),
            text_focus: super::control_input::FocusTarget::Content,
            image,
            png: document.png,
            table,
            view: View::Text,
            layout,
            hovered: None,
            pointer: None,
            source_menu: None,
            popup_kind: super::source_menu::Kind::Text,
            popup_editor_id: None,
            preview: super::source_preview::SourcePreview::default(),
            focused: true,
            focus_target: super::control_input::FocusTarget::Content,
            control_press: None,
            status: None,
            feedback: Feedback::default(),
            more_menu: super::more_menu::MoreMenu::default(),
            dragging: false,
            last_click: None,
        })
    }

    /// 【文字识别】【字体补充】按 text 补充缺少字形的定向字体文件，无返回值，不扫描全部系统字体
    pub(super) fn ensure_fonts(&mut self, text: &str) {
        crate::fonts::ocr_ui::ensure_text(&mut self.fonts, text);
    }

    /// 【文字识别】【布局更新】根据当前页面、反馈和正文撤销刷新几何，无参数和返回值；原文编辑状态不变
    pub(super) fn refresh_layout(&mut self) {
        super::translation::controller::refresh(self);
        let source = self.feedback.label(Pane::Source);
        let table = self.feedback.label(Pane::Table);
        let undo = self.view == View::Text && self.source.can_undo();
        let status = self.status.is_some();
        let translation = self.view == View::Text && self.translation.state.visible;
        if self.layout.titlebar.language == self.language
            && self.layout.copy.width() == super::labels::copy_width(self.language.text(source))
            && self.layout.titlebar.slots[0].is_some() == (self.view == View::Text)
            && self.layout.undo.is_some() == undo
            && self
                .layout
                .titlebar
                .tabs
                .iter()
                .any(|(view, _)| *view == View::Table)
                == self.table.is_some()
            && self.layout.table_copy.width()
                == super::labels::table_copy_width(table, self.language)
            && (self.layout.grip.right() == self.layout.size.0 - 8.0) == (status || translation)
            && self.layout.translation.is_some() == translation
            && !self.translation.layout_dirty
        {
            return;
        }
        self.layout = Layout::localized(
            (self.layout.size.0 as u32, self.layout.size.1 as u32),
            self.table.is_some(),
            true,
            self.view,
            source,
            table,
            undo,
            status,
            self.language,
        );
        if translation {
            let notice = self.translation.state.localized_notice(self.language);
            let layout = super::translation::layout::Layout::localized(
                self.layout.size,
                self.translation.ratio,
                self.translation.editor.can_undo(),
                self.language.text(self.feedback.label(Pane::Translation)),
                &notice,
                self.translation.state.busy(),
                &mut self.fonts,
                self.language,
            );
            let source = layout.source;
            let top = source.y() + if undo { 22.0 } else { 0.0 };
            self.layout.body = super::titlebar_layout::rect(
                8.0,
                71.0,
                self.layout.size.0 - 16.0,
                self.layout.size.1 - 97.0,
            );
            self.layout.editor_frame = super::titlebar_layout::rect(
                source.x(),
                top,
                source.width(),
                source.bottom() - top,
            );
            self.layout.editor = super::titlebar_layout::rect(
                source.x() + 3.0,
                top + 3.0,
                source.width() - 6.0,
                source.bottom() - top - 6.0,
            );
            self.layout.undo = undo.then(|| {
                super::titlebar_layout::rect(source.right() - 28.0, source.y(), 28.0, 20.0)
            });
            self.layout.grip = super::titlebar_layout::rect(
                self.layout.size.0 - 26.0,
                self.layout.size.1 - 22.0,
                18.0,
                18.0,
            );
            self.layout.translation = Some(layout);
        }
        self.translation.layout_dirty = false;
        if let Some(position) = self.pointer {
            self.hovered = self.layout.control(position, self.view);
        }
    }

    /// 【文字识别】【复制反馈】将 text 复制到持久剪贴板；返回是否成功并在窗口内反馈
    pub(super) fn copy_text(&mut self, text: &str) -> bool {
        self.copy_pane_text(text, Pane::Source)
    }

    /// 【文字识别】【分区复制】将 text 交付剪贴板并更新 pane 自己的反馈期限，返回是否成功，不改变其他分区或页脚提示
    pub(super) fn copy_pane_text(&mut self, text: &str, pane: Pane) -> bool {
        let result = crate::utils::copy_to_clipboard(text);
        self.feedback.show(pane, result.is_ok(), Instant::now());
        self.refresh_layout();
        if let Err(error) = result {
            log::error!("【文字识别】【结果复制】文字复制失败: {error}");
            false
        } else {
            true
        }
    }

    /// 【文字识别】【编辑器复制】将选区 text 交付剪贴板，返回是否成功；普通编辑器复制与剪切不触发全文复制按钮反馈
    fn copy_editor_text(&mut self, text: &str) -> bool {
        match crate::utils::copy_to_clipboard(text) {
            Ok(()) => true,
            Err(error) => {
                log::error!("【文字识别】【结果复制】文字复制失败: {error}");
                self.status = Some(format!("Copy failed: {error}"));
                false
            }
        }
    }

    /// 【文字识别】【编辑结果】处理 event 的剪贴板与修改请求；只在复制成功后剪切选中文字
    pub(super) fn edit_event(&mut self, event: EditEvent) -> WindowAction {
        match event {
            EditEvent::None => return WindowAction::None,
            EditEvent::Copy(text) => {
                self.copy_editor_text(&text);
            }
            EditEvent::Cut(text) => {
                if self.copy_editor_text(&text)
                    && let Some(field) = super::translation::input::field(self)
                    && super::translation::input::editable(self, field)
                {
                    super::translation::input::with_editor(self, field, |editor, fonts| {
                        editor.delete_selection(fonts)
                    });
                }
            }
            EditEvent::Paste => {
                if let Some(text) = crate::utils::paste_from_clipboard() {
                    self.ensure_fonts(&text);
                    let changed = super::translation::input::field(self)
                        .filter(|field| super::translation::input::editable(self, *field))
                        .and_then(|field| {
                            super::translation::input::with_editor(self, field, |editor, fonts| {
                                editor.insert(fonts, &text)
                            })
                        })
                        .unwrap_or(false);
                    if changed {
                        self.status = None;
                    }
                }
            }
            EditEvent::Changed => {
                self.status = None;
            }
            EditEvent::Redraw => {}
        }
        WindowAction::Redraw
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
    pub(super) fn control(&mut self, control: Control) -> WindowAction {
        if control == Control::Undo && !self.can_undo() {
            return WindowAction::None;
        }
        self.dragging = false;
        self.translation.splitter_drag = false;
        self.last_click = None;
        match control {
            Control::Close => return WindowAction::Close,
            Control::View(view) => {
                if self.view == View::Text
                    && matches!(
                        self.focus_target,
                        super::control_input::FocusTarget::Content
                            | super::control_input::FocusTarget::Translation
                    )
                {
                    self.text_focus = self.focus_target;
                }
                if self.view != view {
                    super::input_method::cancel_non_text(self);
                }
                self.focus_target = if view == View::Text && self.translation.state.visible {
                    self.text_focus
                } else {
                    super::control_input::FocusTarget::Content
                };
                self.control_press = None;
                if self.view == View::Table
                    && view != View::Table
                    && let Some(table) = &mut self.table
                {
                    table.bars.cancel();
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
                self.preview.release();
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
                self.status = None;
            }
            Control::Copy => {
                let text = self.source.text();
                self.copy_text(&text);
            }
            Control::TableCopy => return super::table_menu::open(self),
            Control::More => return super::more_menu::open(self),
            Control::TranslationToggle => return super::translation::controller::toggle(self),
            Control::Translate => {
                if self.translation.state.busy() {
                    super::translation::controller::cancel(self, true);
                } else {
                    return super::translation::controller::start(self);
                }
            }
            Control::TranslationCopy => {
                self.copy_pane_text(&self.translation.editor.text(), Pane::Translation);
            }
            Control::TranslationUndo => {
                if !self.translation.state.busy() {
                    self.translation.editor.undo(&mut self.fonts);
                }
            }
            Control::LanguageMenu => return super::translation::language_menu::open(self),
            Control::AlwaysOnTop => return super::stacking::toggle(self),
        }
        self.refresh_layout();
        WindowAction::Redraw
    }
}

impl WindowContent for ResultWindow {
    /// 【OCR 结果窗口】【置顶请求】返回本窗口独立开关，无参数，不复用贴图设置
    fn always_on_top(&self) -> bool {
        self.stacking.active
    }

    /// 【OCR 结果窗口】【置顶能力】保存 supported，缺少真实协议时禁用入口且不显示已置顶，不改写用户配置，无返回值
    fn stacking_supported(&mut self, supported: bool) {
        self.stacking.supported = supported;
        if !supported && self.stacking.active {
            self.stacking.active = false;
            log::error!(
                "【OCR 结果窗口】【置顶能力】当前合成器没有 layer-shell 协议，使用普通窗口"
            );
        }
    }
    /// 【OCR 翻译】【窗口连接】保存 wake 以接收所属任务完成通知，无返回值，不提前初始化任务或网络
    fn connected(&mut self, wake: window::WindowWake) {
        self.translation.wake = Some(wake);
        if std::mem::take(&mut self.translate_on_connect) {
            super::translation::controller::start(self);
        }
    }
    /// 【文字识别】【反馈期限】返回原文和表格反馈的最近绝对期限，无参数；全部到期后恢复无期限阻塞
    fn next_deadline(&self) -> Option<Instant> {
        self.feedback
            .deadline()
            .into_iter()
            .chain(
                self.more_menu
                    .deadline(self.source_menu.as_ref().map(|menu| menu.id)),
            )
            .chain(
                (self.view == View::Text && self.translation.state.visible)
                    .then_some(self.translation.animation_deadline)
                    .flatten(),
            )
            .chain(self.translation.retire_deadline)
            .chain(self.stacking.deadline)
            .chain(self.table.as_ref().and_then(|table| table.bars.deadline()))
            .min()
    }

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
        self.refresh_layout();
        super::input_method::snapshot(self, focus, target_id, previous, cause)
    }

    /// 【文字识别】【候选取消】清除全部显示候选并返回是否需要重绘，无参数；不修改确认正文
    fn cancel_text_input(&mut self) -> bool {
        super::input_method::cancel(self)
    }

    /// 【文字识别】【候选停用】临时切换页签或菜单时保留多行候选外观，返回其他候选是否清理；协议目标始终重新分配，无参数
    fn suspend_text_input(&mut self) -> bool {
        super::input_method::cancel_non_text(self)
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
        crate::ui::window_frame::draw(canvas, scale);
    }

    /// 【文字识别】【窗口事件】处理 event 并返回窗口动作，菜单期间隔离正文输入
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match &event {
            WindowEvent::PointerButton { position, .. } | WindowEvent::PointerMove { position } => {
                self.pointer = Some(*position)
            }
            WindowEvent::PointerLeave => self.pointer = None,
            _ => {}
        }
        self.refresh_layout();
        let action = self.input_event(event);
        self.refresh_layout();
        action
    }

    /// 【文字识别】【菜单身份】返回当前原文菜单的会话编号，无参数
    fn popup_id(&self) -> Option<u64> {
        self.translation
            .language_menu
            .as_ref()
            .map(|menu| menu.id)
            .or_else(|| self.source_menu.as_ref().map(|menu| menu.id))
    }

    /// 【文字识别】【菜单父链】返回当前根菜单及真实子菜单定位节点，无参数；普通右键菜单保留点定位
    fn popup_nodes(&self) -> Vec<crate::types::popup_tree::PopupNode> {
        if let Some(menu) = &self.translation.language_menu {
            let mut node = crate::types::popup_tree::PopupNode::root(menu.id);
            node.placement = crate::types::popup_tree::Placement::BelowControl(menu.control);
            return vec![node];
        }
        let Some(root) = &self.source_menu else {
            return Vec::new();
        };
        let mut node = crate::types::popup_tree::PopupNode::root(root.id);
        if let Some(bounds) = root.control_anchor {
            node.placement = crate::types::popup_tree::Placement::BelowControl(bounds);
        }
        let mut nodes = vec![node];
        if let Some(child) = self.more_menu.child_node(root) {
            nodes.push(child);
        }
        nodes
    }

    /// 【文字识别】【逐层菜单帧】按 id 和 scale 返回相应菜单的变化像素，父子菜单使用不同原生表面
    fn popup_frame_for(
        &mut self,
        id: u64,
        scale: f32,
    ) -> Option<crate::types::popup::PopupFrame<'_>> {
        if self
            .translation
            .language_menu
            .as_ref()
            .is_some_and(|menu| menu.id == id)
        {
            return super::translation::language_menu::frame(self, scale);
        }
        if self.popup_id() == Some(id) {
            super::source_menu::frame(self, scale)
        } else {
            super::more_menu::frame(self, id, scale)
        }
    }

    /// 【文字识别】【菜单帧】按 scale 返回变化后的独立原文菜单帧，不写入父窗口画布
    fn popup_frame(&mut self, scale: f32) -> Option<crate::types::popup::PopupFrame<'_>> {
        if self.translation.language_menu.is_some() {
            return super::translation::language_menu::frame(self, scale);
        }
        super::source_menu::frame(self, scale)
    }

    /// 【文字识别】【菜单事件】将 id 对应 event 转换为菜单状态或待执行动作，此阶段不修改正文
    fn popup_event(
        &mut self,
        id: u64,
        event: crate::types::popup::PopupEvent,
    ) -> window::WindowPopupAction {
        if self
            .translation
            .language_menu
            .as_ref()
            .is_some_and(|menu| menu.id == id)
        {
            return super::translation::language_menu::event(self, id, event);
        }
        super::source_menu::event(self, id, event)
    }

    /// 【文字识别】【菜单交付】原生表面销毁后执行 id 对应 action，空值为取消；返回父窗口动作
    fn popup_closed(&mut self, id: u64, action: Option<usize>) -> WindowAction {
        if self
            .translation
            .language_menu
            .as_ref()
            .is_some_and(|menu| menu.id == id)
        {
            return super::translation::language_menu::closed(self, id, action);
        }
        super::source_menu::closed(self, id, action)
    }

    /// 【文字识别】【结果指针】按 position 返回文字或调整大小指针；其他区域使用默认指针
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if let Some(layout) = &self.layout.translation {
            if contains(layout.splitter, position) || self.translation.splitter_drag {
                return if layout.horizontal {
                    WindowCursor::HorizontalResize
                } else {
                    WindowCursor::VerticalResize
                };
            }
            if contains(layout.frame, position)
                || !self.translation.state.busy() && contains(layout.language_text, position)
            {
                return WindowCursor::Text;
            }
        }
        if self.preview.dragging
            || (self.view == View::Image && contains(self.layout.body, position))
        {
            return self.preview.cursor();
        }
        if self.layout.resize_at(position) {
            return WindowCursor::Resize;
        }
        let text = (self.view == View::Text && contains(self.layout.editor, position))
            || (self.view == View::Table
                && self.table.as_ref().is_some_and(|table| {
                    table.editor.is_some()
                        && table.cell_at(self.layout.table_body, position).is_some()
                        && table
                            .input_rect(self.layout.table_body)
                            .is_some_and(|rect| contains(rect, position))
                }));
        if text && self.layout.control(position, self.view).is_none() {
            WindowCursor::Text
        } else {
            WindowCursor::Default
        }
    }

    /// 【文字识别】【完整滚轮】按 input 分发原版表格逐项刻度，其他页面继续使用已有像素滚动，返回窗口动作
    fn scroll(&mut self, input: window::ScrollInput) -> WindowAction {
        self.scroll_input(input)
    }
}

/// 【文字识别】【结果窗口】打开 document 的可编辑普通窗口；关闭窗口后释放原图、字体和编辑状态
pub fn run(document: OcrDocument) -> AppResult {
    let content = ResultWindow::new(document)?;
    window::run(
        WindowSpec {
            title: content.language.text("OCR Result").into(),
            app_id: "mark-shot-ocr-result".into(),
            size: super::layout::INITIAL_SIZE,
            minimum: super::layout::MINIMUM_SIZE,
        },
        content,
    )
}
