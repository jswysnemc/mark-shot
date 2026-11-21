//! 【贴图测试】【内容兼容】验证色值、HTML 正文及复制文件的首次和重复行为

use super::{
    content,
    html::{self, BlockKind, RichText, Span},
    html_style, rich_text,
    source::Clipboard,
};
use crate::theme::Rgba;

/// 【贴图测试】【片段查找】按 text 查找普通块或表格单元中去除首尾空白后相等的片段，返回片段引用
fn span<'a>(rich: &'a RichText, text: &str) -> &'a Span {
    for block in &rich.blocks {
        match &block.kind {
            BlockKind::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        for candidate in &cell.spans {
                            if candidate.text.trim() == text {
                                return candidate;
                            }
                        }
                    }
                }
            }
            _ => {
                for candidate in &block.spans {
                    if candidate.text.trim() == text {
                        return candidate;
                    }
                }
            }
        }
    }
    panic!("missing span");
}

/// 【贴图测试】【单元文本】按表格单元返回所有片段拼接后的文本
fn cell_text(cell: &super::html::TableCell) -> String {
    cell.spans.iter().map(|span| span.text.as_str()).collect()
}

/// 【贴图测试】【文本拼接】按 block 返回所有片段拼接后的文本
fn text_of(block: &super::html::Block) -> String {
    block.spans.iter().map(|span| span.text.as_str()).collect()
}

/// 【贴图测试】【色值范围】无参数，合法三分量和短 HEX 保留颜色，非法输入继续作为文本
#[test]
fn color_inputs_keep_components_and_reject_invalid_ranges() {
    for (text, expected) in [
        ("#abc", [170, 187, 204, 255]),
        ("#abcd", [170, 187, 204, 221]),
        ("255, 0, 62", [255, 0, 62, 255]),
        ("0.5 0.0 1.0", [128, 0, 255, 255]),
    ] {
        assert_eq!(content::color(text), Some(expected));
    }
    for text in [
        "#ab",
        "#ééé",
        "256 0 0",
        "NaN 0 1",
        "1.1 0.0 1.0",
        "ordinary text",
    ] {
        assert!(content::color(text).is_none());
    }
}

/// 【贴图测试】【HTML转换】无参数，保留中文、换行和实体，丢弃脚本及传输头
#[test]
fn html_pinning_preserves_visible_text_without_executable_content() {
    assert_eq!(
        content::html_text(
            "Version:1.0\n<!--StartFragment--><p>测试 &amp; &#x4e2d;</p><script>secret()</script><div>下一行<br>结束</div><!--EndFragment-->"
        ),
        "测试 & 中\n下一行\n结束"
    );
}

/// 【贴图测试】【文件重复】无参数，图片文件首次使用原像素，再次转为路径；停用路径只保留图片文件首次贴图
#[test]
fn copied_image_is_pinned_then_its_path_can_be_pinned() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sample.png");
    let mut image = tiny_skia::Pixmap::new(19, 13).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(20, 70, 130, 255));
    image.save_png(&path).unwrap();
    let clipboard = Clipboard {
        files: vec![path],
        ..Default::default()
    };
    let png = content::image(&clipboard, false, false).unwrap();
    let first = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((first.width(), first.height()), (19, 13));
    assert!(content::image(&clipboard, true, false).is_err());
    let repeated = content::image(&clipboard, true, true).unwrap();
    assert!(tiny_skia::Pixmap::decode_png(&repeated).unwrap().width() > 19);
}

/// 【贴图测试】【文件URI】无参数，解码空格和中文路径，不接收网页链接或注释
#[test]
fn copied_file_uri_list_ignores_non_file_sources() {
    let uri = if cfg!(target_os = "windows") {
        "file:///C:/Temp/a%20b.png"
    } else {
        "file:///tmp/a%20b.png"
    };
    let files = super::source::files(&format!("# comment\n{uri}\nhttps://example.com/x\n"));
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().ends_with("a b.png"));
}

