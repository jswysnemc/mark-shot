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
