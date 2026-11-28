//! 【贴图测试】【图片与对齐】验证 align 继承、占位图尺寸、图片行排版与 GitHub 头部布局

use super::{
    html::{self, Align, BlockKind, RichText},
    local_image, rich_text, text_of,
};

/// 【贴图测试】【渲染解码】按富文本渲染并解码 PNG，失败即 panic
fn decoded(rich: &RichText) -> tiny_skia::Pixmap {
    tiny_skia::Pixmap::decode_png(&rich_text::image(rich, 1.0).unwrap()).unwrap()
}

/// 【贴图测试】【非背景像素】按图片返回与左上角画布颜色不同的像素数量
fn non_background(image: &tiny_skia::Pixmap) -> usize {
    let Some(background) = image.pixel(0, 0) else {
        return 0;
    };
    image
        .pixels()
        .iter()
        .filter(|pixel| {
            (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha())
                != (
                    background.red(),
                    background.green(),
                    background.blue(),
                    background.alpha(),
                )
        })
        .count()
}

/// 【贴图测试】【对齐继承】无参数，align 属性使标题与段落居中且嵌套作用域可覆盖
#[test]
fn html_align_attribute_centers_blocks_and_nested_overrides() {
    let rich = html::parse("<div align=\"center\"><h1>Title</h1><p>sub</p></div>");
    assert_eq!(rich.blocks[0].kind, BlockKind::Heading(1));
    assert_eq!(rich.blocks[0].align, Align::Center);
    assert_eq!(rich.blocks[1].align, Align::Center);

    let nested =
        html::parse("<div align=\"center\"><div align=\"right\"><p>in</p></div><p>out</p></div>");
    assert_eq!(text_of(&nested.blocks[0]), "in");
    assert_eq!(nested.blocks[0].align, Align::Right);
    assert_eq!(text_of(&nested.blocks[1]), "out");
    assert_eq!(nested.blocks[1].align, Align::Center);
}

/// 【贴图测试】【网络占位】无参数，网络图片生成空数据占位块并按 alt 估算尺寸
#[test]
fn html_network_image_renders_placeholder() {
    let rich = html::parse("<p><img src=\"https://example.com/x.png\" alt=\"Release\"></p>");
    let BlockKind::Image(image) = &rich.blocks[0].kind else {
        panic!("expected a placeholder image block");
    };
    assert!(image.data.is_none());
    assert_eq!(image.label, "Release");
    assert_eq!(image.height, 20);
    assert!(image.width > 0);
    let rendered = decoded(&rich);
    assert!(non_background(&rendered) > 0, "placeholder should draw");
}

/// 【贴图测试】【正方形占位】无参数，仅声明宽度 112 的 logo 占位为 112×112
#[test]
fn html_network_logo_width_only_becomes_square_placeholder() {
    let rich =
        html::parse("<img src=\"https://example.com/mark-shot.svg\" width=\"112\" alt=\"logo\">");
    let BlockKind::Image(image) = &rich.blocks[0].kind else {
        panic!("expected a placeholder image block");
    };
    assert_eq!((image.width, image.height), (112, 112));
    assert_eq!(image.display_width, Some(112.0));
    assert_eq!(image.display_height, Some(112.0));
}

/// 【贴图测试】【图片行】无参数，段落中的两张图片合并为一行并保持居中且不超宽
#[test]
fn html_paragraph_images_form_one_centered_row() {
    let rich = html::parse(
        "<div align=\"center\"><p>\
         <img src=\"https://a.example/1\" alt=\"Release\">\
         <img src=\"https://b.example/2\" alt=\"Gitter\">\
         </p></div>",
    );
    let BlockKind::ImageRow(images) = &rich.blocks[0].kind else {
        panic!("expected an image row");
    };
    assert_eq!(images.len(), 2);
    assert!(images.iter().all(|image| image.data.is_none()));
    assert_eq!(rich.blocks[0].align, Align::Center);
    let rendered = decoded(&rich);
    assert!(rendered.width() <= 752);
    assert!(
        rendered.height() < 60,
        "both placeholders should share one line"
    );
}

/// 【贴图测试】【混合图片行】无参数，本地图片与网络占位图可同处一行且本地数据保留
#[test]
fn html_mixed_local_and_placeholder_form_a_row() {
    let (_directory, url) = local_image(20, 20);
    let rich = html::parse(&format!(
        "<p><img src=\"{url}\" alt=\"local\"><img src=\"https://example.com/b.png\" alt=\"Remote\"></p>"
    ));
    let BlockKind::ImageRow(images) = &rich.blocks[0].kind else {
        panic!("expected a mixed image row");
    };
    assert_eq!(images.len(), 2);
    assert!(images[0].data.is_some());
    assert!(images[1].data.is_none());
    assert!(tiny_skia::Pixmap::decode_png(&rich_text::image(&rich, 1.0).unwrap()).is_ok());
}

/// 【贴图测试】【GitHub头部】无参数，居中 div 使 logo、标题、副标题与徽章行全部居中
#[test]
fn html_github_header_centers_logo_heading_and_badges() {
    let rich = html::parse(
        "<div align=\"center\">\
         <a href=\"#\"><img src=\"https://example.com/mark-shot.svg\" alt=\"Mark Shot logo\" width=\"112\" style=\"max-width:100%\"></a>\
         <h1>Mark Shot</h1>\
         <p>Native screenshot tool</p>\
         <p>\
         <a href=\"#\"><img src=\"https://example.com/b1\" alt=\"Release\"></a>\
         <a href=\"#\"><img src=\"https://example.com/b2\" alt=\"Gitter\"></a>\
         <a href=\"#\"><img src=\"https://example.com/b3\" alt=\"Language: Rust\"></a>\
         </p></div>",
    );
    assert_eq!(rich.blocks.len(), 4);
    assert!(matches!(rich.blocks[0].kind, BlockKind::Image(_)));
    assert_eq!(rich.blocks[0].align, Align::Center);
    assert_eq!(rich.blocks[1].kind, BlockKind::Heading(1));
    assert_eq!(rich.blocks[1].align, Align::Center);
    assert_eq!(rich.blocks[2].align, Align::Center);
    let BlockKind::ImageRow(badges) = &rich.blocks[3].kind else {
        panic!("expected a badge row");
    };
    assert_eq!(badges.len(), 3);
    assert_eq!(rich.blocks[3].align, Align::Center);
    assert!(tiny_skia::Pixmap::decode_png(&rich_text::image(&rich, 1.0).unwrap()).is_ok());
}

/// 【贴图测试】【徽章单行】无参数，略超可用宽度的 6 个徽章占位等比缩小后保持一行
#[test]
fn slightly_wide_badge_row_scales_to_one_line() {
    let rich = html::parse(
        "<div align=\"center\"><p>\
         <img src=\"https://e/1\" alt=\"Release\">\
         <img src=\"https://e/2\" alt=\"Gitter\">\
         <img src=\"https://e/3\" alt=\"Language: Rust\">\
         <img src=\"https://e/4\" alt=\"Platform: Linux, Windows\">\
         <img src=\"https://e/5\" alt=\"Display: Wayland, X11\">\
         <img src=\"https://e/6\" alt=\"License: MIT OR Apache-2.0\">\
         </p></div>",
    );
    let BlockKind::ImageRow(images) = &rich.blocks[0].kind else {
        panic!("expected a badge row");
    };
    assert_eq!(images.len(), 6);
    let rendered = decoded(&rich);
    assert!(rendered.width() <= 752);
    assert!(
        rendered.height() < 60,
        "badges should share one line, got {}",
        rendered.height()
    );
}
