//! 【覆盖层轻提示】【胶囊绘制】使用统一微交互徽标组件绘制淡入淡出的语义提示胶囊

use crate::{
    renderer::Painter,
    ui::{
        micro_badge::{self, BadgeKind},
        toast::{ToastKind, Toasts},
    },
};
use tiny_skia::Rect;

/// 【覆盖层轻提示】【语义分类】按提示种类与文本语义映射对应的胶囊状态点类型
pub(crate) fn kind_for(kind: ToastKind, text: &str) -> BadgeKind {
    match kind {
        ToastKind::ColorCopied => BadgeKind::Success,
        ToastKind::ScanFailed
        | ToastKind::SettingsFailed
        | ToastKind::SaveAsFailed
        | ToastKind::ExternalActionFailed
        | ToastKind::HistoryFailed
        | ToastKind::OcrDownloadFailed
        | ToastKind::OcrFailed
        | ToastKind::CopyFailed => BadgeKind::Error,
        ToastKind::SelectionTooSmall | ToastKind::OcrNoText | ToastKind::NumberExhausted => {
            BadgeKind::Notice
        }
        ToastKind::UploadFeedback | ToastKind::TranslationFeedback => {
            let lower = text.to_ascii_lowercase();
            if lower.contains("copied") || text.contains("已复制") {
                BadgeKind::Success
            } else if lower.contains("fail")
                || lower.contains("timeout")
                || lower.contains("timed out")
                || text.contains("失败")
                || text.contains("超时")
            {
                BadgeKind::Error
            } else {
                BadgeKind::Notice
            }
        }
        ToastKind::RulerMode
        | ToastKind::OcrPickRegion
        | ToastKind::OcrNoModel
        | ToastKind::PickColor
        | ToastKind::ScrollNeedsRegion
        | ToastKind::RecordingFeedback => BadgeKind::Neutral,
    }
}

/// 【覆盖层轻提示】【批量绘制】在局部脏区 clip 交集内绘制当前显示器的全部活跃提示胶囊，无返回值
pub fn draw_toasts(p: &mut Painter, toasts: &Toasts, monitor_idx: usize, clip: Option<&Rect>) {
    for toast in &toasts.items {
        if toast.monitor_idx != monitor_idx {
            continue;
        }
        let Some(rect) = toast.rect else { continue };
        let opacity = toast.opacity();
        if opacity <= 0.0 {
            continue;
        }
        if let Some(clip) = clip
            && (clip.left() >= rect.right()
                || clip.right() <= rect.left()
                || clip.top() >= rect.bottom()
                || clip.bottom() <= rect.top())
        {
            continue;
        }
        let badge_kind = kind_for(toast.kind, toast.text.as_ref());
        micro_badge::draw(p, rect, badge_kind, toast.text.as_ref(), "", opacity);
    }
}
