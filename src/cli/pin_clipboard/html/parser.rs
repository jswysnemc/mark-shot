//! 【贴图】【HTML解析器】迭代扫描标签、维护样式与结构栈并构建富文本块

use super::super::html_style;
use super::model::{Align, Block, BlockKind, ImageBlock, RichText, Span, SpanStyle};
use super::tags::{
    append_span, finalize_spans, heading_level, is_block, is_block_background_tag, is_heading,
    is_outer_container_tag, tag_background, tag_color,
};
use super::token::{Tag, decode_entities, is_raw, parse_start_tag, skip_raw, tag_name};
use crate::theme::Rgba;

mod align;
mod image;
mod table;

use table::TableBuilder;

const IMAGE_COUNT_LIMIT: usize = 16;
const IMAGE_BYTES_LIMIT: usize = 64 * 1024 * 1024;

/// 【贴图】【解析入口】按 html 保留常见格式并返回富文本；脚本、样式与传输头不会成为正文
pub(in crate::cli::pin_clipboard) fn parse(html: &str) -> RichText {
    let body = fragment(html);
    let mut parser = Parser::new();
    parser.scan(body);
    parser.flush();
    RichText {
        blocks: parser.blocks,
        background: parser.background,
        foreground: parser.foreground,
    }
}

/// 【贴图】【片段截取】按 CF_HTML 注释截取有效片段，返回不含系统传输头的正文
fn fragment(html: &str) -> &str {
    const START: &str = "<!--StartFragment-->";
    const END: &str = "<!--EndFragment-->";
    let start = html.find(START);
    let end = html.find(END);
    match (start, end) {
        (Some(start), Some(end)) if end > start => &html[start + START.len()..end],
        (Some(start), _) => &html[start + START.len()..],
        (None, Some(end)) => &html[..end],
        (None, None) => html,
    }
}

/// 【贴图】【列表状态】记录每层有序列表的编号或无序标记
struct ListContext {
    ordered: bool,
    counter: usize,
}

/// 【贴图】【列表项状态】记录当前打开的列表项缩进、标记与是否已写出首个块
#[derive(Clone)]
struct ListMarker {
    depth: usize,
    marker: String,
    used: bool,
}

/// 【贴图】【待建块】记录下一个文本块应使用的类型与对齐
struct Pending {
    kind: BlockKind,
    align: Align,
}

/// 【贴图】【块草稿】记录尚未写出的文本片段
struct Draft {
    kind: BlockKind,
    align: Align,
    background: Option<Rgba>,
    spans: Vec<Span>,
}

/// 【贴图】【标签帧】记录开始标签保存的样式、容器背景与结构类型，供结束标签恢复
struct Frame {
    name: String,
    saved: SpanStyle,
    saved_background: Option<Rgba>,
    kind: FrameKind,
    align_pushed: bool,
}

/// 【贴图】【帧类型】区分普通内联、块、列表、列表项、引用、代码、表格、单元格与居中容器
enum FrameKind {
    Inline,
    Block,
    List,
    ListItem,
    Quote,
    Pre,
    Table,
    Cell,
    Row,
    Center,
}

/// 【贴图】【解析状态】以迭代扫描维护样式栈、列表栈、引用深度、表格与当前草稿
struct Parser {
    blocks: Vec<Block>,
    draft: Option<Draft>,
    style: SpanStyle,
    background: Option<Rgba>,
    foreground: Option<Rgba>,
    block_background: Option<Rgba>,
    block_tag: bool,
    frames: Vec<Frame>,
    lists: Vec<ListContext>,
    list_items: Vec<ListMarker>,
    quotes: usize,
    pre: usize,
    pending: Option<Pending>,
    at_start: bool,
    space: bool,
    center: usize,
    table: Option<TableBuilder>,
    table_depth: usize,
    strip_pre_newline: bool,
    image_bytes: usize,
    image_count: usize,
    images: Vec<ImageBlock>,
    image_align: Option<Align>,
    align_scope: Vec<Align>,
}

