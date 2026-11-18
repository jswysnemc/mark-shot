//! 【识别资源】【折叠布局】默认突出当前语言，资源详情与其他语言按需展开

use super::{ModelPopover, ModelPopoverElement, cfg};
use crate::ocr::models::MODELS;
use tiny_skia::Rect;

/// 【识别资源】【可见模型】按 panel 返回稳定模型编号，当前或推荐模型始终排在最前
pub(super) fn models(panel: &ModelPopover) -> Vec<usize> {
    if !panel.setup.show_models() {
        return Vec::new();
    }
    let primary = panel
        .rows
        .iter()
        .position(|row| row.active)
        .or_else(|| panel.rows.iter().position(|row| row.recommended))
        .unwrap_or_else(|| {
            MODELS
                .iter()
                .position(|model| model.id == "chinese")
                .unwrap_or(0)
        });
    let mut visible = vec![primary];
    if panel.languages_expanded {
        visible.extend((0..MODELS.len()).filter(|index| *index != primary));
    }
    visible
}

/// 【识别资源】【资源行数】按 panel 返回展开的引擎资源行数，收起时只留摘要
pub(super) fn setup_rows(panel: &ModelPopover) -> usize {
    if !panel.resources_expanded {
        return 0;
    }
    if panel.setup.show_models() {
        super::setup::rows()
    } else {
        2
    }
}

/// 【识别资源】【折叠标题】按 panel、origin 和 resources 返回对应展开控件的矩形
pub(super) fn heading(panel: &ModelPopover, origin: (f32, f32), resources: bool) -> Rect {
    let primary = usize::from(panel.setup.show_models());
    let offset = if resources {
        primary as f32 * cfg().row_height
    } else {
        32.0 + (setup_rows(panel) + primary) as f32 * cfg().row_height
    };
    Rect::from_xywh(
        origin.0 + cfg().padding,
        origin.1 + cfg().padding + cfg().title_height + offset,
        cfg().width - cfg().padding * 2.0,
        32.0,
    )
    .unwrap()
}

/// 【识别资源】【完整高度】按 panel 返回折叠后的逻辑高度，供绘制、定位和窗口适配共用
pub(super) fn height(panel: &ModelPopover) -> f32 {
    cfg().padding * 2.0
        + cfg().title_height
        + 32.0
        + (setup_rows(panel) + models(panel).len()) as f32 * cfg().row_height
        + if panel.setup.show_models() { 32.0 } else { 0.0 }
}

/// 【识别资源】【键盘顺序】按 panel 返回真实可见控件的显示顺序，收起的引擎与语言不参与导航
pub fn navigation_items(panel: &ModelPopover) -> Vec<ModelPopoverElement> {
    let visible = models(panel);
    let mut items = Vec::new();
    // 1. 【识别资源】【当前模型】首先访问顶部当前或推荐语言，其后是资源展开按钮
    if let Some(index) = visible.first() {
        items.push(ModelPopoverElement::Row(*index));
    }
    items.push(ModelPopoverElement::Resources);
    // 2. 【识别资源】【展开资源】仅加入实际显示的引擎与推理库
    for index in 0..setup_rows(panel) {
        items.push(match (cfg!(target_os = "windows"), index) {
            (true, 0) => ModelPopoverElement::SystemEngine,
            (true, 1) => ModelPopoverElement::PaddleEngine,
            _ => ModelPopoverElement::Runtime,
        });
    }
    // 3. 【识别资源】【其他语言】展开按钮之后按卡片位置加入可见语言
    if panel.setup.show_models() {
        items.push(ModelPopoverElement::Languages);
        items.extend(visible.into_iter().skip(1).map(ModelPopoverElement::Row));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【识别资源】【折叠回归】无参数，验证默认只显示一个模型，展开后保留全部语言且高度同步
    #[test]
    fn folding_preserves_language_catalog() {
        let mut panel = ModelPopover::new();
        panel.setup.engine = crate::ocr::engine_choice::Engine::Paddle;
        assert_eq!(models(&panel).len(), 1);
        let compact = height(&panel);
        panel.languages_expanded = true;
        assert_eq!(models(&panel).len(), MODELS.len());
        assert!(height(&panel) > compact);
        panel.resources_expanded = true;
        assert_eq!(setup_rows(&panel), super::super::setup::rows());
    }

    /// 【识别资源测试】【可见导航】无参数，验证收起状态只访问可见模型和展开按钮，展开后才访问其余资源，无返回值
    #[test]
    fn keyboard_navigation_skips_folded_controls() {
        let mut panel = ModelPopover::new();
        panel.setup.engine = crate::ocr::engine_choice::Engine::Paddle;
        let compact = navigation_items(&panel);
        assert_eq!(compact.len(), 3);
        assert!(matches!(compact[0], ModelPopoverElement::Row(_)));
        assert_eq!(compact[1], ModelPopoverElement::Resources);
        assert_eq!(compact[2], ModelPopoverElement::Languages);
        panel.resources_expanded = true;
        let resources = navigation_items(&panel);
        assert_eq!(resources.len(), compact.len() + super::super::setup::rows());
        assert!(resources.contains(&ModelPopoverElement::Runtime));
        panel.languages_expanded = true;
        assert_eq!(
            navigation_items(&panel).len(),
            resources.len() + MODELS.len() - 1
        );
    }
}
