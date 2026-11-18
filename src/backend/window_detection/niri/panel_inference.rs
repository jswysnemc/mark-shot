//! 【niri 窗口识别】【自定义面板】从水平面板命名空间和最高平铺列推算有界独占区

use super::{layout, model::Layer};

/// 【niri 窗口识别】【面板边缘】按 namespace 识别水平 bar、dock、panel，返回上边或下边索引，方向冲突返回空值
fn edge(namespace: &str) -> Option<usize> {
    let namespace = namespace.to_ascii_lowercase();
    let words: Vec<_> = namespace
        .split(|value: char| !value.is_ascii_alphanumeric())
        .collect();
    if !words
        .iter()
        .any(|word| ["bar", "dock", "panel"].contains(word))
        || words
            .iter()
            .any(|word| ["left", "right", "vertical"].contains(word))
        || (words.contains(&"top") && words.contains(&"bottom"))
    {
        return None;
    }
    Some(usize::from(
        words.contains(&"bottom") || (!words.contains(&"top") && words.contains(&"dock")),
    ))
}

/// 【niri 窗口识别】【独占区推算】按 layers、output、layout、height 和 tallest 返回单边占用，缺少证据或超过输出高度四分之一时返回空值
pub(super) fn insets(
    layers: &[Layer],
    output: &str,
    layout: &layout::Layout,
    height: f64,
    tallest: Option<f64>,
) -> Option<[f64; 4]> {
    // 1. 【niri 窗口识别】【表面筛选】只接受当前输出顶部或覆盖层的水平面板，方向不一致时保留手动覆盖入口
    let mut detected = None;
    for layer in layers {
        if layer.output.as_deref() != Some(output)
            || !layer.layer.as_deref().is_some_and(|name| {
                name.eq_ignore_ascii_case("top") || name.eq_ignore_ascii_case("overlay")
            })
        {
            continue;
        }
        let Some(index) = edge(&layer.namespace) else {
            continue;
        };
        if detected.is_some_and(|previous| previous != index) {
            return None;
        }
        detected = Some(index);
    }
    // 2. 【niri 窗口识别】【高度校验】最高列已包含间隙，只扣除 struts，拒绝未满高窗口产生的大幅误判
    let index = detected?;
    let amount = height - layout.struts[0] - layout.struts[1] - tallest?;
    if !height.is_finite()
        || height <= 0.0
        || !amount.is_finite()
        || amount <= 0.0
        || amount > height * 0.25
    {
        return None;
    }
    let mut insets = [0.0; 4];
    insets[index] = amount;
    Some(insets)
}