/// 【贴图测试】【特殊文件】无参数，复制 FIFO 时直接渲染路径，不能等待管道写入者
#[cfg(target_os = "linux")]
#[test]
fn copied_fifo_is_rendered_as_a_path_without_opening_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("named-pipe");
    nix::unistd::mkfifo(
        &path,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .unwrap();
    let clipboard = Clipboard {
        files: vec![path],
        ..Default::default()
    };
    let image = content::image(&clipboard, false, true).unwrap();
    assert!(tiny_skia::Pixmap::decode_png(&image).is_ok());
}

/// 【贴图测试】【内联格式】无参数，粗体、斜体、下划线、删除线与等宽标签分别保留样式
#[test]
fn html_inline_tags_keep_their_formatting_flags() {
    let rich = html::parse("<p>a <b>b</b> <i>i</i> <u>u</u> <s>s</s> <code>c</code></p>");
    assert_eq!(rich.blocks.len(), 1);
    assert!(span(&rich, "b").style.bold);
    assert!(span(&rich, "i").style.italic);
    assert!(span(&rich, "u").style.underline);
    assert!(span(&rich, "s").style.strike);
    assert!(span(&rich, "c").style.code);
    let plain = span(&rich, "a");
    assert!(!plain.style.bold && !plain.style.italic && !plain.style.underline);
}

/// 【贴图测试】【内联样式】无参数，style 属性中的颜色、字重、斜体、字号与背景均生效
#[test]
fn html_inline_style_attributes_apply_supported_declarations() {
    let rich = html::parse(
        "<p><span style=\"color:#ff0000\">r</span>\
         <span style=\"font-weight:700\">w</span>\
         <span style=\"font-style:italic\">i</span>\
         <span style=\"font-size:20px\">s</span>\
         <span style=\"background-color:#00ff00\">g</span></p>",
    );
    assert_eq!(span(&rich, "r").style.color, Some(Rgba(255, 0, 0, 255)));
    assert!(span(&rich, "w").style.bold);
    assert!(span(&rich, "i").style.italic);
    assert_eq!(span(&rich, "s").style.size, Some(20.0));
    assert_eq!(
        span(&rich, "g").style.background,
        Some(Rgba(0, 255, 0, 255))
    );
}

