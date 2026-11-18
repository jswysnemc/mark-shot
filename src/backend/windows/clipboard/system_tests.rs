//! 【Windows剪贴板测试】【系统交接】仅允许私有测试会话执行，避免改动真实桌面剪贴板

use super::*;
use crate::backend::ClipboardProvider;

/// 【Windows剪贴板测试】【完整所有权】无参数，真实系统接收PNG和DIBV5，隐藏拥有窗口释放后仍可读取；再仅提供DIB验证回退，无返回值
#[test]
#[ignore = "requires an isolated Windows clipboard session"]
fn png_and_dibv5_survive_owner_destruction_and_dib_only_fallback() {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
        Ok("1")
    );
    let (rgba, png) = image_tests::fixture();
    Provider.copy_image_to_clipboard(&png).unwrap();
    assert_eq!(read_image_png().unwrap(), png);
    let dib = {
        let _opened = Opened::read().unwrap();
        let compatible = memory::read(8, 128 * 1024 * 1024).unwrap();
        assert_eq!(u32::from_le_bytes(compatible[..4].try_into().unwrap()), 40);
        assert_eq!(
            u16::from_le_bytes(compatible[14..16].try_into().unwrap()),
            24
        );
        assert_eq!(
            u32::from_le_bytes(compatible[16..20].try_into().unwrap()),
            0
        );
        assert_eq!(
            i32::from_le_bytes(compatible[8..12].try_into().unwrap()),
            16
        );
        memory::read(17, 128 * 1024 * 1024).unwrap()
    };
    let decoded = ::image::load_from_memory(&image::png(&dib).unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(decoded.as_raw(), &rgba);
    // 1. 【Windows剪贴板测试】【单格式交付】不登记PNG数据，实际读取必须使用DIBV5回退而非原PNG缓存
    {
        let opened = Opened::write().unwrap();
        opened.clear().unwrap();
        Buffer::new(&dib).unwrap().give(17).unwrap();
    }
    let decoded = ::image::load_from_memory(&read_image_png().unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(decoded.as_raw(), &rgba);
    // 2. 【Windows剪贴板测试】【格式替换】Unicode写入清除图像，全部系统格式随范围结束正常交接
    copy_text("透明图像测试\n𠮷").unwrap();
    assert_eq!(read_text().unwrap(), "透明图像测试\n𠮷");
    assert!(read_image_png().is_err());
}

/// 【Windows剪贴板测试】【文件及HTML】无参数，私有会话中验证 CF_HDROP Unicode 路径和 HTML 格式的真实读取
#[test]
#[ignore = "requires an isolated Windows clipboard session"]
fn copied_files_and_html_are_read_from_native_formats() {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
        Ok("1")
    );
    // 1. 【Windows剪贴板测试】【文件列表】交接双零终止 Unicode 路径，保留空格和中文
    let paths = [r"C:\截图目录\first image.png", r"C:\documents\notes.txt"];
    let mut drop = vec![0_u8; 20];
    drop[..4].copy_from_slice(&20_u32.to_le_bytes());
    drop[16..20].copy_from_slice(&1_u32.to_le_bytes());
    for path in paths {
        drop.extend(path.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
    }
    drop.extend(0_u16.to_le_bytes());
    {
        let opened = Opened::write().unwrap();
        opened.clear().unwrap();
        Buffer::new(&drop).unwrap().give(15).unwrap();
    }
    assert_eq!(read_files().unwrap(), paths.map(std::path::PathBuf::from));
    // 2. 【Windows剪贴板测试】【HTML交接】传输格式按 UTF-8 读取，不依赖纯文本替代格式
    let html = "Version:1.0\r\n<!--StartFragment--><p>测试 &amp; HTML</p><!--EndFragment-->";
    let name: Vec<_> = "HTML Format".encode_utf16().chain([0]).collect();
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    {
        let opened = Opened::write().unwrap();
        opened.clear().unwrap();
        Buffer::new(format!("{html}\0").as_bytes())
            .unwrap()
            .give(format)
            .unwrap();
    }
    assert_eq!(read_html().unwrap(), html);
}
