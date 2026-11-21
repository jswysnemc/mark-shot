//! 【贴图】【富文本绘制】使用 cosmic-text 排版解析后的块，并通过现有绘制器输出 PNG

use super::IMAGE_LIMIT;
use super::html::{Align, Block, BlockKind, RichText, Span, TableRow};
use crate::renderer::Painter;
use crate::theme::Rgba;
use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, Style, SwashCache,
    UnderlineStyle, Weight,
};
use tiny_skia::{Color, Pixmap, Rect};

mod picture;
mod table;

const PADDING: f32 = 16.0;
const MAX_CONTENT: f32 = 720.0;
const MAX_CODE_CONTENT: f32 = 1600.0;
const MIN_CONTENT: f32 = 120.0;
const CODE_PAD: f32 = 6.0;
const LIST_INDENT: f32 = 20.0;
const QUOTE_INDENT: f32 = 14.0;
const QUOTE_GAP: f32 = 10.0;
const BODY_SIZE: f32 = 16.0;
const CODE_SIZE: f32 = 14.0;

/// 【贴图】【富文本图像】按 rich 排版并绘制段落、标题、列表、引用与代码，返回 PNG 或明确错误
pub(super) fn image(rich: &RichText) -> crate::AppResult<Vec<u8>> {
    if !rich.has_content() {
        return Err("the clipboard rich text has no visible content".into());
    }
    let mut fonts = crate::fonts::bundled();
    crate::fonts::add_system(&mut fonts, crate::fonts::system());
    let mut cache = SwashCache::new();
    let palette = Palette::new(rich);
    let mut backgrounds = vec![None];
    let prepared = prepare(rich, &palette, &mut backgrounds);

    // 1. 【贴图】【固有宽度】分别测量正文、代码与表格宽度，再决定不把整张画布压回 720
    let mut prose_intrinsic = MIN_CONTENT;
    let mut code_intrinsic = MIN_CONTENT;
    let mut table_intrinsic = MIN_CONTENT;
    let mut buffers: Vec<Option<Buffer>> = Vec::with_capacity(prepared.len());
    for block in &prepared {
        if matches!(block.kind, BlockKind::Rule) {
            buffers.push(None);
            continue;
        }
        if picture::is_picture(&block.kind) {
            prose_intrinsic = prose_intrinsic.max(picture::measure(&block.kind).min(MAX_CONTENT));
            buffers.push(None);
            continue;
        }
        if let Some(data) = &block.table {
            table_intrinsic = table_intrinsic.max(table::intrinsic_width(&mut fonts, data));
            buffers.push(None);
            continue;
        }
        let buffer = build_buffer(&mut fonts, block, None);
        let width = block.indent + max_line_width(&buffer);
        if matches!(block.kind, BlockKind::Code) {
            code_intrinsic = code_intrinsic.max(width);
        } else {
            prose_intrinsic = prose_intrinsic.max(width.min(MAX_CONTENT));
        }
        buffers.push(Some(buffer));
    }
    let content_width = prose_intrinsic
        .clamp(MIN_CONTENT, MAX_CONTENT)
        .max(code_intrinsic.clamp(MIN_CONTENT, MAX_CODE_CONTENT))
        .max(table_intrinsic.clamp(MIN_CONTENT, MAX_CONTENT));

    // 2. 【贴图】【换行重排】正文按 720 换行，代码保留到 1600，表格在自身列宽内换行
    let prose_width = content_width.min(MAX_CONTENT);
    for (block, buffer) in prepared.iter().zip(buffers.iter_mut()) {
        if let Some(buffer) = buffer {
            let limit = if matches!(block.kind, BlockKind::Code) {
                content_width
            } else {
                prose_width
            };
            let available = (limit - block.indent).max(1.0);
            buffer.set_size(Some(available), None);
            buffer.shape_until_scroll(&mut fonts, false);
        }
    }

    // 3. 【贴图】【块定位】累计行高、块间距、表格高度与代码内边距，得到最终画布高度
    let mut laid = Vec::with_capacity(buffers.len());
    let mut cursor = PADDING;
    let count = buffers.len();
    for (index, (block, buffer)) in prepared.into_iter().zip(buffers).enumerate() {
        let PreparedBlock {
            kind,
            align,
            indent,
            background,
            table,
            ..
        } = block;
        let gap = block_gap(&kind);
        if picture::is_picture(&kind) {
            let layout = picture::layout(&kind, align, prose_width, cursor);
            let height = layout.height();
            laid.push(LaidBlock {
                kind,
                buffer: None,
                x: PADDING,
                y: cursor,
                height,
                extra: 0.0,
                background,
                table: None,
                picture: Some(layout),
            });
            cursor += height;
        } else if matches!(kind, BlockKind::Rule) {
            laid.push(LaidBlock {
                kind,
                buffer: None,
                x: PADDING,
                y: cursor,
                height: 1.0,
                extra: 0.0,
                background: None,
                table: None,
                picture: None,
            });
            cursor += 1.0;
        } else if let Some(data) = table {
            let laid_table = table::layout(&mut fonts, &data, prose_width);
            let height = laid_table.height;
            laid.push(LaidBlock {
                kind,
                buffer: None,
                x: PADDING,
                y: cursor,
                height,
                extra: 0.0,
                background,
                table: Some(laid_table),
                picture: None,
            });
            cursor += height;
        } else {
            let extra = if matches!(kind, BlockKind::Code) {
                CODE_PAD
            } else {
                0.0
            };
            let height = buffer.as_ref().map_or(0.0, block_height);
            let text_y = cursor + extra;
            laid.push(LaidBlock {
                kind,
                buffer,
                x: PADDING + indent,
                y: text_y,
                height,
                extra,
                background,
                table: None,
                picture: None,
            });
            cursor = text_y + height + extra;
        }
        if index + 1 < count {
            cursor += gap;
        }
    }

    let width = (content_width + PADDING * 2.0).ceil() as u32;
    let height = (cursor + PADDING).ceil() as u32;
    if width == 0 || height == 0 {
        return Err("the clipboard rich text has no drawable size".into());
    }
    if u64::from(width) * u64::from(height) * 4 > IMAGE_LIMIT {
        return Err("clipboard rich text image exceeds the pin size limit".into());
    }
    let mut image =
        Pixmap::new(width, height).ok_or("cannot allocate clipboard rich text image")?;
    image.fill(palette.canvas.color());

    // 4. 【贴图】【绘制输出】先画块背景与片段底色，再绘制字形与装饰线
    {
        let layout = Layout {
            content_width,
            canvas: palette.canvas,
        };
        let mut painter = Painter::new(&mut image).with_text(&mut fonts, &mut cache);
        for block in &laid {
            draw_block_background(&mut painter, block, &layout);
            if picture::is_picture(&block.kind) {
                if let Some(picture) = &block.picture {
                    picture::draw(&mut painter, &block.kind, picture, &palette);
                }
                continue;
            }
            if let Some(data) = &block.table {
                table::draw(
                    &mut painter,
                    data,
                    (block.x, block.y),
                    &palette,
                    &backgrounds,
                );
                continue;
            }
            let Some(buffer) = &block.buffer else {
                continue;
            };
            draw_span_backgrounds(&mut painter, buffer, (block.x, block.y), &backgrounds);
            painter.text_buffer(buffer, (block.x, block.y), palette.foreground.color());
            draw_decorations(
                &mut painter,
                buffer,
                (block.x, block.y),
                palette.foreground.cosmic(),
            );
        }
    }
    Ok(crate::utils::encode_png_with_alpha(image))
}