/// 【贴图测试】【块结构】无参数，标题、嵌套列表、引用、代码、分隔线与表格行保持语义
#[test]
fn html_block_tags_build_headings_lists_quotes_code_and_rows() {
    let rich = html::parse(
        "<h1>H</h1><h2>H2</h2><ul><li>a<ol><li>b</li></ol></li></ul>\
         <blockquote>q</blockquote><pre>line1\n  line2</pre><hr>\
         <table><tr><td>c1</td><td>c2</td></tr></table>",
    );
    assert_eq!(rich.blocks[0].kind, BlockKind::Heading(1));
    assert_eq!(rich.blocks[1].kind, BlockKind::Heading(2));
    assert_eq!(
        rich.blocks[2].kind,
        BlockKind::ListItem {
            depth: 0,
            marker: "• ".into()
        }
    );
    assert_eq!(
        rich.blocks[3].kind,
        BlockKind::ListItem {
            depth: 1,
            marker: "1. ".into()
        }
    );
    assert!(text_of(&rich.blocks[3]).contains('b'));
    assert_eq!(rich.blocks[4].kind, BlockKind::Quote { depth: 0 });
    assert_eq!(rich.blocks[5].kind, BlockKind::Code);
    assert_eq!(text_of(&rich.blocks[5]), "line1\n  line2");
    assert_eq!(rich.blocks[6].kind, BlockKind::Rule);
    let rows = match &rich.blocks[7].kind {
        BlockKind::Table { rows } => rows,
        other => panic!("expected table block, got {other:?}"),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(cell_text(&rows[0].cells[0]), "c1");
    assert_eq!(cell_text(&rows[0].cells[1]), "c2");
}

/// 【贴图测试】【实体空白】无参数，实体解码、普通空白折叠、脚本隐藏与传输头排除同时成立
#[test]
fn html_entities_whitespace_and_fragment_are_handled() {
    let rich = html::parse(
        "Version:1.0\n<!--StartFragment--><p>&amp; &#x4e2d;&nbsp;&copy;</p>\
         <script>secret()</script><style>x{}</style><p>a\n   b</p><!--EndFragment-->",
    );
    assert_eq!(text_of(&rich.blocks[0]), "& 中\u{a0}©");
    assert_eq!(text_of(&rich.blocks[1]), "a b");
    let all = rich.blocks.iter().map(text_of).collect::<Vec<_>>().join("");
    assert!(!all.contains("secret") && !all.contains("Version"));
    assert_eq!(html_style::color("#abc"), Some(Rgba(170, 187, 204, 255)));
    assert_eq!(
        html_style::color("rgba(255, 0, 0, 0.5)"),
        Some(Rgba(255, 0, 0, 128))
    );
    assert_eq!(html_style::color("currentcolor"), None);
}

/// 【贴图测试】【富文本图像】无参数，可见内容输出可解码 PNG 并包含红色像素，空内容返回错误
#[test]
fn rich_text_image_renders_png_and_rejects_empty_content() {
    let rich = html::parse(
        "<h1>Title</h1><p>plain <b>bold</b> \
         <span style=\"color:#ff0000\">red</span></p><pre>code\n  x</pre>",
    );
    let png = rich_text::image(&rich).unwrap();
    let image = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert!(image.width() <= 752);
    assert!(image.height() > 32);
    let red = image.pixels().iter().any(|pixel| {
        pixel.red() > 150 && pixel.green() < 120 && pixel.blue() < 120 && pixel.alpha() > 200
    });
    assert!(red, "expected red text pixels");
    assert!(rich_text::image(&html::parse("   ")).is_err());
}

/// 【贴图测试】【容器背景】无参数，IDEA 外层容器背景成为画布、颜色成为默认前景且不泄漏为片段底色
#[test]
fn html_container_background_becomes_canvas() {
    let rich = html::parse(
        "<html><head><meta http-equiv=\"content-type\" content=\"text/html; charset=UTF-8\"></head><body>\
         <div style=\"background-color:#1e1f22;color:#bcbec4\">\
         <pre style=\"font-family:'Source Code Pro',monospace;font-size:12.0pt;\">\
         <span style=\"color:#cf8e6d\">package</span>&#32;main<br>class A</pre>\
         </div></body></html>",
    );
    assert_eq!(rich.background, Some(Rgba(30, 31, 34, 255)));
    assert_eq!(rich.foreground, Some(Rgba(188, 190, 196, 255)));
    assert_eq!(
        span(&rich, "package").style.color,
        Some(Rgba(207, 142, 109, 255))
    );
    assert!(
        rich.blocks
            .iter()
            .flat_map(|block| &block.spans)
            .all(|span| span.style.background != Some(Rgba(30, 31, 34, 255)))
    );
    let png = rich_text::image(&rich).unwrap();
    let image = tiny_skia::Pixmap::decode_png(&png).unwrap();
    let dark = image
        .pixels()
        .iter()
        .any(|pixel| pixel.alpha() > 200 && pixel.red() < 60 && pixel.green() < 60);
    assert!(dark, "expected dark canvas pixels");
    let light = image
        .pixels()
        .iter()
        .any(|pixel| pixel.alpha() > 200 && pixel.red() > 150 && pixel.green() > 150);
    assert!(light, "expected light text pixels");
}

/// 【贴图测试】【块背景】无参数，外层 div 成为画布而段落背景留在其块上且不进入片段样式
#[test]
fn html_block_background_stays_on_its_block() {
    let rich = html::parse(
        "<div style=\"background-color:#eef\"><p style=\"background-color:#fee\">x</p><p>y</p></div>",
    );
    assert_eq!(rich.background, Some(Rgba(238, 238, 255, 255)));
    assert_eq!(rich.blocks[0].background, Some(Rgba(255, 238, 238, 255)));
    assert!(
        rich.blocks
            .iter()
            .flat_map(|block| &block.spans)
            .all(|span| span.style.background.is_none())
    );
}

/// 【贴图测试】【衬线行高】无参数，衬线与等宽字体族及无单位、像素行高分别解析正确
#[test]
fn html_serif_and_line_height_are_parsed() {
    assert!(html_style::serif("Georgia,serif"));
    assert!(!html_style::serif("monospace"));
    assert!(html_style::monospace("monospace"));
    assert_eq!(html_style::line_height("1.6", 16.0), Some(1.6));
    assert_eq!(html_style::line_height("24px", 16.0), Some(1.5));
    let rich = html::parse(
        "<p><span style=\"font-family:Georgia,serif\">g</span>\
         <span style=\"font-family:monospace\">m</span>\
         <span style=\"line-height:1.6\">l</span>\
         <span style=\"font-size:16px;line-height:24px\">p</span></p>",
    );
    assert!(span(&rich, "g").style.serif && !span(&rich, "g").style.code);
    assert!(span(&rich, "m").style.code);
    assert_eq!(span(&rich, "l").style.line_height, Some(1.6));
    assert_eq!(span(&rich, "p").style.line_height, Some(1.5));
}

/// 【贴图测试】【列表块】无参数，列表项内的段落仍保留列表标记与层级
#[test]
fn html_list_item_with_paragraph_keeps_its_marker() {
    let rich = html::parse("<ul><li><p>wrapped</p></li></ul>");
    assert!(matches!(
        rich.blocks[0].kind,
        BlockKind::ListItem { depth: 0, .. }
    ));
    assert!(text_of(&rich.blocks[0]).contains("wrapped"));
    assert!(text_of(&rich.blocks[0]).contains('•'));
}

/// 【贴图测试】【列表标记】按 block 返回列表项标记，非列表返回空文本
fn marker(block: &super::html::Block) -> &str {
    match &block.kind {
        BlockKind::ListItem { marker, .. } => marker.as_str(),
        _ => "",
    }
}

/// 【贴图测试】【标记唯一】无参数，列表项内多个段落只在首个块写出一个标记
#[test]
fn html_list_marker_is_written_once_per_item() {
    let rich = html::parse("<ul><li><p>a</p><p>b</p></li><li>c</li></ul>");
    assert_eq!(marker(&rich.blocks[0]), "• ");
    assert_eq!(marker(&rich.blocks[1]), "");
    assert_eq!(marker(&rich.blocks[2]), "• ");
    assert_eq!(text_of(&rich.blocks[0]).matches('•').count(), 1);
    assert_eq!(text_of(&rich.blocks[1]), "b");
    let bullets = rich
        .blocks
        .iter()
        .map(|block| text_of(block).matches('•').count())
        .sum::<usize>();
    assert_eq!(bullets, 2);
}

/// 【贴图测试】【优先级注释】无参数，!important 后缀被忽略且样式颜色仍生效
#[test]
fn html_style_ignores_important_annotations() {
    assert_eq!(
        html_style::color("#ff0000 !important"),
        Some(Rgba(255, 0, 0, 255))
    );
    let rich = html::parse(
        "<p><span style=\"color:#00ff00 !important\">g</span>\
         <span style=\"font-weight:700!important\">w</span></p>",
    );
    assert_eq!(span(&rich, "g").style.color, Some(Rgba(0, 255, 0, 255)));
    assert!(span(&rich, "w").style.bold);
}

/// 【贴图测试】【表头字重】无参数，表头默认加粗但 style 的 font-weight:normal 可覆盖
#[test]
fn html_table_header_weight_can_be_overridden() {
    let rich =
        html::parse("<table><tr><th>h</th><th style=\"font-weight:normal\">n</th></tr></table>");
    assert!(span(&rich, "h").style.bold);
    assert!(!span(&rich, "n").style.bold);
}

/// 【贴图测试】【表格结构】无参数，表格保留行列、表头、单元背景、链接与默认加粗
#[test]
fn html_table_preserves_rows_cells_headers_and_backgrounds() {
    let rich = html::parse(
        "<table><tr><th>Document</th><th style=\"background-color:#eee\">Content</th></tr>\
         <tr><td><a href=\"#\">Installation</a></td><td>Build deps</td></tr></table>",
    );
    let rows = match &rich.blocks[0].kind {
        BlockKind::Table { rows } => rows,
        other => panic!("expected table block, got {other:?}"),
    };
    assert_eq!(rows.len(), 2);
    assert!(rows[0].header);
    assert!(!rows[1].header);
    assert_eq!(cell_text(&rows[0].cells[0]), "Document");
    assert_eq!(cell_text(&rows[0].cells[1]), "Content");
    assert_eq!(rows[0].cells[1].background, Some(Rgba(238, 238, 238, 255)));
    assert_eq!(cell_text(&rows[1].cells[0]), "Installation");
    assert_eq!(cell_text(&rows[1].cells[1]), "Build deps");
    assert!(span(&rich, "Document").style.bold);
    assert!(span(&rich, "Installation").style.link);
}

/// 【贴图测试】【表格绘制】无参数，多列不同长度文字绘制为不超 720 宽的表并出现边框像素
#[test]
fn html_table_renders_aligned_columns_and_borders() {
    let rich = html::parse(
        "<table><tr><th>Name</th><th>Value</th><th>Note</th></tr>\
         <tr><td>a</td><td>1</td><td>short</td></tr>\
         <tr><td>a much longer label here</td><td>22</td><td>text</td></tr></table>",
    );
    let png = rich_text::image(&rich).unwrap();
    let image = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert!(image.width() <= 752);
    assert!(image.height() > 0);
    let borders = image
        .pixels()
        .iter()
        .filter(|pixel| {
            pixel.alpha() > 200
                && pixel.red() >= 180
                && pixel.red() <= 245
                && pixel.green() >= 185
                && pixel.green() <= 247
                && pixel.blue() >= 190
                && pixel.blue() <= 249
                && pixel.red() < pixel.green()
                && pixel.green() < pixel.blue()
        })
        .count();
    assert!(borders > 0, "expected table border pixels");
}

/// 【贴图测试】【代码宽度】无参数，长等宽代码行不按 720 换行而短文仍受 720 限制
#[test]
fn html_code_block_keeps_long_lines_up_to_cap() {
    let line = "x".repeat(120);
    let rich = html::parse(&format!(
        "<pre style=\"font-family:monospace\">{line}</pre>"
    ));
    let image = tiny_skia::Pixmap::decode_png(&rich_text::image(&rich).unwrap()).unwrap();
    assert!(image.width() > 752, "long code should not wrap at 720");
    assert!(
        image.width() <= 1664,
        "code should stay within the 1600 cap"
    );
    let short = html::parse("<p>short prose</p>");
    let short = tiny_skia::Pixmap::decode_png(&rich_text::image(&short).unwrap()).unwrap();
    assert!(short.width() <= 752);
}

/// 【贴图测试】【链接颜色】无参数，链接使用可读的深蓝文字而不是低透明度主题色
#[test]
fn html_links_render_with_readable_blue_text() {
    let rich = html::parse("<p><a href=\"https://example.com\">link</a></p>");
    let png = rich_text::image(&rich).unwrap();
    let image = tiny_skia::Pixmap::decode_png(&png).unwrap();
    let readable = image
        .pixels()
        .iter()
        .filter(|pixel| {
            pixel.alpha() > 200
                && pixel.red() < 80
                && pixel.blue() > 150
                && pixel.blue() > pixel.red() + 100
        })
        .count();
    assert!(readable >= 3, "expected readable blue link pixels");
}