impl Parser {
    /// 【贴图】【初始化】新建空解析器，块起始状态为真
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            draft: None,
            style: SpanStyle::default(),
            background: None,
            foreground: None,
            block_background: None,
            block_tag: false,
            frames: Vec::new(),
            lists: Vec::new(),
            list_items: Vec::new(),
            quotes: 0,
            pre: 0,
            pending: None,
            at_start: true,
            space: false,
            center: 0,
            table: None,
            table_depth: 0,
            strip_pre_newline: false,
            image_bytes: 0,
            image_count: 0,
            images: Vec::new(),
            image_align: None,
            align_scope: Vec::new(),
        }
    }

    /// 【贴图】【扫描】按 html 迭代处理文本、注释、声明与标签，不递归嵌套结构
    fn scan(&mut self, html: &str) {
        let mut pos = 0;
        while pos < html.len() {
            let Some(offset) = html[pos..].find('<') else {
                self.text(&html[pos..]);
                break;
            };
            if offset > 0 {
                self.text(&html[pos..pos + offset]);
            }
            pos += offset;
            let tail = &html[pos..];
            if tail.starts_with("<!--") {
                pos = tail
                    .find("-->")
                    .map_or(html.len(), |end| pos + end + "-->".len());
            } else if tail.starts_with("<!") || tail.starts_with("<?") {
                pos = tail.find('>').map_or(html.len(), |end| pos + end + 1);
            } else if let Some(rest) = tail.strip_prefix("</") {
                if let Some(end) = rest.find('>') {
                    let name = tag_name(&rest[..end]);
                    self.end_tag(&name);
                    pos += 2 + end + 1;
                } else {
                    pos = html.len();
                }
            } else if let Some((tag, len)) = parse_start_tag(tail) {
                if is_raw(&tag.name) {
                    pos = skip_raw(html, pos + len, &tag.name);
                } else {
                    self.start_tag(&tag);
                    pos += len;
                }
            } else {
                self.text("<");
                pos += 1;
            }
        }
    }

    /// 【贴图】【文本追加】按 text 解码实体并折叠普通模式空白或保留 pre 原文，写入当前单元格或块草稿
    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.table_depth > 0 && !self.cell_open() {
            return;
        }
        let decoded = decode_entities(text);
        let text = decoded.as_str();
        if !self.images.is_empty() {
            if text.chars().all(|ch| ch.is_whitespace()) {
                return;
            }
            self.flush_images();
        }
        if self.pre > 0 {
            let mut text = text;
            if self.strip_pre_newline {
                self.strip_pre_newline = false;
                text = text
                    .strip_prefix("\r\n")
                    .or_else(|| text.strip_prefix('\n'))
                    .unwrap_or(text);
            }
            if !text.is_empty() {
                let style = self.span_style();
                self.push_span(text, &style);
            }
            return;
        }
        let mut run = String::new();
        for ch in text.chars() {
            if ch.is_ascii_whitespace() {
                if !self.at_start && !self.space {
                    run.push(' ');
                    self.space = true;
                }
            } else {
                run.push(ch);
                self.space = false;
                self.at_start = false;
            }
        }
        if !run.is_empty() {
            let style = self.span_style();
            self.push_span(&run, &style);
        }
    }

    /// 【贴图】【换行】按 br 在当前单元格或块草稿写入真实换行，生成空行
    fn newline(&mut self) {
        self.space = false;
        if self.table_depth > 0 {
            if !self.cell_open() {
                return;
            }
            let style = self.span_style();
            self.push_span("\n", &style);
            return;
        }
        self.flush_images();
        self.ensure_draft();
        let style = self.span_style();
        if let Some(draft) = self.draft.as_mut() {
            append_span(&mut draft.spans, "\n", &style);
        }
    }

    /// 【贴图】【当前样式】无参数，返回含 pre 代码标记的有效内联样式
    fn span_style(&self) -> SpanStyle {
        let mut style = self.style.clone();
        if self.pre > 0 {
            style.code = true;
        }
        style
    }

    /// 【贴图】【片段写入】按 text 与 style 追加片段，表格单元或块草稿中相同样式与上一片段合并
    fn push_span(&mut self, text: &str, style: &SpanStyle) {
        if text.is_empty() {
            return;
        }
        if self.table_depth > 0 {
            if !self.cell_open() {
                return;
            }
            if let Some(cell) = self.table.as_mut().and_then(|table| table.cell.as_mut()) {
                append_span(&mut cell.spans, text, style);
            }
            return;
        }
        self.ensure_draft();
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        append_span(&mut draft.spans, text, style);
    }

    /// 【贴图】【草稿保证】无参数，按待建块或默认段落创建草稿，并写入列表标记
    fn ensure_draft(&mut self) {
        if self.draft.is_some() {
            return;
        }
        let (kind, align) = match self.pending.take() {
            Some(pending) => (pending.kind, pending.align),
            None => (BlockKind::Paragraph, self.current_align()),
        };
        let align = if self.center > 0 {
            Align::Center
        } else {
            align
        };
        let kind = match kind {
            BlockKind::Paragraph => {
                if let Some(item) = self.list_items.last_mut() {
                    let marker = if item.used {
                        String::new()
                    } else {
                        item.used = true;
                        item.marker.clone()
                    };
                    BlockKind::ListItem {
                        depth: item.depth,
                        marker,
                    }
                } else if self.quotes > 0 {
                    BlockKind::Quote {
                        depth: self.quotes - 1,
                    }
                } else {
                    BlockKind::Paragraph
                }
            }
            other => other,
        };
        let marker = match &kind {
            BlockKind::ListItem { marker, .. } => Some(marker.clone()),
            _ => None,
        };
        self.draft = Some(Draft {
            kind,
            align,
            background: self.block_background,
            spans: Vec::new(),
        });
        self.at_start = true;
        self.space = false;
        if let Some(marker) = marker {
            let style = self.span_style();
            if let Some(draft) = self.draft.as_mut() {
                draft.spans.push(Span {
                    text: marker,
                    style,
                });
            }
        }
    }

    /// 【贴图】【写出块】无参数，先写出待定图片再裁剪尾部空白并把有内容的草稿写入块列表
    fn flush(&mut self) {
        self.flush_images();
        let Some(mut draft) = self.draft.take() else {
            return;
        };
        self.at_start = true;
        self.space = false;
        finalize_spans(&mut draft.spans, matches!(draft.kind, BlockKind::Code));
        if draft.spans.is_empty() {
            return;
        }
        self.blocks.push(Block {
            kind: draft.kind,
            align: draft.align,
            background: draft.background,
            spans: draft.spans,
        });
    }

    /// 【贴图】【写出图片】无参数，把待定图片写成单个图片块或图片行并应用首个图片的对齐
    fn flush_images(&mut self) {
        if self.images.is_empty() {
            return;
        }
        let align = self.image_align.take().unwrap_or(Align::Left);
        let mut images = std::mem::take(&mut self.images);
        let kind = if images.len() == 1 {
            BlockKind::Image(images.remove(0))
        } else {
            BlockKind::ImageRow(images)
        };
        self.blocks.push(Block {
            kind,
            align,
            background: None,
            spans: Vec::new(),
        });
        self.at_start = true;
        self.space = false;
    }

    /// 【贴图】【开始标签】按 tag 更新结构状态与样式栈，块级标签先写出上一块
    fn start_tag(&mut self, tag: &Tag) {
        let name = tag.name.as_str();
        if name == "table" && self.table_depth >= 1 {
            self.table_depth += 1;
            if !tag.self_closing {
                self.push_frame(name, FrameKind::Table);
                self.enter_align_scope(tag);
            }
            return;
        }
        if self.table_depth > 1 {
            if !tag.self_closing {
                self.push_frame(name, FrameKind::Inline);
            }
            return;
        }
        match name {
            "br" => {
                self.newline();
                return;
            }
            "hr" => {
                if self.table_depth > 0 {
                    return;
                }
                self.flush();
                self.blocks.push(Block {
                    kind: BlockKind::Rule,
                    align: Align::Left,
                    background: None,
                    spans: Vec::new(),
                });
                return;
            }
            "img" => {
                self.handle_img(tag);
                return;
            }
            _ => {}
        }
        if self.at_outer_container() && is_outer_container_tag(name) {
            if self.background.is_none()
                && let Some(color) = tag_background(tag)
            {
                self.background = Some(color);
            }
            if self.foreground.is_none()
                && let Some(color) = tag_color(tag)
            {
                self.foreground = Some(color);
            }
        }
        if name == "table" {
            self.start_table(tag);
            return;
        }
        if self.table_depth == 1 {
            match name {
                "tr" => {
                    self.start_row(tag);
                    return;
                }
                "td" | "th" => {
                    self.start_cell(tag);
                    return;
                }
                "thead" | "tbody" | "tfoot" => {
                    if !tag.self_closing {
                        self.push_frame(name, FrameKind::Inline);
                    }
                    return;
                }
                _ => {}
            }
            if self.cell_open() && (is_block(name) || is_heading(name)) {
                self.start_cell_block(tag);
                return;
            }
        }
        if name == "ul" || name == "ol" {
            self.flush();
            self.lists.push(ListContext {
                ordered: name == "ol",
                counter: 0,
            });
            self.push_frame(name, FrameKind::List);
            self.apply_tag_style(tag);
            return;
        }
        if name == "blockquote" {
            self.flush();
            self.quotes += 1;
            self.push_frame(name, FrameKind::Quote);
            self.apply_tag_style(tag);
            return;
        }
        if name == "pre" {
            self.flush();
            self.pre += 1;
            self.strip_pre_newline = true;
            self.pending = Some(Pending {
                kind: BlockKind::Code,
                align: Align::Left,
            });
            self.push_frame(name, FrameKind::Pre);
            self.apply_tag_style(tag);
            return;
        }
        if name == "li" {
            self.flush();
            let item = self.next_list_item();
            self.pending = Some(Pending {
                kind: BlockKind::Paragraph,
                align: self.effective_align(tag),
            });
            self.list_items.push(item);
            self.push_frame(name, FrameKind::ListItem);
            self.apply_tag_style(tag);
            return;
        }
        if name == "center" {
            self.center += 1;
            self.push_frame(name, FrameKind::Center);
            self.apply_tag_style(tag);
            return;
        }
        if let Some(level) = heading_level(name) {
            self.flush();
            self.pending = Some(Pending {
                kind: BlockKind::Heading(level),
                align: self.effective_align(tag),
            });
            self.push_frame(name, FrameKind::Block);
            self.enter_align_scope(tag);
            self.apply_tag_style(tag);
            return;
        }
        if is_block(name) {
            self.flush();
            self.pending = Some(Pending {
                kind: BlockKind::Paragraph,
                align: self.effective_align(tag),
            });
            self.push_frame(name, FrameKind::Block);
            self.enter_align_scope(tag);
            self.apply_tag_style(tag);
            return;
        }
        if tag.self_closing {
            return;
        }
        self.push_frame(name, FrameKind::Inline);
        self.apply_tag_style(tag);
    }

    /// 【贴图】【结束标签】按 name 关闭最近的同名帧，恢复样式与结构，并写出块级草稿
    fn end_tag(&mut self, name: &str) {
        if name == "table" && self.table_depth > 1 {
            self.table_depth -= 1;
        } else if self.table_depth == 1 {
            match name {
                "td" | "th" => self.finish_cell(),
                "tr" => {
                    self.finish_cell();
                    self.finish_row();
                }
                "table" => {
                    self.table_depth = 0;
                    self.finish_table();
                }
                _ => {}
            }
        }
        let Some(index) = self.frames.iter().rposition(|frame| frame.name == name) else {
            return;
        };
        while self.frames.len() > index {
            let Some(frame) = self.frames.pop() else {
                break;
            };
            match frame.kind {
                FrameKind::List => {
                    self.lists.pop();
                }
                FrameKind::ListItem => {
                    self.list_items.pop();
                }
                FrameKind::Quote => self.quotes = self.quotes.saturating_sub(1),
                FrameKind::Pre => self.pre = self.pre.saturating_sub(1),
                FrameKind::Center => self.center = self.center.saturating_sub(1),
                _ => {}
            }
            self.style = frame.saved;
            self.block_background = frame.saved_background;
            if frame.align_pushed {
                self.align_scope.pop();
            }
        }
        if self.table_depth == 0
            && (is_block(name)
                || is_heading(name)
                || matches!(name, "li" | "tr" | "ul" | "ol" | "blockquote" | "pre"))
        {
            self.flush();
        }
    }

    /// 【贴图】【草稿文本】无参数，返回当前草稿是否已含非空白文本
    fn draft_has_text(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| {
            draft
                .spans
                .iter()
                .any(|span| span.text.chars().any(|ch| !ch.is_whitespace()))
        })
    }

    /// 【贴图】【帧入栈】按 name 与 kind 保存当前样式与容器背景，供结束标签恢复
    fn push_frame(&mut self, name: &str, kind: FrameKind) {
        self.frames.push(Frame {
            name: name.into(),
            saved: self.style.clone(),
            saved_background: self.block_background,
            kind,
            align_pushed: false,
        });
    }

    /// 【贴图】【外层容器判断】无参数，返回是否仍位于任何块内容之前且尚未进入引用或列表
    fn at_outer_container(&self) -> bool {
        self.blocks.is_empty()
            && self.draft.is_none()
            && self.quotes == 0
            && self.list_items.is_empty()
    }

    /// 【贴图】【列表项】无参数，按当前列表类型生成标记并推进有序编号，返回列表项状态
    fn next_list_item(&mut self) -> ListMarker {
        let depth = self.lists.len().saturating_sub(1);
        let marker = match self.lists.last_mut() {
            Some(list) if list.ordered => {
                list.counter += 1;
                format!("{}. ", list.counter)
            }
            _ => "• ".to_string(),
        };
        ListMarker {
            depth,
            marker,
            used: false,
        }
    }

    /// 【贴图】【样式应用】按标签语义与 style 属性修改当前内联样式，字号以当前字号为基准
    fn apply_tag_style(&mut self, tag: &Tag) {
        let base = self.style.size.unwrap_or(16.0);
        self.block_tag = is_block_background_tag(&tag.name);
        match tag.name.as_str() {
            "b" | "strong" => self.style.bold = true,
            "i" | "em" => self.style.italic = true,
            "u" | "ins" => self.style.underline = true,
            "s" | "strike" | "del" => self.style.strike = true,
            "code" | "kbd" | "samp" | "tt" => self.style.code = true,
            "mark" => self.style.background = Some(Rgba(255, 235, 59, 90)),
            "small" => self.style.size = Some((base * 0.85).clamp(6.0, 96.0)),
            "sub" | "sup" => self.style.size = Some((base * 0.75).clamp(6.0, 96.0)),
            "a" if tag.attr("href").is_some() => self.style.link = true,
            "font" => {
                if let Some(color) = tag.attr("color").and_then(html_style::color) {
                    self.style.color = Some(color);
                }
                if tag.attr("face").is_some_and(html_style::monospace) {
                    self.style.code = true;
                }
            }
            _ => {}
        }
        if let Some(css) = tag.attr("style") {
            for (property, value) in html_style::declarations(css) {
                self.apply_declaration(property, value, base);
            }
        }
    }

    /// 【贴图】【声明应用】按 property 与 value 应用单条受支持的内联声明，未知属性忽略
    fn apply_declaration(&mut self, property: &str, value: &str, base: f32) {
        if property.eq_ignore_ascii_case("color") {
            if let Some(color) = html_style::color(value) {
                self.style.color = Some(color);
            }
        } else if property.eq_ignore_ascii_case("background-color")
            || property.eq_ignore_ascii_case("background")
        {
            if let Some(color) = html_style::color(value) {
                if self.block_tag {
                    self.block_background = Some(color);
                } else {
                    self.style.background = Some(color);
                }
            }
        } else if property.eq_ignore_ascii_case("font-weight") {
            if let Some(bold) = html_style::bold(value) {
                self.style.bold = bold;
            }
        } else if property.eq_ignore_ascii_case("font-style") {
            if let Some(italic) = html_style::italic(value) {
                self.style.italic = italic;
            }
        } else if property.eq_ignore_ascii_case("font-size") {
            if let Some(size) = html_style::font_size(value, base) {
                self.style.size = Some(size);
            }
        } else if property.eq_ignore_ascii_case("line-height") {
            if let Some(multiplier) = html_style::line_height(value, base) {
                self.style.line_height = Some(multiplier);
            }
        } else if property.eq_ignore_ascii_case("font-family") {
            if html_style::monospace(value) {
                self.style.code = true;
            } else {
                self.style.serif = html_style::serif(value);
            }
        } else if property.eq_ignore_ascii_case("text-decoration")
            || property.eq_ignore_ascii_case("text-decoration-line")
        {
            for word in value.split_ascii_whitespace() {
                if word.eq_ignore_ascii_case("underline") {
                    self.style.underline = true;
                } else if word.eq_ignore_ascii_case("line-through") {
                    self.style.strike = true;
                } else if word.eq_ignore_ascii_case("overline") {
                    self.style.overline = true;
                } else if word.eq_ignore_ascii_case("none") {
                    self.style.underline = false;
                    self.style.strike = false;
                    self.style.overline = false;
                }
            }
        }
    }
}
