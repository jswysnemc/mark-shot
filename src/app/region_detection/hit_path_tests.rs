//! 【截图控件测试】【层级回归】验证默认层级、模式切换、显式身份和异步路径兼容

use super::*;
/// 【截图控件测试】【窗口样本】无参数，返回冻结窗口范围
fn window() -> PixelRect {
    PixelRect::new(-100, -50, 400, 300)
}
/// 【截图控件测试】【路径样本】无参数，返回带有叶子和容器的真实嵌套路径
fn path() -> Vec<PixelRect> {
    vec![
        PixelRect::new(0, 0, 30, 30),
        PixelRect::new(-20, -20, 100, 100),
        window(),
    ]
}
/// 【截图控件测试】【命中状态】无参数，返回当前指针已经收到组件结果的状态
fn ready() -> HitPath {
    let mut hit = HitPath::default();
    hit.target((10, 10), Some((1, window())));
    assert!(hit.apply((10, 10), path(), false));
    hit
}
/// 【截图控件测试】【默认与端点】无参数，验证默认选择最深组件，滚轮按离散量切换并停留端点
#[test]
fn defaults_to_leaf_and_accumulates_wheel_steps() {
    let mut hit = ready();
    assert_eq!(hit.current(), Some(path()[0]));
    assert_eq!(hit.step(30.0), Some(path()[0]));
    assert_eq!(hit.step(90.0), Some(path()[1]));
    assert_eq!(hit.step(240.0), Some(window()));
    assert_eq!(hit.step(120.0), Some(window()));
    assert_eq!(hit.step(-360.0), Some(path()[0]));
}
/// 【截图控件测试】【无需等待】无参数，验证整窗切换不等待组件接口，返回组件模式后恢复最深层级
#[test]
fn window_mode_is_immediate_and_component_mode_restores_leaf() {
    let mut hit = HitPath::default();
    hit.target((10, 10), Some((1, window())));
    assert!(!hit.native_selection());
    hit.toggle();
    assert!(hit.window_only());
    assert!(hit.native_selection());
    assert_eq!(hit.current(), Some(window()));
    hit.apply((10, 10), path(), false);
    assert_eq!(hit.step(-120.0), Some(window()));
    hit.toggle();
    assert_eq!(hit.current(), Some(path()[0]));
}
/// 【截图控件测试】【显式保留】无参数，验证补充查询插入更细组件时保留用户选择的矩形，包括零索引显式选择
#[test]
fn refinement_preserves_explicit_rect_instead_of_numeric_index() {
    for delta in [-120.0, 120.0, 240.0] {
        let mut hit = ready();
        let selected = hit.step(delta);
        let mut refined = path();
        refined.insert(0, PixelRect::new(8, 8, 8, 8));
        assert!(hit.apply((10, 10), refined, true));
        assert_eq!(hit.current(), selected);
    }
    let mut hit = ready();
    let leaf = PixelRect::new(8, 8, 8, 8);
    let mut refined = path();
    refined.insert(0, leaf);
    assert!(hit.apply((10, 10), refined, true));
    assert_eq!(hit.current(), Some(leaf));
}
/// 【截图控件测试】【路径归属】无参数，验证过期位置、外窗边界和更换分支的补充结果直接丢弃
#[test]
fn rejects_stale_foreign_and_incompatible_refinement() {
    let mut hit = ready();
    assert!(!hit.apply((11, 10), path(), false));
    let mut foreign = path();
    *foreign.last_mut().unwrap() = PixelRect::new(0, 0, 1000, 1000);
    assert!(!hit.apply((10, 10), foreign, false));
    let mut sibling = path();
    sibling[0] = PixelRect::new(5, 5, 30, 30);
    assert!(!hit.apply((10, 10), sibling, true));
    assert!(!hit.apply((10, 10), vec![window()], false));
    assert_eq!(hit.current(), Some(path()[0]));
}
/// 【截图控件测试】【指针容差】无参数，验证六像素内保留滚轮范围，移出叶子或切换窗口后恢复默认
#[test]
fn pointer_motion_preserves_nearby_preference_and_resets_other_targets() {
    let mut hit = ready();
    hit.step(120.0);
    hit.target((16, 10), Some((1, window())));
    assert_eq!(hit.current(), Some(path()[1]));
    hit.target((17, 10), Some((1, window())));
    assert_eq!(hit.current(), Some(path()[0]));
    hit.target((31, 10), Some((1, window())));
    assert_eq!(hit.current(), Some(window()));
    hit.target((10, 10), Some((2, window())));
    assert!(!hit.native_selection());
}
