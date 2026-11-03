//! 【贴图测试】【原图与菜单】不连接显示服务器，验证旋转、缩放、透明度和菜单命中边界

use mark_shot::features::pinned_image::{
    menu::{self, PinAction},
    model::PinnedImage,
};
use tiny_skia::{Color, Pixmap};

/// 【贴图测试】【样本创建】返回非方形透明样本，像素颜色随位置变化
fn sample() -> Vec<u8> {
    let mut image = Pixmap::new(9, 5).unwrap();
    for (index, pixel) in image.pixels_mut().iter_mut().enumerate() {
        *pixel = Color::from_rgba8(index as u8 * 3, index as u8 * 2, 100, 128)
            .premultiply()
            .to_color_u8();
    }
    image.encode_png().unwrap()
}

/// 【贴图测试】【旋转回环】四次右转必须完整恢复预乘像素，左转与右转相互抵消
#[test]
fn rotation_keeps_all_source_pixels() {
    let mut image = PinnedImage::decode(&sample(), 1.2).unwrap();
    let original = image.image.clone();
    image.rotate(true).unwrap();
    assert_eq!((image.image.width(), image.image.height()), (5, 9));
    image.rotate(false).unwrap();
    assert_eq!(image.image.data(), original.data());
    for _ in 0..4 {
        image.rotate(true).unwrap();
    }
    assert_eq!(image.image.data(), original.data());
}

/// 【贴图测试】【显示与导出分离】滚轮缩放不改变 PNG 原图尺寸、像素或透明度
#[test]
fn zoom_does_not_change_export() {
    let mut image = PinnedImage::decode(&sample(), 1.0).unwrap();
    let original = image.png().unwrap();
    assert!(image.set_zoom(4.0, (1280, 800)));
    assert_eq!(image.size(), (36, 20));
    assert_eq!(image.png().unwrap(), original);
    image.rotate(true).unwrap();
    let output = image::load_from_memory(&image.png().unwrap())
        .unwrap()
        .into_rgba8();
    assert_eq!(output.dimensions(), (5, 9));
    assert!(output.pixels().all(|pixel| pixel[3] == 128));
}

/// 【贴图测试】【缩放输入】非法缩放不会改变状态，合法极端值仍限制在屏幕范围
#[test]
fn scale_and_size_are_bounded() {
    for density in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(PinnedImage::decode(&sample(), density).is_err());
    }
    let mut image = PinnedImage::decode(&sample(), 1.0).unwrap();
    for zoom in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(!image.set_zoom(zoom, (50, 50)));
    }
    image.set_zoom(1000.0, (20, 10));
    assert!(image.size().0 <= 20 && image.size().1 <= 10);
    image.set_zoom(1000.0, (1280, 800));
    assert_eq!(image.zoom, 6.0);
}

/// 【贴图测试】【菜单项目】所有可操作行中心命中唯一项目，分隔线与边距不触发动作
#[test]
fn menu_hit_regions_match_rows() {
    let mut top = menu::PADDING;
    for (index, item) in menu::ITEMS.iter().enumerate() {
        let height = if item.action.is_some() {
            menu::ROW
        } else {
            menu::SEPARATOR
        };
        assert_eq!(
            menu::hit((40.0, f64::from(top + height / 2))),
            item.action.map(|_| index)
        );
        top += height;
    }
    assert_eq!(top + menu::PADDING, menu::height());
    assert_eq!(menu::hit((-1.0, 20.0)), None);
    assert_eq!(menu::hit((menu::WIDTH as f64, 20.0)), None);
}

/// 【贴图测试】【菜单可用性】没有文字选择不能复制选中文字，没有置顶协议不能提供置顶假状态
#[test]
fn unavailable_actions_are_not_executable() {
    assert!(!menu::enabled(
        PinAction::CopySelectedText,
        true,
        false,
        true
    ));
    assert!(menu::enabled(PinAction::CopySelectedText, true, true, true));
    assert!(!menu::enabled(PinAction::AlwaysOnTop, false, false, false));
    assert!(menu::enabled(PinAction::TextSelection, true, false, false));
}

/// 【贴图测试】【识别进程边界】错误任务返回有界错误帧，不连接 Wayland，也不创建模型目录
#[test]
fn recognition_worker_rejects_invalid_requests_without_graphics_or_models() {
    use mark_shot::ocr::daemon::protocol::{self, Outgoing, Reply};
    use std::process::{Command, Stdio};
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mark-shot"))
        .arg(mark_shot::features::pinned_image::recognition::WORKER_ARG)
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    protocol::write_request(&mut child.stdin.take().unwrap(), Outgoing::Status).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        matches!(protocol::read_reply(&mut output.stdout.as_slice()).unwrap(), Reply::Error(error) if error.contains("expected model files"))
    );
    assert!(!root.path().join("data/mark-shot/models").exists());
}

/// 【贴图测试】【识别预取消】已取消的任务不读取模型或创建子进程
#[test]
fn cancelled_recognition_does_not_initialize_a_worker() {
    use mark_shot::ocr::{OcrImage, models::ModelFiles};
    let files = ModelFiles {
        detector: "absent".into(),
        recognizer: "absent".into(),
        dict: "absent".into(),
    };
    let image = OcrImage {
        rgb: vec![0; 3],
        width: 1,
        height: 1,
        origin: (0.0, 0.0),
    };
    let result = mark_shot::features::pinned_image::recognition::recognize(
        image,
        files,
        &std::sync::atomic::AtomicBool::new(true),
    );
    assert!(result.unwrap_err().contains("cancelled"));
}
