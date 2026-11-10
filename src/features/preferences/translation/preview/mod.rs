//! 【翻译测试】【请求状态】使用正式翻译任务测试当前查看的服务，不改变已启用服务

pub(super) mod draw;

use crate::{
    backend::window::WindowWake,
    config::Translation,
    translation::{Error, ErrorKind, request::Request, task::Task},
};
use std::{path::Path, time::Instant};

pub(super) const SOURCE: &str =
    "Hello! This is a translation test.\nThe meeting starts at 9:30 tomorrow morning.";

/// 【翻译测试】【预制原文】按 target 返回固定示例，英语目标使用中文，便于直观看出翻译结果
pub(super) fn sample(target: &str) -> &'static str {
    if crate::translation::language::canonical(target) == Some("en") {
        "你好！这是一段翻译测试。\n会议将在明天上午 9:30 开始。"
    } else {
        SOURCE
    }
}

#[derive(Default)]
pub(super) enum ResultState {
    #[default]
    Idle,
    Running,
    Success(String),
    Failed(Error),
    Canceled,
}

#[derive(Default)]
pub(super) struct Preview {
    pub result: ResultState,
    task: Option<Task>,
    started: Option<Instant>,
    pub elapsed: Option<f32>,
    pub scroll: usize,
    pub text_height: usize,
    pub drag: Option<crate::ui::scrollbar::Drag>,
}

impl Preview {
    /// 【翻译测试】【执行状态】无参数，返回是否持有当前测试任务，不按计时器轮询
    pub fn busy(&self) -> bool {
        self.task.is_some()
    }

    /// 【翻译测试】【参数快照】按 root 和 provider 生成测试配置，仅覆盖当前查看的服务，不写入配置文件
    fn config(root: &serde_json::Value, provider: &str) -> Result<Translation, Error> {
        let mut config: Translation = serde_json::from_value(root.clone()).map_err(|_| {
            Error::new(
                ErrorKind::Configuration,
                "无法读取当前翻译配置 / Invalid translation configuration",
            )
        })?;
        config.provider = provider.into();
        if provider != "command" {
            config.command.clear();
        }
        config.normalize();
        Ok(config)
    }

    /// 【翻译测试】【主动请求】按 root、provider、path 和 wake 翻译预制原文，结果通过窗口唤醒交付，启动失败立即显示
    pub fn start(
        &mut self,
        root: &serde_json::Value,
        provider: &str,
        path: &Path,
        wake: Option<WindowWake>,
    ) {
        self.reset();
        self.started = Some(Instant::now());
        let task = Self::config(root, provider).and_then(|config| {
            Task::start_request(
                &config,
                Request::text(sample(&config.target_language)),
                &config.target_language,
                path,
                move || {
                    if let Some(wake) = wake {
                        wake.notify();
                    }
                },
            )
        });
        match task {
            Ok(task) => {
                self.task = Some(task);
                self.result = ResultState::Running;
            }
            Err(error) => self.fail(error),
        }
    }

    /// 【翻译测试】【完成交付】无参数，读取一次当前任务结果，返回是否需要重绘，取消后的旧任务不能覆盖新结果
    pub fn poll(&mut self) -> bool {
        let Some(result) = self.task.as_mut().and_then(Task::take_result) else {
            return false;
        };
        self.task = None;
        self.elapsed = self
            .started
            .take()
            .map(|start| start.elapsed().as_secs_f32());
        self.scroll = 0;
        self.result = match result {
            Ok(output) => ResultState::Success(output.text()),
            Err(error) => ResultState::Failed(error),
        };
        true
    }

    /// 【翻译测试】【失败显示】按 error 保留错误类型与脱敏详情，返回空值，不将失败伪装成测试成功
    pub fn fail(&mut self, error: Error) {
        self.task = None;
        self.elapsed = self
            .started
            .take()
            .map(|start| start.elapsed().as_secs_f32());
        self.scroll = 0;
        self.result = ResultState::Failed(error);
    }

    /// 【翻译测试】【取消请求】无参数，仅取消正在运行的任务并保留取消提示，不清除已完成结果
    pub fn cancel(&mut self) {
        if self.task.take().is_some() {
            self.started = None;
            self.elapsed = None;
            self.result = ResultState::Canceled;
            self.scroll = 0;
        }
    }

    /// 【翻译测试】【配置失效】无参数，清理旧任务及结果，防止服务、模型或参数变化后继续展示旧测试结论
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 【翻译测试】【可复制结果】无参数，返回是否有译文或错误详情
    pub fn copyable(&self) -> bool {
        matches!(
            self.result,
            ResultState::Success(_) | ResultState::Failed(_)
        )
    }

    /// 【翻译测试】【结果正文】按 chinese 返回译文、状态或错误类型与完整脱敏详情，不输出配置内容
    pub fn text(&self, chinese: bool) -> String {
        match &self.result {
            ResultState::Idle => if chinese {
                "点击“测试翻译”，使用当前查看的服务翻译上方原文。"
            } else {
                "Test the displayed text with the service you are viewing."
            }
            .into(),
            ResultState::Running => if chinese {
                "正在请求翻译服务…"
            } else {
                "Requesting translation…"
            }
            .into(),
            ResultState::Canceled => if chinese {
                "测试已取消，可重新测试。"
            } else {
                "Test canceled. You can try again."
            }
            .into(),
            ResultState::Success(text) => text.clone(),
            ResultState::Failed(error) => {
                let (cn, en) = match error.kind {
                    ErrorKind::Configuration => ("配置错误", "Configuration error"),
                    ErrorKind::Network => ("网络连接失败", "Network error"),
                    ErrorKind::Http => ("HTTP 请求失败", "HTTP error"),
                    ErrorKind::Api => ("服务接口错误", "API error"),
                    ErrorKind::Protocol => ("响应解析失败", "Invalid response"),
                    ErrorKind::Timeout => ("请求超时", "Request timed out"),
                    ErrorKind::Canceled => ("请求已取消", "Request canceled"),
                    ErrorKind::StartFailed => ("翻译任务启动失败", "Unable to start translation"),
                };
                format!("{}\n{}", if chinese { cn } else { en }, error.detail)
            }
        }
    }
}

#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;
