//! 【截图输入法】【浮层层级】KDE 截图只在文字输入期间降低层级，始终保留 layer-shell

use crate::backend::wayland::utils::surface::{Role, SurfaceData};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::Layer;
use std::collections::HashMap;

pub(super) struct Policy {
    kde: bool,
    layer: Layer,
    pub namespace: &'static str,
}

/// 【截图输入法】【桌面识别】按 desktop 的标准冒号分隔值判断是否为 KDE，返回识别结果
fn kde(desktop: &str) -> bool {
    desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE") || name.eq_ignore_ascii_case("PLASMA"))
}

impl Policy {
    /// 【截图输入法】【初始策略】按 desktop 返回初始 Overlay 层级与命名空间，KDE 动画策略不变
    fn for_desktop(desktop: &str) -> Self {
        let kde = kde(desktop);
        Self {
            kde,
            layer: Layer::Overlay,
            namespace: if kde { "utility" } else { "mark-shot" },
        }
    }

    /// 【截图输入法】【当前桌面】无参数，按环境变量创建浮层策略，返回策略实例
    pub(super) fn current() -> Self {
        Self::for_desktop(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default())
    }

    /// 【截图输入法】【表面层级】按 scrolling 返回当前截图层级，滚动控制始终保留 Overlay
    pub(super) fn layer(&self, scrolling: bool) -> Layer {
        if scrolling {
            Layer::Overlay
        } else {
            self.layer
        }
    }

    /// 【截图输入法】【层级裁决】按 editing 和 scrolling 返回目标层级，仅 KDE 文字输入需要 Top
    fn target(&self, editing: bool, scrolling: bool) -> Layer {
        if self.kde && editing && !scrolling {
            Layer::Top
        } else {
            Layer::Overlay
        }
    }

    /// 【截图输入法】【层级同步】按 editing、scrolling 更新 surfaces 的浮层层级，无返回值；不重建表面或更改键盘策略
    pub(super) fn sync(
        &mut self,
        editing: bool,
        scrolling: bool,
        surfaces: &HashMap<usize, SurfaceData>,
    ) {
        let target = self.target(editing, scrolling);
        if self.layer == target {
            return;
        }
        self.layer = target;
        // 1. 【截图输入法】【跨屏候选】同步全部截图表面，避免靠近屏幕边缘的候选被相邻截图浮层遮挡
        for data in surfaces.values() {
            if let Role::Layer(layer) = &data.role {
                layer.set_layer(target);
                // 2. 【截图输入法】【映射提交】未映射表面等待正常帧提交，已有表面只提交层级，不复制像素缓冲
                if data.mapped {
                    layer.commit();
                }
            }
        }
        log::info!("【截图输入法】【浮层层级】文字输入状态为 {editing}，截图浮层切换到 {target:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图输入法测试】【桌面别名】无参数，KDE 初始使用 Overlay，仅输入期间使用 Top 并保留禁用动画的命名空间，无返回值
    #[test]
    fn kde_desktop_aliases_lower_only_while_editing() {
        for desktop in ["KDE", "PLASMA", "ubuntu:plasma", "kde:wayland"] {
            let mut policy = Policy::for_desktop(desktop);
            assert_eq!(policy.layer(false), Layer::Overlay);
            assert_eq!(policy.namespace, "utility");
            policy.sync(true, false, &HashMap::new());
            assert_eq!(policy.layer(false), Layer::Top);
            policy.sync(false, false, &HashMap::new());
            assert_eq!(policy.layer(false), Layer::Overlay);
        }
    }

    /// 【截图输入法测试】【其他桌面】无参数，其他桌面或相似名称保持 Overlay 层级及原命名空间，无返回值
    #[test]
    fn other_desktops_keep_overlay_layer() {
        for desktop in ["niri", "GNOME", "not-KDE", "", "KDE-like"] {
            let mut policy = Policy::for_desktop(desktop);
            policy.sync(true, false, &HashMap::new());
            assert_eq!(policy.layer(false), Layer::Overlay);
            assert_eq!(policy.namespace, "mark-shot");
        }
    }

    /// 【截图输入法测试】【滚动控制】无参数，KDE 滚动截图控制仍使用 Overlay，避免影响对外部窗口的交互，无返回值
    #[test]
    fn scrolling_controls_keep_overlay_layer() {
        let mut policy = Policy::for_desktop("KDE");
        policy.sync(true, false, &HashMap::new());
        assert_eq!(policy.layer(true), Layer::Overlay);
        policy.sync(true, true, &HashMap::new());
        assert_eq!(policy.layer(false), Layer::Overlay);
    }
}
