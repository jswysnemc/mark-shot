//! 【录屏工具】【会话状态】持有当前参数、短菜单与明确提交的录制请求

use super::tool_options::Parameter;
use crate::{config::recording::Recording, types::MouseButton, ui::controls::Choice};

pub(crate) struct Menu {
    pub parameter: Parameter,
    pub monitor: usize,
    pub choice: Choice,
}

pub(crate) struct ToolState {
    pub config: Recording,
    pub menu: Option<Menu>,
    pub release_owner: Option<MouseButton>,
    pub request: Option<Recording>,
}

impl Default for ToolState {
    /// 【录屏工具】【初始参数】无参数，复制当前录制偏好；菜单与开始请求均为空
    fn default() -> Self {
        Self {
            config: crate::config::preferences::current().recording.clone(),
            menu: None,
            release_owner: None,
            request: None,
        }
    }
}
