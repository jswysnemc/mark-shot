//! 【识别资源】【折叠布局】默认突出当前语言，资源详情与其他语言按需展开

use super::{ModelPopover, cfg};
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
}
