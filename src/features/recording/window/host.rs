//! 【录制会话】【进程宿主】IPC 服务与录制控制器脱离窗口存活，界面关闭后录制继续

use super::super::{Status, Target, controller::Controller, ipc::Server};
use crate::config::recording::Recording;
use std::{cell::RefCell, rc::Rc};

/// 会话宿主的共享句柄，准备窗口在同一线程内读写
pub(super) type Shared = Rc<RefCell<Host>>;

pub(super) struct Host {
    server: Server,
    controller: Option<Controller>,
    target: Target,
    request: Option<Recording>,
}

impl Host {
    /// 【录制会话】【建立宿主】按 target 取得独占文件锁并启动本地命令服务，返回占用或资源错误
    pub(super) fn new(target: Target) -> crate::AppResult<Self> {
        Ok(Self {
            server: Server::new()?,
            controller: None,
            target,
            request: None,
        })
    }

    /// 【录制会话】【当前目标】无参数，返回本次录制目标，供准备窗口编辑并写回
    pub(super) fn target(&self) -> &Target {
        &self.target
    }

    /// 【录制会话】【更新目标】按 target 替换本次录制目标，无返回值；不写入用户配置
    pub(super) fn set_target(&mut self, target: Target) {
        self.target = target;
    }

    /// 【录制会话】【开始请求】按 config 记录用户确认的选项，准备窗口关闭后由流程读取，无返回值
    pub(super) fn request(&mut self, config: Recording) {
        self.request = Some(config);
    }

    /// 【录制会话】【领取请求】无参数，取出用户确认的选项，没有请求表示用户关闭了准备窗口
    pub(super) fn take_request(&mut self) -> Option<Recording> {
        self.request.take()
    }

    /// 【录制会话】【开始录制】按 config 启动工作线程并绑定命令服务，返回配置或输出错误
    ///
    /// 倒计时已由小组件在界面上完成，工作线程立即进入采集，保证小组件不出现在录制画面中
    pub(super) fn start(&mut self, mut config: Recording) -> Result<(), String> {
        config.countdown = 0;
        // 1. 【录制会话】【旧会话回收】上一轮已结束的控制器在此释放，避免线程句柄堆积
        self.controller = None;
        let controller = Controller::start(config, self.target.clone())?;
        self.server.attach(&controller);
        self.controller = Some(controller);
        Ok(())
    }

    /// 【录制会话】【活动判断】无参数，返回设备或导出线程是否仍在工作
    pub(super) fn active(&self) -> bool {
        self.controller
            .as_ref()
            .is_some_and(|controller| !controller.finished())
    }

    /// 【录制会话】【状态快照】无参数，返回最新录制状态，没有会话时返回空闲状态
    pub(super) fn snapshot(&self) -> Status {
        self.controller
            .as_ref()
            .map_or_else(Status::default, |controller| controller.snapshot())
    }

    /// 【录制会话】【会话重置】无参数，释放已结束的控制器，保留目标与命令服务
    pub(super) fn reset(&mut self) {
        self.controller = None;
    }
}
