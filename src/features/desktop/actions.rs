//! 【桌面集成】【菜单动作】托盘和全局按键共用动作定义，窗口启动与延时控制分别处理

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Capture,
    Fullscreen,
    Pin,
    Delay(u64),
    CancelDelay,
    Settings,
    History,
    Record,
    PauseRecording,
    ResumeRecording,
    StopRecording,
    Quit,
}

impl Action {
    /// 【桌面集成】【启动参数】返回当前动作对应的命令行参数，延时与退出动作由常驻服务处理
    pub fn arguments(self) -> Option<&'static [&'static str]> {
        match self {
            Self::Capture => Some(&["--capture"]),
            Self::Fullscreen => Some(&["--fullscreen"]),
            Self::Pin => Some(&["--pin-clipboard"]),
            Self::Settings => Some(&["--settings"]),
            Self::History => Some(&["--history"]),
            Self::Record => Some(&["--record"]),
            Self::PauseRecording => Some(&["--pause-recording"]),
            Self::ResumeRecording => Some(&["--resume-recording"]),
            Self::StopRecording => Some(&["--stop-recording"]),
            Self::Delay(_) | Self::CancelDelay | Self::Quit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::arguments::{Command, parse};

    /// 【桌面集成测试】【真实入口】无参数，验证菜单启动现有业务入口，全屏使用标注模式而非直接导出
    #[test]
    fn tray_actions_parse_into_expected_commands() {
        let command = |action: Action| {
            parse(
                action.arguments().unwrap().iter().map(|arg| (*arg).into()),
                std::time::Instant::now(),
            )
            .unwrap()
            .command
        };
        assert!(
            matches!(command(Action::Capture), Command::Editor(launch) if !launch.fullscreen && launch.mode == crate::types::Mode::Editor)
        );
        assert!(
            matches!(command(Action::Fullscreen), Command::Editor(launch) if launch.fullscreen && launch.mode == crate::types::Mode::Editor)
        );
        assert!(matches!(command(Action::Pin), Command::PinClipboard));
        assert!(matches!(command(Action::Settings), Command::Settings));
        assert!(matches!(command(Action::History), Command::History));
    }
}
