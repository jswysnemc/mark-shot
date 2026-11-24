//! 【贴图微反馈】【双通道状态胶囊】分离后台识别翻译任务胶囊与滚轮倍率、复制、旋转、置顶等瞬时微提示，不修改原图字节

use crate::{
    renderer::Painter,
    ui::{
        language::Language,
        micro_badge::{self, BadgeKind},
    },
};
use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

#[derive(Clone)]
struct Entry {
    kind: BadgeKind,
    title: String,
    detail: String,
    deadline: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct Feedback {
    task: Option<Entry>,
    hud: Option<Entry>,
}

impl Feedback {
    /// 【贴图微反馈】【开始识别】显示持续文字识别状态，直至识别结果返回或被取消
    pub fn recognizing(&mut self) {
        self.set_task(
            BadgeKind::Busy,
            "正在识别文字…",
            "Recognizing text...",
            "",
            None,
        );
    }

    /// 【贴图微反馈】【开始翻译】按 target 语言显示本地化翻译进行中胶囊，网络请求期间持续可见
    pub fn translating(&mut self, target: &str) {
        let language = Language::current();
        let display = crate::translation::language::display_for(target, language);
        let title = if language == Language::Chinese {
            format!("正在翻译 · {display}")
        } else {
            format!("Translating · {display}")
        };
        self.task = Some(Entry {
            kind: BadgeKind::Busy,
            title,
            detail: String::new(),
            deadline: None,
        });
    }

    /// 【贴图微反馈】【翻译完成】按 target 语言显示限时完成胶囊，到期后自动恢复无遮挡贴图
    pub fn translated(&mut self, target: &str) {
        let language = Language::current();
        let display = crate::translation::language::display_for(target, language);
        let title = if language == Language::Chinese {
            format!("翻译完成 · {display}")
        } else {
            format!("Translated · {display}")
        };
        self.task = Some(Entry {
            kind: BadgeKind::Success,
            title,
            detail: String::new(),
            deadline: Some(Instant::now() + Duration::from_millis(2200)),
        });
    }

    /// 【贴图微反馈】【空识别结果】提示当前图像未检测到可复制或可选择的文字，限时自动消失
    pub fn no_text(&mut self) {
        self.set_task(
            BadgeKind::Notice,
            "未识别到文字",
            "No text found",
            "",
            Some(Duration::from_millis(1800)),
        );
    }

    /// 【贴图微反馈】【失败提示】按 detail 展示错误原因，保留有限时长后自动收起避免永久遮挡贴图
    pub fn error(&mut self, detail: &str) {
        self.set_task(
            BadgeKind::Error,
            "操作失败",
            "Action failed",
            detail,
            Some(Duration::from_millis(4200)),
        );
    }

    /// 【贴图微反馈】【任务清除】清除后台识别与翻译任务状态，保留瞬时缩放或操作微提示
    pub fn clear(&mut self) {
        self.task = None;
    }

    /// 【贴图微反馈】【缩放百分比】在滚轮或菜单缩放时显示紧凑倍率胶囊，不覆盖正在进行的识别或翻译状态
    pub fn zoom(&mut self, ratio: f32) {
        let percent = (ratio.clamp(0.05, 99.0) * 100.0).round() as i32;
        self.hud = Some(Entry {
            kind: BadgeKind::Zoom,
            title: format!("{percent}%"),
            detail: String::new(),
            deadline: Some(Instant::now() + Duration::from_millis(900)),
        });
    }

    /// 【贴图微反馈】【复制图片】在复制原图到剪贴板后展示简短完成胶囊
    pub fn copied_image(&mut self) {
        self.set_hud(
            BadgeKind::Success,
            "已复制图片",
            "Image copied",
            Duration::from_millis(1200),
        );
    }

    /// 【贴图微反馈】【复制文字】在复制识别文字或选中文本后展示简短完成胶囊
    pub fn copied_text(&mut self) {
        self.set_hud(
            BadgeKind::Success,
            "已复制文字",
            "Text copied",
            Duration::from_millis(1200),
        );
    }

    /// 【贴图微反馈】【旋转角度】按顺逆时针方向显示旋转微提示
    pub fn rotated(&mut self, clockwise: bool) {
        if clockwise {
            self.set_hud(
                BadgeKind::Neutral,
                "顺时针 90°",
                "Rotated +90°",
                Duration::from_millis(900),
            );
        } else {
            self.set_hud(
                BadgeKind::Neutral,
                "逆时针 90°",
                "Rotated -90°",
                Duration::from_millis(900),
            );
        }
    }

