//! 【识别窗口测试】【尺寸与像素】验证折叠窗口不留下空白透底，展开控件使用统一悬停反馈

use super::*;

/// 【识别窗口测试】【状态准备】无参数，返回不会自动下载或关闭的资源窗口，不连接桌面服务
fn guide() -> Guide {
    Guide::new(OcrModels::load(), Arc::new(OnceLock::new()), false)
}

/// 【识别窗口测试】【真实绘制】按 guide 返回窗口内容的实际像素，不使用界面草图代替生产绘制
fn render(guide: &mut Guide) -> Pixmap {
    let mut canvas = Pixmap::new(guide.size.0, guide.size.1).unwrap();
    guide.draw(&mut canvas, 1.0);
    canvas
}

/// 【识别窗口测试】【内容高度】无参数，折叠窗口按内容收缩，资源及语言展开时分别增高，再收起恢复初始尺寸，无返回值
#[test]
fn window_height_tracks_resource_and_language_expansion() {
    let mut guide = guide();
    let compact = guide
        .preferred_size()
        .expect("OCR guide must expose its content size");
    assert!(
        compact.1 < 300,
        "collapsed guide retained a large blank window"
    );
    guide.activate(ModelPopoverElement::Resources);
    let resources = guide.preferred_size().unwrap();
    assert!(resources.1 > compact.1);
    guide.activate(ModelPopoverElement::Languages);
    assert!(guide.preferred_size().unwrap().1 > resources.1);
    guide.activate(ModelPopoverElement::Resources);
    guide.activate(ModelPopoverElement::Languages);
    assert_eq!(guide.preferred_size(), Some(compact));
}

/// 【识别窗口测试】【窗口底色】无参数，实际内容底色保持不透明，父窗口文字不会透过折叠窗口，无返回值
#[test]
fn guide_background_is_opaque_below_the_model_card() {
    let mut guide = guide();
    let canvas = render(&mut guide);
    assert_eq!(
        canvas
            .pixel(guide.size.0 / 2, guide.size.1 - 8)
            .unwrap()
            .alpha(),
        255
    );
}

/// 【识别窗口测试】【展开悬停】无参数，资源展开控件悬停改变实际背景像素，移出后恢复原像素，无返回值
#[test]
fn expansion_heading_has_visible_hover_feedback() {
    let mut guide = guide();
    let normal = render(&mut guide);
    guide.panel.hovered = Some(ModelPopoverElement::Resources);
    let hovered = render(&mut guide);
    assert!(
        normal.data() != hovered.data(),
        "expansion heading has no hover feedback"
    );
    guide.panel.hovered = None;
    assert!(
        normal.data() == render(&mut guide).data(),
        "leaving the heading did not restore its pixels"
    );
}
