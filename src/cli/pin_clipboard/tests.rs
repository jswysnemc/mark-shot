//! 【贴图测试】【内容兼容】验证色值、HTML 正文及复制文件的首次和重复行为

use super::{content, source::Clipboard};

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