    /// 【贴图微反馈】【置顶切换】按 enabled 显示窗口置顶状态变更微提示
    pub fn topmost(&mut self, enabled: bool) {
        if enabled {
            self.set_hud(
                BadgeKind::Success,
                "已开启置顶",
                "Pinned on top",
                Duration::from_millis(1000),
            );
        } else {
            self.set_hud(
                BadgeKind::Neutral,
                "已取消置顶",
                "Unpinned",
                Duration::from_millis(1000),
            );
        }
    }

    /// 【贴图微反馈】【译文切换】按 active 提示当前显示译文或原文
    pub fn translation_view(&mut self, active: bool) {
        if active {
            self.set_hud(
                BadgeKind::Neutral,
                "显示译文",
                "Translated view",
                Duration::from_millis(1000),
            );
        } else {
            self.set_hud(
                BadgeKind::Neutral,
                "显示原文",
                "Original view",
                Duration::from_millis(1000),
            );
        }
    }

    /// 【贴图微反馈】【文字选择切换】按 enabled 提示划词选择模式状态
    pub fn text_selection(&mut self, enabled: bool) {
        if enabled {
            self.set_hud(
                BadgeKind::Success,
                "可拖拽选词",
                "Text selection on",
                Duration::from_millis(1100),
            );
        } else {
            self.set_hud(
                BadgeKind::Neutral,
                "已退出选词",
                "Text selection off",
                Duration::from_millis(900),
            );
        }
    }

