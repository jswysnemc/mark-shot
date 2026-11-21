//! 【RTF解析】【主解析】扫描 RTF 记号，维护分组格式栈并构建段落、字段、表格与代码块

use super::super::html::{Align, Block, BlockKind, RichText, Span, SpanStyle};
use super::super::spans;
use super::code::merge_code_blocks;
use super::colors::{ColorTable, Component};
use super::encoding;
use super::fonts::FontTable;
use super::lexer::{Lexer, Token};
use super::table::TableBuilder;
use crate::theme::Rgba;

/// 【RTF解析】【字符状态】记录当前片段的样式、可见性、Unicode 回退计数、代码页与字体索引
#[derive(Clone)]
struct Format {
    style: SpanStyle,
    visible: bool,
    uc: usize,
    codepage: u16,
    font: i32,
}

impl Default for Format {
    /// 【RTF解析】【默认字符状态】无参数，返回无色、可见、uc 为 1 且代码页 1252 的初始状态
    fn default() -> Self {
        Self {
            style: SpanStyle::default(),
            visible: true,
            uc: 1,
            codepage: 1252,
            font: 0,
        }
    }
}

/// 【RTF解析】【段落状态】记录对齐、列表深度、项目符号与段落背景
#[derive(Clone, Copy)]
struct Paragraph {
    align: Align,
    depth: usize,
    bullet: bool,
    background: Option<Rgba>,
    spacing: i32,
    multiple: bool,
}

impl Default for Paragraph {
    /// 【RTF解析】【默认段落】无参数，返回左对齐、无列表与无背景的段落状态
    fn default() -> Self {
        Self {
            align: Align::Left,
            depth: 0,
            bullet: false,
            background: None,
            spacing: 0,
            multiple: false,
        }
    }
}

/// 【RTF解析】【分组模式】描述当前分组正在解析的目标结构
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Skip,
    FontTable,
    ColorTable,
    FieldInstruction,
}

/// 【RTF解析】【分组帧】保存进入分组时的字符状态、目标模式与字段信息
struct Frame {
    fmt: Format,
    para: Paragraph,
    mode: Mode,
    skip: bool,
    field: bool,
    link: bool,
    capture: bool,
}

/// 【RTF解析】【解析器】聚合格式栈、资源表、段落草稿与表格构建状态
pub(super) struct Parser {
    fmt: Format,
    frames: Vec<Frame>,
    para: Paragraph,
    fonts: FontTable,
    colors: ColorTable,
    font_def: Option<(i32, String)>,
    capture: String,
    fallback: usize,
    pending_high: Option<u32>,
    pending_bytes: Vec<u8>,
    blocks: Vec<Block>,
    draft: Vec<Span>,
    draft_para: Option<Paragraph>,
    background: Option<Rgba>,
    first_paragraph_done: bool,
    table: Option<TableBuilder>,
}

impl Parser {
    /// 【RTF解析】【新建解析器】无参数，返回带默认字符与段落状态的解析器
    pub(super) fn new() -> Self {
        Self {
            fmt: Format::default(),
            frames: Vec::new(),
            para: Paragraph::default(),
            fonts: FontTable::new(),
            colors: ColorTable::new(),
            font_def: None,
            capture: String::new(),
            fallback: 0,
            pending_high: None,
            pending_bytes: Vec::new(),
            blocks: Vec::new(),
            draft: Vec::new(),
            draft_para: None,
            background: None,
            first_paragraph_done: false,
            table: None,
        }
    }

    /// 【RTF解析】【解析入口】按 text 扫描全部记号并返回富文本；任意输入都不会 panic
    pub(super) fn parse(mut self, text: &[u8]) -> RichText {
        let mut lexer = Lexer::new(text);
        while let Some(token) = lexer.next() {
            match token {
                Token::Start => self.open_group(),
                Token::End => self.close_group(),
                Token::Control(word, param) => self.control(&mut lexer, word, param),
                Token::Symbol(symbol) => self.symbol(symbol),
                Token::Hex(byte) => self.hex(byte),
                Token::Text(bytes) => self.text(bytes),
            }
        }
        self.finish()
    }

    /// 【RTF解析】【收尾】无参数，刷新残余字节、表格与段落并合并代码块后返回富文本
    fn finish(mut self) -> RichText {
        self.flush_hex();
        self.pending_high = None;
        self.finish_font();
        self.colors.finish();
        if let Some(mut table) = self.table.take()
            && let Some(block) = table.take_block()
        {
            self.push_block(block);
        }
        self.flush_paragraph();
        let blocks = merge_code_blocks(std::mem::take(&mut self.blocks));
        RichText {
            blocks,
            background: self.background,
            foreground: None,
        }
    }

