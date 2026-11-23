//! 【贴图缩放测试】【尺寸微反馈】窗口尺寸变化后显示当前贴图倍率，首次初始化尺寸不打扰用户

use super::*;

/// 【贴图缩放测试】【状态构造】按默认偏好创建无原生窗口的一像素贴图，返回可接收尺寸事件的贴图状态
fn pin() -> Pin {
    let png = crate::utils::encode_rgba(&[71, 92, 113, 255], 1, 1);
    Pin::new(
        PinnedImage::decode(&png, 1.0).unwrap(),
        None,
        None,
        &crate::config::Pinned::default(),
    )
    .unwrap()
}

/// 【贴图缩放测试】【拖动倍率】验证初始化尺寸不触发微提示，后续调整显示相对原图的百分比并请求重绘
#[test]
fn resizing_the_window_shows_the_current_zoom_percentage() {
    let mut pin = pin();
    assert!(matches!(
        pin.event(WindowEvent::Resize { size: (3, 3) }),
        WindowAction::None
    ));
    assert!(
        pin.feedback.hud_title().is_none(),
        "initial window size must not show a zoom badge"
    );
    assert!(matches!(
        pin.event(WindowEvent::Resize { size: (2, 2) }),
        WindowAction::Redraw
    ));
    assert_eq!(pin.feedback.hud_title(), Some("200%"));
}