    /// 【贴图微反馈】【最近期限】返回任务胶囊与瞬时微提示中的最早到期时刻，空闲时返回空值避免轮询
    pub fn deadline(&self) -> Option<Instant> {
        [
            self.task.as_ref().and_then(|e| e.deadline),
            self.hud.as_ref().and_then(|e| e.deadline),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// 【贴图微反馈】【到期回收】按当前时钟清理已到期通道，返回是否有提示消失并需要重绘
    pub fn expire(&mut self) -> bool {
        self.expire_at(Instant::now())
    }

    /// 【贴图微反馈】【指定时刻回收】在 now 时刻分别检查任务通道与瞬时通道，互不误删未到期状态
    fn expire_at(&mut self, now: Instant) -> bool {
        let mut changed = false;
        if self
            .task
            .as_ref()
            .and_then(|e| e.deadline)
            .is_some_and(|deadline| now >= deadline)
        {
            self.task = None;
            changed = true;
        }
        if self
            .hud
            .as_ref()
            .and_then(|e| e.deadline)
            .is_some_and(|deadline| now >= deadline)
        {
            self.hud = None;
            changed = true;
        }
        changed
    }

    /// 【贴图微反馈】【写入任务态】按当前界面语言构造持久或限时任务状态记录，无返回值
    fn set_task(
        &mut self,
        kind: BadgeKind,
        zh: &str,
        en: &str,
        detail: &str,
        duration: Option<Duration>,
    ) {
        let title = if Language::current() == Language::Chinese {
            zh
        } else {
            en
        };
        self.task = Some(Entry {
            kind,
            title: title.into(),
            detail: detail.replace(['\r', '\n', '\t'], " "),
            deadline: duration.map(|d| Instant::now() + d),
        });
    }

    /// 【贴图微反馈】【写入瞬时态】按当前界面语言构造限时微交互胶囊，无返回值
    fn set_hud(&mut self, kind: BadgeKind, zh: &str, en: &str, duration: Duration) {
        let title = if Language::current() == Language::Chinese {
            zh
        } else {
            en
        };
        self.hud = Some(Entry {
            kind,
            title: title.into(),
            detail: String::new(),
            deadline: Some(Instant::now() + duration),
        });
    }

    /// 【贴图微反馈】【双层绘制】在贴图预览左上角绘制缩放倍率、右上角绘制其他瞬时胶囊，在底部居中绘制识别或翻译状态胶囊
    pub fn draw(
        &self,
        canvas: &mut Pixmap,
        scale: f32,
        fonts: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
    ) {
        if self.task.is_none() && self.hud.is_none() {
            return;
        }
        let scale = scale.max(0.1);
        let width = canvas.width() as f32 / scale;
        let height = canvas.height() as f32 / scale;
        if width < 36.0 || height < 24.0 {
            return;
        }

        let max_w = (width - 12.0).clamp(32.0, 320.0);
        // 1. 先测量两个通道胶囊的紧凑尺寸，避免可变借用冲突
        let hud_box = self.hud.as_ref().map(|entry| {
            let (w, h) = micro_badge::measure(fonts, &entry.title, "", entry.kind, max_w);
            let h = h.min((height - 6.0).max(16.0));
            let x = if entry.kind == BadgeKind::Zoom {
                hud_left(width, w)
            } else {
                hud_right(width, w)
            };
            let y = hud_top(height, h);
            (entry, Rect::from_xywh(x, y, w, h))
        });

        let task_box = self.task.as_ref().and_then(|entry| {
            if height < 34.0 && hud_box.is_some() {
                return None;
            }
            let detail = if height < 68.0 {
                ""
            } else {
                entry.detail.as_str()
            };
            let (w, h) = micro_badge::measure(fonts, &entry.title, detail, entry.kind, max_w);
            let h = h.min((height - 8.0).max(18.0));
            let x = ((width - w) * 0.5).max(6.0);
            let y = (height - h - 8.0).max(4.0);
            Some((entry, detail, Rect::from_xywh(x, y, w, h)))
        });

        // 2. 依次绘制底部任务胶囊与顶部瞬时微提示，不污染独立原图缓冲
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(fonts, cache);
        if let Some((entry, detail, Some(rect))) = task_box {
            micro_badge::draw(&mut painter, rect, entry.kind, &entry.title, detail, 1.0);
        }
        if let Some((entry, Some(rect))) = hud_box {
            micro_badge::draw(&mut painter, rect, entry.kind, &entry.title, "", 1.0);
        }
    }

    /// 【贴图微反馈】【提示读取】返回当前瞬时胶囊标题，测试据此确认缩放百分比确实进入显示通道
    #[cfg(test)]
    pub(crate) fn hud_title(&self) -> Option<&str> {
        self.hud.as_ref().map(|entry| entry.title.as_str())
    }

    /// 【贴图微反馈】【任务读取】返回当前底部任务胶囊标题，测试据此确认空识别或任务提示进入显示通道
    #[cfg(test)]
    pub(crate) fn task_title(&self) -> Option<&str> {
        self.task.as_ref().map(|entry| entry.title.as_str())
    }
}

/// 【贴图微反馈】【左上锚点】按可见宽度与胶囊宽度返回贴左上角的横坐标，窄窗口收拢避免溢出
fn hud_left(width: f32, pill_width: f32) -> f32 {
    8.0_f32.min((width - pill_width - 3.0).max(3.0))
}

/// 【贴图微反馈】【右上锚点】按可见宽度与胶囊宽度返回贴右上角的横坐标，窄窗口居中
fn hud_right(width: f32, pill_width: f32) -> f32 {
    if width < 96.0 {
        ((width - pill_width) * 0.5).max(3.0)
    } else {
        (width - pill_width - 8.0).max(6.0)
    }
}

/// 【贴图微反馈】【顶部锚点】按可见高度与胶囊高度返回顶部纵坐标，过矮窗口垂直居中
fn hud_top(height: f32, pill_height: f32) -> f32 {
    if height < 44.0 {
        ((height - pill_height) * 0.5).max(2.0)
    } else {
        8.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【贴图微反馈测试】【双通道隔离】验证识别或翻译期间触发滚轮缩放时两个胶囊共存且各自按独立期限回收
    #[test]
    fn zoom_hud_coexists_with_ongoing_translation_and_expires_independently() {
        let mut feedback = Feedback::default();
        feedback.translating("Simplified Chinese");
        assert!(feedback.deadline().is_none());

        feedback.zoom(1.25);
        let zoom_deadline = feedback
            .deadline()
            .expect("zoom pill sets one-shot deadline");
        assert_eq!(feedback.hud.as_ref().unwrap().title, "125%");
        assert!(feedback.task.is_some());

        assert!(feedback.expire_at(zoom_deadline + Duration::from_millis(1)));
        assert!(feedback.hud.is_none());
        assert!(
            feedback.task.is_some(),
            "expiring zoom HUD must not clear active translation status"
        );
        assert!(feedback.deadline().is_none());

        feedback.clear();
        assert!(feedback.task.is_none());
    }

    /// 【贴图微反馈测试】【左上角定位】验证倍率胶囊贴左上角、窄窗口收拢，其他瞬时提示仍贴右上角
    #[test]
    fn zoom_pill_anchors_at_the_top_left_corner() {
        assert_eq!(hud_left(320.0, 48.0), 8.0);
        assert_eq!(hud_top(200.0, 26.0), 8.0);
        assert_eq!(hud_left(40.0, 44.0), 3.0);
        assert_eq!(hud_top(30.0, 26.0), 2.0);
        assert_eq!(hud_right(320.0, 48.0), 264.0);
    }
}