    /// 【RTF解析】【进入分组】无参数，先刷新字节再压入当前格式与继承的目标模式
    fn open_group(&mut self) {
        self.flush_hex();
        self.pending_high = None;
        self.fallback = 0;
        let (mode, skip) = self
            .frames
            .last()
            .map_or((Mode::Normal, false), |frame| (frame.mode, frame.skip));
        self.frames.push(Frame {
            fmt: self.fmt.clone(),
            para: self.para,
            mode,
            skip,
            field: false,
            link: false,
            capture: false,
        });
    }

    /// 【RTF解析】【结束分组】无参数，刷新字节、结算字段与资源表并恢复字符格式
    fn close_group(&mut self) {
        self.flush_hex();
        self.pending_high = None;
        self.fallback = 0;
        let Some(frame) = self.frames.pop() else {
            return;
        };
        if frame.capture {
            let link = self.capture.to_ascii_lowercase().contains("hyperlink");
            self.capture.clear();
            if let Some(field) = self.frames.iter_mut().rev().find(|frame| frame.field) {
                field.link = link;
            }
        }
        if frame.mode == Mode::FontTable {
            self.finish_font();
        }
        if frame.mode == Mode::ColorTable {
            self.colors.finish();
        }
        self.fmt = frame.fmt;
        self.para = frame.para;
    }

    /// 【RTF解析】【有效模式】无参数，返回当前分组的目标模式，忽略分组直接视为跳过
    fn mode(&self) -> Mode {
        match self.frames.last() {
            Some(frame) if frame.skip => Mode::Skip,
            Some(frame) => frame.mode,
            None => Mode::Normal,
        }
    }

    /// 【RTF解析】【跳过判断】无参数，返回当前分组是否处于忽略目标
    fn skipping(&self) -> bool {
        self.frames
            .last()
            .is_some_and(|frame| frame.skip || frame.mode == Mode::Skip)
    }

    /// 【RTF解析】【设置模式】按 mode 修改当前分组目标模式，无分组时忽略
    fn set_mode(&mut self, mode: Mode) {
        if let Some(frame) = self.frames.last_mut() {
            frame.mode = mode;
        }
    }

    /// 【RTF解析】【控制字】按 word 与 param 处理字段、跳过目标及普通控制字
    fn control(&mut self, lexer: &mut Lexer<'_>, word: &[u8], param: Option<i32>) {
        let word = std::str::from_utf8(word).unwrap_or_default();
        if word == "bin" {
            self.flush_hex();
            if let Some(count) = param.filter(|count| *count > 0) {
                lexer.skip(count as usize);
            }
            return;
        }
        if word == "fldinst" {
            self.flush_hex();
            self.fallback = 0;
            self.pending_high = None;
            self.capture.clear();
            if let Some(frame) = self.frames.last_mut() {
                frame.mode = Mode::FieldInstruction;
                frame.skip = false;
                frame.capture = true;
            }
            return;
        }
        if word == "fldrslt" {
            self.flush_hex();
            self.fallback = 0;
            self.pending_high = None;
            let link = self
                .frames
                .iter()
                .rev()
                .find(|frame| frame.field)
                .is_some_and(|frame| frame.link);
            if let Some(frame) = self.frames.last_mut() {
                frame.mode = Mode::Normal;
                frame.skip = false;
            }
            self.fmt.style.link = link;
            return;
        }
        if self.skipping() {
            self.pending_bytes.clear();
            return;
        }
        self.flush_hex();
        self.fallback = 0;
        if word != "u" {
            self.pending_high = None;
        }
        match self.mode() {
            Mode::FontTable => self.font_control(word, param),
            Mode::ColorTable => self.color_control(word, param),
            Mode::FieldInstruction => self.capture_control(word, param),
            Mode::Normal | Mode::Skip => self.normal_control(word, param),
        }
    }

