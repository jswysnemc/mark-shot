//! 【识别安装】【控件绘制】引擎和推理库复用统一按钮状态与前景颜色

use super::{ModelPopover, ModelPopoverElement, cfg, setup::rect};
use crate::{
    ocr::{engine_choice::Engine, models::ModelStatus},
    renderer::{Painter, text::TextStyle},
    theme::{color, font},
    ui::icons,
};
use tiny_skia::Rect;

/// 【识别安装】【控件绘制】按 panel 绘制引擎选项与独立下载行，p 为共享主题画笔，无返回值
pub fn draw(p: &mut Painter, panel: &ModelPopover) {
    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let count = super::layout::setup_rows(panel);
    for index in 0..count {
        let (element, title, note, selected) = match (cfg!(target_os = "windows"), index) {
            (true, 0) => (
                ModelPopoverElement::SystemEngine,
                if chinese {
                    "Windows 系统 OCR"
                } else {
                    "Windows OCR"
                },
                if chinese {
                    "使用系统语言包，适合简单单语言文本"
                } else {
                    "Installed language packs; simple single-language text"
                },
                panel.setup.engine == Engine::System,
            ),
            (true, 1) => (
                ModelPopoverElement::PaddleEngine,
                if chinese {
                    "本地模型 OCR（推荐）"
                } else {
                    "Local model OCR (recommended)"
                },
                if chinese {
                    "手动下载推理库和语言模型"
                } else {
                    "Download runtime and language model manually"
                },
                panel.setup.engine == Engine::Paddle,
            ),
            _ => (
                ModelPopoverElement::Runtime,
                if chinese {
                    "推理库 · ONNX Runtime"
                } else {
                    "Runtime · ONNX Runtime"
                },
                if chinese {
                    "下载一次，安装后可离线识别"
                } else {
                    "Install once for offline recognition"
                },
                panel.setup.runtime == ModelStatus::Installed,
            ),
        };
        let area = rect(panel, (0.0, 0.0), index);
        let hovered = panel.hovered == Some(element);
        let foreground = super::draw::control(p, area, selected, hovered);
        let secondary = if selected || hovered {
            foreground
        } else {
            color::muted()
        };
        let left = area.left() + cfg().row_padding;
        let width =
            (area.width() - cfg().row_padding * 2.0 - cfg().button_size - cfg().status_width)
                .max(1.0);
        p.text(
            Rect::from_xywh(left, area.top() + cfg().name_top, width, cfg().name_height).unwrap(),
            title,
            TextStyle::new(font::label(), foreground),
        );
        p.text(
            Rect::from_xywh(
                left,
                area.top() + cfg().note_top,
                area.width() - cfg().row_padding * 2.0,
                cfg().note_height,
            )
            .unwrap(),
            note,
            TextStyle::new(font::small(), secondary),
        );
        let icon = if element != ModelPopoverElement::Runtime {
            if selected { icons::CHECK } else { icons::OCR }
        } else {
            match panel.setup.runtime {
                ModelStatus::Installed => icons::CHECK,
                ModelStatus::Queued | ModelStatus::Downloading(_) => icons::CLOSE,
                ModelStatus::Failed => icons::RETRY,
                _ => icons::DOWNLOAD,
            }
        };
        p.icon(
            icon,
            cfg().icon_size,
            area.right() - cfg().row_padding - cfg().icon_size,
            area.top() + (area.height() - cfg().icon_size) / 2.0,
            foreground,
        );
        if element == ModelPopoverElement::Runtime {
            let status = match panel.setup.runtime {
                ModelStatus::Downloading(percent) => format!("{percent}%"),
                ModelStatus::Queued => "…".into(),
                ModelStatus::Installed => if chinese { "已安装" } else { "Installed" }.into(),
                _ => crate::ocr::runtime_library::manifest::current()
                    .map(|dist| format!("{:.1} MB", dist.archive.size as f64 / 1_000_000.0))
                    .unwrap_or_else(|| "Unavailable".into()),
            };
            p.text(
                Rect::from_xywh(
                    area.right() - cfg().button_size - cfg().status_width - cfg().row_padding,
                    area.top(),
                    cfg().status_width,
                    cfg().row_height,
                )
                .unwrap(),
                &status,
                TextStyle::new(font::small(), secondary).center(),
            );
        }
    }
}
