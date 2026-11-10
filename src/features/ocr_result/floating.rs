//! 【翻译浮窗】【独立交接】复用文本编辑、目标语言和翻译任务，首帧就绪后交还截图资源

use super::{document::OcrDocument, window::ResultWindow};
use crate::{
    AppResult,
    backend::window::{self, WindowContent, WindowEvent, WindowSpec},
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const ARG: &str = "--translation-window";
const LIMIT: usize = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Header {
    target: String,
    translated: String,
}

/// 【翻译浮窗】【父进程交接】按 document、target、translated 发送原文及已有译文，返回首帧确认结果，不重复请求已完成翻译
pub fn spawn(document: &OcrDocument, target: &str, translated: &str) -> AppResult {
    let header = serde_json::to_vec(&Header {
        target: target.into(),
        translated: translated.into(),
    })?;
    if header.len() > LIMIT {
        return Err("translation window content exceeds the transport limit".into());
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&header);
    super::transport::write(document, &mut bytes)?;
    crate::utils::spawn_self_ready(&[ARG], &bytes, window::READY_TIMEOUT)
}

/// 【翻译浮窗】【窗口构造】按 document、target、translated 恢复独立编辑状态，返回紧凑置顶窗口，未翻译时等待窗口连接
fn content(document: OcrDocument, target: &str, translated: &str) -> AppResult<ResultWindow> {
    let mut content = ResultWindow::new(document)?;
    content.stacking.active = true;
    content.translation.ratio = 0.45;
    content.translation.saved_target = target.into();
    content.translation.config.target_language = target.into();
    let label = crate::translation::language::display_for(target, content.language);
    content.translation.language = crate::ui::text_editor::TextEditor::new_single_line_with_style(
        label,
        &mut content.fonts,
        super::translation::style(true),
    );
    content.translation.state.visible = true;
    content.translate_on_connect = translated.trim().is_empty();
    if !content.translate_on_connect {
        content.ensure_fonts(translated);
        content.translation.editor = crate::ui::text_editor::TextEditor::new_with_style(
            translated,
            &mut content.fonts,
            super::translation::style(false),
        );
        content
            .translation
            .state
            .restore(&content.source.text(), target);
    }
    content.event(WindowEvent::Resize { size: (520, 380) });
    Ok(content)
}

/// 【翻译浮窗】【独立入口】按 args 校验内部命令并读取有界原文、译文和图像，返回窗口生命周期结果
pub fn cli(args: &[String]) -> AppResult {
    let result = (|| {
        if !args.is_empty() {
            return Err("translation window accepts no additional arguments".into());
        }
        let mut reader = std::io::stdin().lock();
        let mut length = [0; 4];
        reader.read_exact(&mut length)?;
        let length = u32::from_le_bytes(length) as usize;
        if length == 0 || length > LIMIT {
            return Err("invalid translation window header".into());
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;
        let header: Header = serde_json::from_slice(&bytes)?;
        let document = super::transport::read(reader)?;
        let content = content(document, &header.target, &header.translated)?;
        window::run(
            WindowSpec {
                title: content.language.text("Translation window").into(),
                app_id: "mark-shot-translation".into(),
                size: (520, 380),
                minimum: super::layout::MINIMUM_SIZE,
            },
            content,
        )
    })();
    if let Err(error) = &result {
        let _ = writeln!(std::io::stdout().lock(), "{error}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【翻译浮窗测试】【缓存交接】无参数，已有译文及目标保持原样，小窗初始化不启动网络任务
    #[test]
    fn restored_translation_opens_compact_and_does_not_request_again() {
        let document = OcrDocument {
            text: "Original text".into(),
            png: crate::utils::encode_png(&tiny_skia::Pixmap::new(100, 40).unwrap()),
            lines: Vec::new(),
        };
        let content = content(document, "Japanese", "元のテキスト").unwrap();
        assert_eq!(content.source.text(), "Original text");
        assert_eq!(content.translation.editor.text(), "元のテキスト");
        assert_eq!(content.translation.target(), "Japanese");
        assert_eq!(content.layout.size, (520.0, 380.0));
        assert!(content.translation.state.visible && content.stacking.active);
        assert!(!content.translate_on_connect);
        assert!(content.translation.task.is_none());
        assert!(content.translation.state.notice().is_empty());
    }
}