    /// 【RTF解析】【普通控制字】按 word 与 param 应用字符、段落、表格或字段格式
    fn normal_control(&mut self, word: &str, param: Option<i32>) {
        match word {
            "fonttbl" => self.set_mode(Mode::FontTable),
            "colortbl" => self.set_mode(Mode::ColorTable),
            "stylesheet" | "info" | "pict" | "object" | "header" | "footer" | "headerl"
            | "headerr" | "headerf" | "footerl" | "footerr" | "footerf" | "footnote"
            | "annotation" | "themedata" | "colorschememapping" | "latentstyles" | "datastore"
            | "xmlnstbl" | "rsidtbl" | "generator" | "listtable" | "listoverridetable"
            | "mmath" | "docvar" | "xe" | "tc" | "bkmkstart" | "bkmkend" | "pntext"
            | "listtext" | "filetbl" | "revtbl" => self.set_mode(Mode::Skip),
            "b" => self.fmt.style.bold = flag(param),
            "i" => self.fmt.style.italic = flag(param),
            "ul" => self.fmt.style.underline = flag(param),
            "ulnone" => self.fmt.style.underline = false,
            "strike" => self.fmt.style.strike = flag(param),
            "striked" => self.fmt.style.strike = true,
            "outl" => self.fmt.style.overline = flag(param),
            "fs" => {
                if let Some(value) = param {
                    let size = value as f32 * (2.0 / 3.0);
                    if size.is_finite() {
                        self.fmt.style.size = Some(size.clamp(6.0, 96.0));
                    }
                }
            }
            "cf" => {
                if let Some(value) = param {
                    self.fmt.style.color = self.colors.get(value);
                }
            }
            "highlight" => {
                if let Some(value) = param {
                    self.fmt.style.background = self.colors.get(value);
                }
            }
            "cb" | "chcbpat" => {
                if let Some(value) = param {
                    self.para.background = self.colors.get(value);
                }
            }
            "f" => {
                if let Some(value) = param {
                    self.fmt.font = value;
                    let info = self.fonts.get(value);
                    self.fmt.style.code = info.is_some_and(|info| info.code);
                    self.fmt.style.serif = info.is_some_and(|info| info.serif);
                }
            }
            "plain" => {
                self.fmt.style = SpanStyle::default();
                self.fmt.font = 0;
            }
            "ansicpg" => {
                if let Some(value) = param {
                    self.fmt.codepage = value.clamp(0, i32::from(u16::MAX)) as u16;
                }
            }
            "uc" => {
                if let Some(value) = param {
                    self.fmt.uc = value.clamp(0, 16) as usize;
                }
            }
            "u" => self.unicode(param),
            "v" => self.fmt.visible = param == Some(0),
            "par" => self.paragraph_break(),
            "line" => self.line_break(),
            "tab" => self.emit_text("\t"),
            "pard" => self.paragraph_reset(),
            "ql" | "qj" => self.para.align = Align::Left,
            "qc" => self.para.align = Align::Center,
            "qr" => self.para.align = Align::Right,
            "sl" => self.para.spacing = param.unwrap_or(0),
            "slmult" => self.para.multiple = param == Some(1),
            "li" => {
                if let Some(value) = param {
                    self.para.depth = (value.max(0) as usize / 720).min(8);
                }
            }
            "bullet" | "pn" => self.para.bullet = true,
            "trowd" => self.start_row(),
            "trhdr" => {
                if let Some(table) = self.table.as_mut() {
                    table.mark_header();
                }
            }
            "intbl" => {
                self.ensure_table();
                if let Some(table) = self.table.as_mut() {
                    table.mark_intbl();
                }
            }
            "cell" => {
                if let Some(table) = self.table.as_mut() {
                    table.finish_cell();
                }
            }
            "row" => {
                if let Some(table) = self.table.as_mut() {
                    table.finish_row();
                }
            }
            "clcbpat" => {
                if let Some(value) = param {
                    let color = self.colors.get(value);
                    if let Some(table) = self.table.as_mut() {
                        table.cell_background(color);
                    }
                }
            }
            "field" => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.field = true;
                }
            }
            _ => {}
        }
    }

    /// 【RTF解析】【字体控制字】按 word 与 param 解析 fonttbl 中的字体定义开始
    fn font_control(&mut self, word: &str, param: Option<i32>) {
        if word == "f"
            && let Some(index) = param
        {
            self.finish_font();
            self.font_def = Some((index, String::new()));
        }
    }

    /// 【RTF解析】【颜色控制字】按 word 与 param 写入 colortbl 的红绿蓝分量
    fn color_control(&mut self, word: &str, param: Option<i32>) {
        let Some(value) = param else {
            return;
        };
        match word {
            "red" => self.colors.set(Component::Red, value),
            "green" => self.colors.set(Component::Green, value),
            "blue" => self.colors.set(Component::Blue, value),
            _ => {}
        }
    }

    /// 【RTF解析】【字段指令】按 word 与 param 追加字段指令文本，供 HYPERLINK 检测
    fn capture_control(&mut self, word: &str, param: Option<i32>) {
        self.capture.push(' ');
        self.capture.push_str(word);
        if let Some(value) = param {
            self.capture.push_str(&value.to_string());
        }
        self.capture.push(' ');
    }

    /// 【RTF解析】【字体收尾】无参数，把当前累计字体名登记到字体表
    fn finish_font(&mut self) {
        if let Some((index, name)) = self.font_def.take() {
            self.fonts.define(index, &name);
        }
    }

    /// 【RTF解析】【控制符号】按 byte 处理转义字符、不可见标记与可忽略目标标记
    fn symbol(&mut self, byte: u8) {
        if self.fallback > 0 {
            self.fallback -= 1;
            return;
        }
        if matches!(byte, b'\\' | b'{' | b'}') {
            self.pending_high = None;
            if self.mode() != Mode::Skip {
                self.pending_bytes.push(byte);
            }
            return;
        }
        self.flush_hex();
        self.pending_high = None;
        if self.mode() == Mode::Skip {
            return;
        }
        match byte {
            b'*' => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.skip = true;
                }
            }
            b'~' => self.emit_text("\u{00a0}"),
            b'_' => self.emit_text("-"),
            b' ' => self.emit_text(" "),
            _ => {}
        }
    }

    /// 【RTF解析】【十六进制字节】按 byte 累加到待解码缓冲；回退字符时直接消耗
    fn hex(&mut self, byte: u8) {
        if self.fallback > 0 {
            self.fallback -= 1;
            return;
        }
        self.pending_high = None;
        if self.mode() == Mode::Skip {
            return;
        }
        self.pending_bytes.push(byte);
        if self.pending_bytes.len() >= 65_536 {
            self.flush_hex();
        }
    }

    /// 【RTF解析】【纯文本】按 bytes 消耗 Unicode 回退字节后与十六进制字节共同解码，无返回值，保留跨记号的多字节字符
    fn text(&mut self, bytes: &[u8]) {
        let skip = self.fallback.min(bytes.len());
        self.fallback -= skip;
        if skip == bytes.len() {
            return;
        }
        self.pending_high = None;
        if self.mode() != Mode::Skip {
            self.pending_bytes.extend_from_slice(&bytes[skip..]);
        }
    }

    /// 【RTF解析】【Unicode 转义】按 param 插入字符，处理负值、代理对与回退字符计数
    fn unicode(&mut self, param: Option<i32>) {
        let Some(value) = param else {
            return;
        };
        let mut code = value;
        if code < 0 {
            code += 65_536;
        }
        if code <= 0 || code > 0x10_FFFF {
            return;
        }
        let code = code as u32;
        self.fallback = self.fmt.uc;
        if !self.fmt.visible {
            self.pending_high = None;
            return;
        }
        if (0xD800..=0xDBFF).contains(&code) {
            self.pending_high = Some(code);
            return;
        }
        if (0xDC00..=0xDFFF).contains(&code) {
            if let Some(high) = self.pending_high.take() {
                let combined = 0x1_0000 + ((high - 0xD800) << 10) + (code - 0xDC00);
                if let Some(character) = char::from_u32(combined) {
                    self.emit_text(&character.to_string());
                }
            }
            return;
        }
        self.pending_high = None;
        if let Some(character) = char::from_u32(code) {
            self.emit_text(&character.to_string());
        }
    }

    /// 【RTF解析】【追加文本】按当前分组模式把 text 写入正常片段、资源表或字段指令
    fn emit_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.mode() {
            Mode::Normal => self.normal_text(text),
            Mode::FontTable => self.font_text(text),
            Mode::ColorTable => self.color_text(text),
            Mode::FieldInstruction => self.capture.push_str(text),
            Mode::Skip => {}
        }
    }

    /// 【RTF解析】【正文文本】按 text 追加到当前表格单元格或段落草稿，隐藏文本被丢弃
    fn normal_text(&mut self, text: &str) {
        if !self.fmt.visible || text.is_empty() {
            return;
        }
        let mut style = self.fmt.style.clone();
        if self.para.spacing != 0 {
            let height = if self.para.multiple {
                self.para.spacing.unsigned_abs() as f32 / 240.0
            } else {
                self.para.spacing.unsigned_abs() as f32 / 15.0 / style.size.unwrap_or(16.0)
            };
            let height = if !self.para.multiple && self.para.spacing > 0 {
                height.max(crate::theme::font::line_height())
            } else {
                height
            };
            style.line_height = Some(height.clamp(0.1, 20.0));
        }
        if let Some(table) = self.table.as_mut()
            && table.accepts_text()
        {
            table.append_span(
                Span {
                    text: text.into(),
                    style,
                },
                self.para.align,
            );
            return;
        }
        self.draft_para = Some(self.para);
        spans::append_span(&mut self.draft, text, &style);
    }

    /// 【RTF解析】【字体文本】按 text 累加当前字体名，遇到分号时登记字体
    fn font_text(&mut self, text: &str) {
        let mut rest = text;
        while let Some(index) = rest.find(';') {
            if let Some((_, name)) = self.font_def.as_mut() {
                name.push_str(&rest[..index]);
            }
            self.finish_font();
            rest = &rest[index + 1..];
        }
        if !rest.is_empty()
            && let Some((_, name)) = self.font_def.as_mut()
        {
            name.push_str(rest);
        }
    }

    /// 【RTF解析】【颜色文本】按 text 中的分号结束当前颜色条目，其余字符忽略
    fn color_text(&mut self, text: &str) {
        let mut rest = text;
        while let Some(index) = rest.find(';') {
            self.colors.semicolon();
            rest = &rest[index + 1..];
        }
    }

    /// 【RTF解析】【刷新字节】无参数，按当前代码页把累计的 `\'hh` 字节解码为文本
    fn flush_hex(&mut self) {
        if self.pending_bytes.is_empty() {
            return;
        }
        let bytes = std::mem::take(&mut self.pending_bytes);
        let text = encoding::decode(self.fmt.codepage, &bytes);
        self.emit_text(&text);
    }

    /// 【RTF解析】【段落换行】无参数，表格内追加单元格换行，否则写出当前段落
    fn paragraph_break(&mut self) {
        if let Some(table) = self.table.as_mut()
            && table.accepts_text()
        {
            table.newline(&self.fmt.style, self.para.align);
            return;
        }
        self.flush_paragraph();
    }

    /// 【RTF解析】【行内换行】无参数，在当前段落或单元格中插入换行符
    fn line_break(&mut self) {
        self.emit_text("\n");
    }

    /// 【RTF解析】【重置段落】无参数，表格活动时仅重置段落，否则结束表格并恢复默认段落属性
    fn paragraph_reset(&mut self) {
        if self.table.as_ref().is_some_and(TableBuilder::active) {
            self.flush_paragraph();
        } else {
            let block = self.table.as_mut().and_then(TableBuilder::take_block);
            if let Some(block) = block {
                self.push_block(block);
            }
            self.table = None;
            self.flush_paragraph();
        }
        self.para = Paragraph::default();
    }

    /// 【RTF解析】【开始表格行】无参数，写出前置段落并创建或推进表格行
    fn start_row(&mut self) {
        self.flush_paragraph();
        self.ensure_table();
        if let Some(table) = self.table.as_mut() {
            table.start_row();
        }
    }

    /// 【RTF解析】【保证表格】无参数，若尚未创建表格则补建构建器
    fn ensure_table(&mut self) {
        if self.table.is_none() {
            self.table = Some(TableBuilder::new());
        }
    }

    /// 【RTF解析】【写出段落】无参数，裁剪空白、补写列表标记并生成普通或列表块
    fn flush_paragraph(&mut self) {
        let mut spans = std::mem::take(&mut self.draft);
        let para = self.draft_para.take().unwrap_or(self.para);
        spans::trim_spans(&mut spans);
        if spans.is_empty() {
            return;
        }
        let kind = if para.bullet {
            spans.insert(
                0,
                Span {
                    text: "• ".into(),
                    style: self.fmt.style.clone(),
                },
            );
            BlockKind::ListItem {
                depth: para.depth,
                marker: "• ".into(),
            }
        } else {
            BlockKind::Paragraph
        };
        let block = Block {
            kind,
            align: para.align,
            background: para.background,
            spans,
        };
        self.push_block(block);
    }

    /// 【RTF解析】【写出块】按 block 追加到结果，并记录首个正文段落的画布背景
    fn push_block(&mut self, block: Block) {
        if !matches!(block.kind, BlockKind::Table { .. }) && !self.first_paragraph_done {
            self.first_paragraph_done = true;
            if let Some(background) = block.background {
                self.background = Some(background);
            }
        }
        self.blocks.push(block);
    }
}

/// 【RTF解析】【布尔参数】按 param 解析开关，缺省或非零为真，零为假
fn flag(param: Option<i32>) -> bool {
    param.is_none_or(|value| value != 0)
}