/// 【贴图】【调色板】记录画布背景、默认文字与内联代码底色，按画布明暗预先计算
struct Palette {
    canvas: Rgba,
    foreground: Rgba,
    inline_code: Rgba,
}

impl Palette {
    /// 【贴图】【调色板构建】按 rich 的外层容器背景与前景决定画布色，缺省回退主题
    fn new(rich: &RichText) -> Self {
        let canvas = rich
            .background
            .unwrap_or(crate::config::get().theme.background);
        let foreground = rich
            .foreground
            .unwrap_or_else(crate::theme::color::foreground);
        Self {
            canvas,
            foreground,
            inline_code: inline_code_background(canvas, foreground),
        }
    }
}

/// 【贴图】【预处理片段】记录块类型、缩进、块背景、带属性片段及表格数据
struct PreparedBlock {
    kind: BlockKind,
    align: Align,
    indent: f32,
    background: Option<Rgba>,
    spans: Vec<(String, Attrs<'static>)>,
    table: Option<table::Table>,
}

/// 【贴图】【已定位块】记录块在画布中的位置、缓冲区、高度、背景与已排表格
struct LaidBlock {
    kind: BlockKind,
    buffer: Option<Buffer>,
    x: f32,
    y: f32,
    height: f32,
    extra: f32,
    background: Option<Rgba>,
    table: Option<table::LaidTable>,
    picture: Option<picture::Layout>,
}

/// 【贴图】【块预处理】按 rich 转换每个片段为宇宙文本属性，并登记片段底色索引
fn prepare(
    rich: &RichText,
    palette: &Palette,
    backgrounds: &mut Vec<Option<Rgba>>,
) -> Vec<PreparedBlock> {
    let line_height = crate::theme::font::line_height();
    rich.blocks
        .iter()
        .map(|block| {
            let mut table = None;
            let kind = match &block.kind {
                BlockKind::Table { rows, style } => {
                    table = Some(prepare_table(
                        rows,
                        style,
                        palette,
                        line_height,
                        backgrounds,
                    ));
                    BlockKind::Table {
                        rows: Vec::new(),
                        style: Default::default(),
                    }
                }
                _ => block.kind.clone(),
            };
            let context = SpanContext {
                base: base_size(block),
                heading: matches!(block.kind, BlockKind::Heading(_)),
                quote: matches!(block.kind, BlockKind::Quote { .. }),
                code_block: matches!(block.kind, BlockKind::Code),
                line_height,
                foreground: palette.foreground,
                canvas: palette.canvas,
                inline_code: palette.inline_code,
            };
            let spans = if table.is_some() {
                Vec::new()
            } else {
                block
                    .spans
                    .iter()
                    .map(|span| (span.text.clone(), span_attrs(span, &context, backgrounds)))
                    .collect()
            };
            PreparedBlock {
                kind,
                align: block.align,
                indent: block_indent(block),
                background: block.background,
                spans,
                table,
            }
        })
        .collect()
}

/// 【贴图】【表格预处理】按行与单元格把片段转换为表格模块所需的属性结构
fn prepare_table(
    rows: &[TableRow],
    style: &super::html::TableStyle,
    palette: &Palette,
    line_height: f32,
    backgrounds: &mut Vec<Option<Rgba>>,
) -> table::Table {
    let context = SpanContext {
        base: BODY_SIZE,
        heading: false,
        quote: false,
        code_block: false,
        line_height,
        foreground: palette.foreground,
        canvas: palette.canvas,
        inline_code: palette.inline_code,
    };
    let rows = rows
        .iter()
        .map(|row| table::Row {
            header: row.header,
            background: row.background,
            cells: row
                .cells
                .iter()
                .map(|cell| table::Cell {
                    header: cell.header,
                    align: cell.align,
                    background: cell.background,
                    spans: cell
                        .spans
                        .iter()
                        .map(|span| (span.text.clone(), span_attrs(span, &context, backgrounds)))
                        .collect(),
                })
                .collect(),
        })
        .collect();
    table::Table {
        rows,
        style: style.clone(),
    }
}

/// 【贴图】【片段上下文】记录所属块的字号、标题、引用、代码与行高基准及调色信息
struct SpanContext {
    base: f32,
    heading: bool,
    quote: bool,
    code_block: bool,
    line_height: f32,
    foreground: Rgba,
    canvas: Rgba,
    inline_code: Rgba,
}

/// 【贴图】【片段属性】按 span 与块语义生成宇宙文本属性，返回携带底色索引的属性
fn span_attrs(
    span: &Span,
    context: &SpanContext,
    backgrounds: &mut Vec<Option<Rgba>>,
) -> Attrs<'static> {
    let size = span.style.size.unwrap_or(if span.style.code {
        CODE_SIZE
    } else {
        context.base
    });
    let color = span.style.color.unwrap_or_else(|| {
        if span.style.link {
            link_color(context.canvas)
        } else if context.quote {
            crate::theme::color::muted()
        } else {
            context.foreground
        }
    });
    let family = if span.style.code {
        Family::Monospace
    } else if span.style.serif {
        Family::Serif
    } else {
        Family::SansSerif
    };
    let mut attrs: Attrs<'static> = Attrs::new()
        .family(family)
        .weight(if context.heading || span.style.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        })
        .style(if span.style.italic {
            Style::Italic
        } else {
            Style::Normal
        })
        .color(color.cosmic())
        .metrics(Metrics::new(
            size,
            size * span.style.line_height.unwrap_or(context.line_height),
        ));
    if span.style.underline || span.style.link {
        attrs = attrs
            .underline(UnderlineStyle::Single)
            .underline_color(color.cosmic());
    }
    if span.style.strike {
        attrs = attrs.strikethrough().strikethrough_color(color.cosmic());
    }
    if span.style.overline {
        attrs = attrs.overline().overline_color(color.cosmic());
    }
    let background = if context.code_block {
        None
    } else if span.style.code {
        Some(context.inline_code)
    } else {
        span.style.background
    };
    let metadata = background.map_or(0, |color| {
        backgrounds.push(Some(color));
        backgrounds.len() - 1
    });
    attrs.metadata(metadata)
}

