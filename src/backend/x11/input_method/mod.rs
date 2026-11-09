//! 【X11输入法】【协议会话】按正文身份创建XIM上下文，失焦销毁旧上下文以隔离延迟结果

mod handler;
mod preedit;

use super::connection::Display;
use crate::types::text_input::{Edit, Snapshot};
use std::{collections::VecDeque, sync::Arc};
use x11rb::{
    protocol::{Event, xproto::*},
    xcb_ffi::XCBConnection,
};
use xim::{Client as _, x11rb::X11rbClient};

type Client = X11rbClient<Arc<XCBConnection>>;

pub(super) enum Message {
    Key { target: u64, event: KeyPressEvent },
    Edit { target: u64, edit: Edit },
}

#[derive(Default)]
struct Handler {
    method: Option<u16>,
    context: Option<u16>,
    target: Option<(Window, Snapshot)>,
    generation: u64,
    creating: VecDeque<u64>,
    preedit: preedit::Composition,
    messages: VecDeque<Message>,
    forward: u32,
    synchronous: u32,
}

pub(super) struct InputMethod {
    client: Option<Client>,
    handler: Handler,
}

impl InputMethod {
    /// 【X11输入法】【会话创建】按display连接环境指定输入法；未配置或不可用时保留XKB输入，返回会话
    pub fn new(display: &Display) -> Self {
        let client = std::env::var("XMODIFIERS")
            .ok()
            .filter(|value| value.starts_with("@im=") && value.len() > 4 && value != "@im=none")
            .and_then(
                |_| match Client::init(display.connection.clone(), display.screen, None) {
                    Ok(client) => Some(client),
                    Err(error) => {
                        log::info!("【X11输入法】【连接回退】{error}");
                        None
                    }
                },
            );
        Self {
            client,
            handler: Handler::default(),
        }
    }

    /// 【X11输入法】【正文同步】按window与snapshot建立当前正文上下文，空值取消候选，返回协议错误
    pub fn sync(&mut self, window: Window, snapshot: Option<&Snapshot>) -> crate::AppResult {
        let identity = snapshot.map(|snapshot| (window, snapshot.target_id, snapshot.focus_id));
        let previous = self
            .handler
            .target
            .as_ref()
            .map(|(window, snapshot)| (*window, snapshot.target_id, snapshot.focus_id));
        if identity != previous {
            self.handler.generation = self.handler.generation.wrapping_add(1);
            if let (Some(client), Some(method), Some(context)) = (
                &mut self.client,
                self.handler.method,
                self.handler.context.take(),
            ) {
                client.unset_focus(method, context)?;
                client.destroy_ic(method, context)?;
            }
            self.handler.preedit.clear();
            self.handler.messages.clear();
            self.handler.target = snapshot.map(|snapshot| (window, snapshot.clone()));
            if let Some(client) = &mut self.client {
                self.handler.create(client)?;
            }
        } else if let Some(snapshot) = snapshot {
            self.handler.target = Some((window, snapshot.clone()));
            if let Some(client) = &mut self.client {
                self.handler.position(client)?;
            }
        }
        Ok(())
    }

    /// 【X11输入法】【协议过滤】按event推进XIM握手或候选消息，返回是否消费；失败停用当前输入法
    pub fn filter(&mut self, event: &Event) -> bool {
        let Some(client) = &mut self.client else {
            return false;
        };
        match client.filter_event(event, &mut self.handler) {
            Ok(consumed) => consumed,
            Err(error) => {
                log::error!("【X11输入法】【协议处理】{error}");
                self.client = None;
                self.handler.context = None;
                self.handler.preedit.clear();
                self.handler.emit(String::new());
                true
            }
        }
    }

    /// 【X11输入法】【按键交付】按event向当前上下文发送所需按键，返回是否等待输入法处理
    pub fn forward(&mut self, event: &KeyPressEvent) -> crate::AppResult<bool> {
        let (Some(client), Some(method), Some(context)) =
            (&mut self.client, self.handler.method, self.handler.context)
        else {
            return Ok(false);
        };
        let Some((window, _)) = &self.handler.target else {
            return Ok(false);
        };
        if *window != event.event {
            return Ok(false);
        }
        let mask = if event.response_type & 0x7f == KEY_PRESS_EVENT {
            u32::from(EventMask::KEY_PRESS)
        } else {
            u32::from(EventMask::KEY_RELEASE)
        };
        if self.handler.forward & mask == 0 {
            return Ok(false);
        }
        let flag = if self.handler.synchronous & mask != 0 {
            xim::ForwardEventFlag::SYNCHRONOUS
        } else {
            xim::ForwardEventFlag::empty()
        };
        client.forward_event(method, context, flag, event)?;
        Ok(true)
    }

    /// 【X11输入法】【事件提取】无参数，按协议顺序取出已完成的输入法消息
    pub fn pop(&mut self) -> Option<Message> {
        self.handler.messages.pop_front()
    }
}

impl Drop for InputMethod {
    /// 【X11输入法】【会话关闭】无参数，关闭输入法服务端资源，不等待已关闭窗口的异步回复
    fn drop(&mut self) {
        if let Some(client) = &mut self.client {
            if let Some(method) = self.handler.method {
                let _ = client.close(method);
            }
            let _ = client.disconnect();
        }
    }
}

impl Handler {
    /// 【X11输入法】【上下文请求】按client为当前正文请求独立上下文，返回协议结果
    fn create(&mut self, client: &mut Client) -> Result<(), xim::ClientError> {
        let (Some(method), Some((window, _))) = (self.method, &self.target) else {
            return Ok(());
        };
        let attributes = client
            .build_ic_attributes()
            .push(
                xim::AttributeName::InputStyle,
                xim::InputStyle::PREEDIT_CALLBACKS | xim::InputStyle::STATUS_NOTHING,
            )
            .push(xim::AttributeName::ClientWindow, *window)
            .push(xim::AttributeName::FocusWindow, *window)
            .build();
        client.create_ic(method, attributes)?;
        self.creating.push_back(self.generation);
        Ok(())
    }

    /// 【X11输入法】【候选定位】按client同步当前正文光标底边，返回协议结果
    fn position(&self, client: &mut Client) -> Result<(), xim::ClientError> {
        let (Some(method), Some(context), Some((_, snapshot))) =
            (self.method, self.context, &self.target)
        else {
            return Ok(());
        };
        let Some((left, top, _, height)) = snapshot.cursor_rect else {
            return Ok(());
        };
        let attributes = client
            .build_ic_attributes()
            .nested_list(xim::AttributeName::PreeditAttributes, |builder| {
                builder.push(
                    xim::AttributeName::SpotLocation,
                    xim::Point {
                        x: left.clamp(i16::MIN.into(), i16::MAX.into()) as i16,
                        y: top
                            .saturating_add(height)
                            .clamp(i16::MIN.into(), i16::MAX.into())
                            as i16,
                    },
                );
            })
            .build();
        client.set_ic_values(method, context, attributes)
    }

    /// 【X11输入法】【编辑排队】按commit排入当前正文编辑消息，保留当前候选，不交付已失焦正文
    fn emit(&mut self, commit: String) {
        if let Some((_, target)) = &self.target {
            self.messages.push_back(Message::Edit {
                target: target.target_id,
                edit: Edit {
                    commit,
                    preedit: self.preedit.snapshot(),
                    replacement: None,
                },
            });
        }
    }
}
