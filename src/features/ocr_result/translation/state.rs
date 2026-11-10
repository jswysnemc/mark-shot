//! 【OCR 翻译】【结果身份】以完整确认原文和目标语言判断新鲜度，不覆盖继续编辑的原文

use crate::translation::{Error, Output};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Notice {
    None,
    Busy,
    Canceled,
    Stale,
    Failed(String),
}

impl Notice {
    /// 【OCR 翻译】【提示文字】返回当前完整提示，无参数，长错误由绘制层限制可见行数
    pub fn text(&self) -> &str {
        match self {
            Self::None => "",
            Self::Busy => "Translating...",
            Self::Canceled => "Translation canceled",
            Self::Stale => "Source or language changed. Translate again to update.",
            Self::Failed(text) => text,
        }
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    source: String,
    target: String,
}

#[derive(Default)]
pub(crate) struct State {
    pub visible: bool,
    pub notice: Option<Notice>,
    pending: Option<(u64, Snapshot)>,
    completed: Option<Snapshot>,
    failed: bool,
}

impl State {
    /// 【翻译浮窗】【已有译文】按 source 和 target 恢复已完成快照，无返回值，不产生网络任务
    pub fn restore(&mut self, source: &str, target: &str) {
        self.visible = true;
        self.completed = Some(Snapshot {
            source: source.trim().into(),
            target: target.trim().into(),
        });
        self.pending = None;
        self.notice = None;
        self.failed = false;
    }
    /// 【OCR 翻译】【任务资格】返回是否存在尚未完成的当前所属任务，无参数，不依赖工作线程状态推测完成
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// 【OCR 翻译】【完整提示】返回当前提示，无参数，空状态不分配默认字符串
    pub fn notice(&self) -> &str {
        self.notice.as_ref().map_or("", Notice::text)
    }

    /// 【OCR 翻译】【语言提示】按 language 返回当前提示，原始服务名和未知错误详情保持原样，已知错误采用原版文案
    pub fn localized_notice(
        &self,
        language: crate::ui::language::Language,
    ) -> std::borrow::Cow<'_, str> {
        let raw = self.notice();
        if language == crate::ui::language::Language::Chinese
            && let Some(detail) = raw.strip_prefix("Translation failed (")
            && let Some((provider, detail)) = detail.split_once("): ")
        {
            let provider = if provider == "unknown provider" {
                language.text(provider)
            } else {
                provider
            };
            let detail = match detail {
                "timed out"
                | "provider failed to start"
                | "provider failed"
                | "no translation result" => language.text(detail),
                _ => detail,
            };
            return std::borrow::Cow::Owned(
                language.format("Translation failed (%1): %2", &[provider, detail]),
            );
        }
        std::borrow::Cow::Borrowed(language.text(raw))
    }

    /// 【OCR 翻译】【错误外观】返回当前提示是否为失败，无参数，不把取消或过期显示为错误
    pub fn error(&self) -> bool {
        matches!(self.notice, Some(Notice::Failed(_)))
    }

    /// 【OCR 翻译】【任务开始】按 id、source 和 target 保存确认快照，返回是否接受；同一窗口不会并行覆盖已有任务
    pub fn begin(&mut self, id: u64, source: &str, target: &str) -> bool {
        if self.busy() || source.trim().is_empty() || target.trim().is_empty() {
            return false;
        }
        self.visible = true;
        self.pending = Some((
            id,
            Snapshot {
                source: source.trim().into(),
                target: target.trim().into(),
            },
        ));
        self.failed = false;
        self.notice = Some(Notice::Busy);
        true
    }

    /// 【OCR 翻译】【新鲜度】按 source 和 target 更新空闲成功结果的提示，返回是否变化；忙碌和失败提示保持原样
    pub fn refresh(&mut self, source: &str, target: &str) -> bool {
        if self.busy() || self.failed {
            return false;
        }
        let Some(previous) = &self.completed else {
            return false;
        };
        let notice = if source.trim() != previous.source || target.trim() != previous.target {
            Notice::Stale
        } else {
            Notice::None
        };
        if self.notice.as_ref() == Some(&notice) {
            return false;
        }
        self.notice = Some(notice);
        true
    }

    /// 【OCR 翻译】【失败保留】按 error 显示当前服务和失败原因，无返回值，已有成功快照及编辑过的译文不变
    pub fn fail(&mut self, error: Error) {
        self.visible = true;
        self.pending = None;
        self.failed = true;
        let provider = error
            .provider
            .map_or("unknown provider", |provider| provider.id());
        self.notice = Some(Notice::Failed(format!(
            "Translation failed ({provider}): {}",
            error.detail
        )));
    }

    /// 【OCR 翻译】【一次完成】按 id、result、当前 source 和 target 接受所属结果，返回需要替换的译文，旧身份无副作用
    pub fn finish(
        &mut self,
        id: u64,
        result: Result<Output, Error>,
        source: &str,
        target: &str,
    ) -> Option<String> {
        if self.pending.as_ref().map(|(pending, _)| *pending) != Some(id) {
            return None;
        }
        let (_, snapshot) = self.pending.take().unwrap();
        match result {
            Ok(output) => {
                let text = output.text();
                if text.is_empty() {
                    let mut error = Error::new(
                        crate::translation::ErrorKind::Protocol,
                        "no translation result",
                    );
                    error.provider = Some(output.provider);
                    self.fail(error);
                    return None;
                }
                self.completed = Some(snapshot);
                self.failed = false;
                self.notice = Some(Notice::None);
                self.refresh(source, target);
                Some(text)
            }
            Err(error) => {
                self.fail(error);
                None
            }
        }
    }

    /// 【OCR 翻译】【主动取消】按 source、target 和 explicit 清理任务，返回原所属编号；明确取消显示提示，收起只保留既有状态
    pub fn cancel(&mut self, source: &str, target: &str, explicit: bool) -> Option<u64> {
        let id = self.pending.take().map(|(id, _)| id);
        self.refresh(source, target);
        if explicit {
            self.notice = Some(Notice::Canceled);
        }
        id
    }
}

#[cfg(test)]
mod tests;