/// 【贴图】【构建缓冲】按 block 与可选的宽度创建宇宙文本缓冲并完成排版
fn build_buffer(fonts: &mut FontSystem, block: &PreparedBlock, width: Option<f32>) -> Buffer {
    build_span_buffer(fonts, &block.spans, block.align, width)
}

/// 【贴图】【片段缓冲】按片段属性、对齐与可选宽度创建宇宙文本缓冲并完成排版
fn build_span_buffer(
    fonts: &mut FontSystem,
    spans: &[(String, Attrs<'static>)],
    align: Align,
    width: Option<f32>,
) -> Buffer {
    let line_height = crate::theme::font::line_height();
    let mut buffer = Buffer::new(fonts, Metrics::new(BODY_SIZE, BODY_SIZE * line_height));
    let default = Attrs::new();
    let alignment = Some(match align {
        Align::Left => cosmic_text::Align::Left,
        Align::Center => cosmic_text::Align::Center,
        Align::Right => cosmic_text::Align::Right,
    });
    buffer.set_rich_text(
        spans
            .iter()
            .map(|(text, attrs)| (text.as_str(), attrs.clone())),
        &default,
        Shaping::Advanced,
        alignment,
    );
    buffer.set_size(width, None);
    buffer.shape_until_scroll(fonts, false);
    buffer
}

/// 【贴图】【块高】按 buffer 返回所有布局行的总高度，空缓冲返回一行高度
fn block_height(buffer: &Buffer) -> f32 {
    let mut height = 0.0_f32;
    for run in buffer.layout_runs() {
        height = height.max(run.line_y + run.line_height);
    }
    if height <= 0.0 {
        BODY_SIZE * crate::theme::font::line_height()
    } else {
        height
    }
}

/// 【贴图】【最大行宽】按 buffer 返回最宽布局行的宽度
fn max_line_width(buffer: &Buffer) -> f32 {
    buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0_f32, f32::max)
}

/// 【贴图】【块间距】按块类型返回与下一块之间的纵向间距
fn block_gap(kind: &BlockKind) -> f32 {
    match kind {
        BlockKind::ListItem { .. } => 2.0,
        BlockKind::Heading(_) => 12.0,
        BlockKind::Code => 10.0,
        BlockKind::Table { .. }
        | BlockKind::Image(_)
        | BlockKind::ImageRow(_)
        | BlockKind::Quote { .. }
        | BlockKind::Rule
        | BlockKind::Paragraph => 8.0,
    }
}

/// 【贴图】【链接颜色】按画布亮度返回可读的链接蓝色，浅色底用深蓝，深色底用亮蓝
fn link_color(canvas: Rgba) -> Rgba {
    if is_light(canvas) {
        Rgba(9, 105, 218, 255)
    } else {
        Rgba(88, 166, 255, 255)
    }
}

/// 【贴图】【画布明暗】按颜色相对亮度判断是否为浅色画布，返回亮度是否大于 140
fn is_light(color: Rgba) -> bool {
    0.2126 * f32::from(color.0) + 0.7152 * f32::from(color.1) + 0.0722 * f32::from(color.2) > 140.0
}

/// 【贴图】【内联代码底色】按画布与前景返回可读底色，浅色底用主题输入底色，深色底混入前景
fn inline_code_background(canvas: Rgba, foreground: Rgba) -> Rgba {
    if is_light(canvas) {
        return crate::theme::color::field();
    }
    let mix = |over: u8, base: u8| {
        (f32::from(base) * 0.9 + f32::from(over) * 0.1)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba(
        mix(foreground.0, canvas.0),
        mix(foreground.1, canvas.1),
        mix(foreground.2, canvas.2),
        255,
    )
}

/// 【贴图】【基础字号】按块类型返回正文、标题或代码字号
fn base_size(block: &Block) -> f32 {
    match block.kind {
        BlockKind::Heading(1) => 30.0,
        BlockKind::Heading(2) => 26.0,
        BlockKind::Heading(3) => 22.0,
        BlockKind::Heading(4) => 19.0,
        BlockKind::Heading(5) => 17.0,
        BlockKind::Heading(_) => 16.0,
        BlockKind::Code => CODE_SIZE,
        _ => BODY_SIZE,
    }
}

/// 【贴图】【块缩进】按列表或引用层级返回左缩进像素
fn block_indent(block: &Block) -> f32 {
    match &block.kind {
        BlockKind::ListItem { depth, .. } => *depth as f32 * LIST_INDENT,
        BlockKind::Quote { depth } => *depth as f32 * QUOTE_INDENT + QUOTE_GAP,
        _ => 0.0,
    }
}

/// 【贴图】【画布布局】记录内容宽度与画布背景，供块背景绘制使用
struct Layout {
    content_width: f32,
    canvas: Rgba,
}

/// 【贴图】【块背景】按块类型绘制显式块背景、代码底框、引用竖条或分隔线
fn draw_block_background(painter: &mut Painter, block: &LaidBlock, layout: &Layout) {
    if matches!(block.kind, BlockKind::Rule) {
        if let Some(rect) = Rect::from_xywh(PADDING, block.y, layout.content_width, 1.0) {
            painter.fill_rounded(rect, 0.0, crate::theme::color::border().color());
        }
        return;
    }
    if let Some(rect) = block_rect(block, layout.content_width) {
        match &block.kind {
            BlockKind::Code => {
                if let Some(background) = block.background {
                    painter.fill_rounded(rect, 6.0, background.color());
                } else if is_light(layout.canvas) {
                    painter.fill_rounded(rect, 6.0, crate::theme::color::field().color());
                }
            }
            _ => {
                if let Some(background) = block.background {
                    painter.fill_rounded(rect, 2.0, background.color());
                }
            }
        }
    }
    if let BlockKind::Quote { depth } = &block.kind {
        let x = PADDING + *depth as f32 * QUOTE_INDENT;
        if let Some(rect) = Rect::from_xywh(x, block.y, 3.0, block.height) {
            painter.fill_rounded(rect, 1.5, crate::theme::color::border().color());
        }
    }
}

/// 【贴图】【块矩形】按已定位块与内容宽度返回覆盖文本与代码内边距的矩形
fn block_rect(block: &LaidBlock, content_width: f32) -> Option<Rect> {
    let indent = (block.x - PADDING).max(0.0);
    let width = (content_width - indent).max(1.0);
    Rect::from_xywh(
        block.x,
        block.y - block.extra,
        width,
        block.height + block.extra * 2.0,
    )
}

/// 【贴图】【片段底色】按 metadata 索引把相邻同色字形合并为圆角矩形，绘制代码与标记底色
fn draw_span_backgrounds(
    painter: &mut Painter,
    buffer: &Buffer,
    origin: (f32, f32),
    backgrounds: &[Option<Rgba>],
) {
    for run in buffer.layout_runs() {
        let glyphs = run.glyphs;
        let mut index = 0;
        while index < glyphs.len() {
            let metadata = glyphs[index].metadata;
            let Some(background) = backgrounds.get(metadata).copied().flatten() else {
                index += 1;
                continue;
            };
            let mut end = index + 1;
            while end < glyphs.len() && glyphs[end].metadata == metadata {
                end += 1;
            }
            let group = &glyphs[index..end];
            let left = group
                .iter()
                .map(|glyph| glyph.x)
                .fold(f32::INFINITY, f32::min);
            let right = group
                .iter()
                .map(|glyph| glyph.x + glyph.w)
                .fold(f32::NEG_INFINITY, f32::max);
            let size = group[0].font_size;
            let width = right - left;
            if width > 0.0 {
                let baseline = origin.1 + run.line_y;
                let top = baseline - size * 0.95;
                if let Some(rect) =
                    Rect::from_xywh(origin.0 + left - 1.0, top, width + 2.0, size * 1.25)
                {
                    painter.fill_rounded(rect, 3.0, background.color());
                }
            }
            index = end;
        }
    }
}

/// 【贴图】【文字装饰】按 run.decorations 绘制下划线、删除线与上划线，坐标与字形基线一致
fn draw_decorations(
    painter: &mut Painter,
    buffer: &Buffer,
    origin: (f32, f32),
    default_color: CosmicColor,
) {
    for run in buffer.layout_runs() {
        for span in run.decorations {
            let glyphs = &run.glyphs[span.glyph_range.clone()];
            if glyphs.is_empty() {
                continue;
            }
            let left = glyphs
                .iter()
                .map(|glyph| glyph.x)
                .fold(f32::INFINITY, f32::min);
            let right = glyphs
                .iter()
                .map(|glyph| glyph.x + glyph.w)
                .fold(f32::NEG_INFINITY, f32::max);
            let width = right - left;
            if width <= 0.0 {
                continue;
            }
            let data = &span.data;
            let decoration = &data.text_decoration;
            let size = span.font_size;
            let fallback = |specific: Option<CosmicColor>| {
                specific.or(span.color_opt).unwrap_or(default_color)
            };
            let x = origin.0 + left;
            if decoration.underline != UnderlineStyle::None {
                let color = sk_color(fallback(decoration.underline_color_opt));
                let thickness = (data.underline_metrics.thickness * size).max(1.0).ceil();
                let y = origin.1 + run.line_y - data.underline_metrics.offset * size;
                fill_rect(painter, x, y, width, thickness, color);
                if decoration.underline == UnderlineStyle::Double {
                    fill_rect(painter, x, y + thickness * 2.0, width, thickness, color);
                }
            }
            if decoration.strikethrough {
                let color = sk_color(fallback(decoration.strikethrough_color_opt));
                let thickness = (data.strikethrough_metrics.thickness * size)
                    .max(1.0)
                    .ceil();
                let y = origin.1 + run.line_y - data.strikethrough_metrics.offset * size;
                fill_rect(painter, x, y, width, thickness, color);
            }
            if decoration.overline {
                let color = sk_color(fallback(decoration.overline_color_opt));
                let thickness = (data.underline_metrics.thickness * size).max(1.0).ceil();
                let y = (origin.1 + run.line_y - data.ascent * size).max(origin.1 + run.line_top);
                fill_rect(painter, x, y, width, thickness, color);
            }
        }
    }
}

/// 【贴图】【色值转换】按宇宙文本颜色返回 tiny-skia 颜色
fn sk_color(color: CosmicColor) -> Color {
    Color::from_rgba8(color.r(), color.g(), color.b(), color.a())
}

/// 【贴图】【矩形填充】按坐标与尺寸使用无圆角路径填充矩形，尺寸至少为 1 像素
fn fill_rect(painter: &mut Painter, x: f32, y: f32, width: f32, height: f32, color: Color) {
    if let Some(rect) = Rect::from_xywh(x, y, width.max(1.0), height.max(1.0)) {
        painter.fill_rounded(rect, 0.0, color);
    }
}
