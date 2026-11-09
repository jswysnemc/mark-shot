//! 【X11输入法】【消息回调】只接受当前上下文的候选与确认，旧上下文创建完成后立即回收

use super::*;
use xim::{ClientError, ClientHandler};

impl ClientHandler<Client> for Handler {
    /// 【X11输入法】【连接完成】按client打开当前区域语言输入法，返回协议结果
    fn handle_connect(&mut self, client: &mut Client) -> Result<(), ClientError> {
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
            .unwrap_or_else(|| "C.UTF-8".into());
        client.open(&locale)
    }

    /// 【X11输入法】【服务就绪】记录method并为等待的正文创建上下文，返回协议结果
    fn handle_open(&mut self, client: &mut Client, method: u16) -> Result<(), ClientError> {
        self.method = Some(method);
        self.create(client)
    }

    /// 【X11输入法】【上下文完成】按method和context确认创建身份，过期回复立即销毁，返回协议结果
    fn handle_create_ic(
        &mut self,
        client: &mut Client,
        method: u16,
        context: u16,
    ) -> Result<(), ClientError> {
        if self.creating.pop_front() != Some(self.generation) || self.target.is_none() {
            return client.destroy_ic(method, context);
        }
        self.context = Some(context);
        self.forward = u32::from(EventMask::KEY_PRESS | EventMask::KEY_RELEASE);
        self.synchronous = 0;
        client.set_focus(method, context)?;
        self.position(client)
    }

    /// 【X11输入法】【事件订阅】按context更新forward与synchronous掩码，不修改其他上下文
    fn handle_set_event_mask(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
        forward: u32,
        synchronous: u32,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) {
            self.forward = forward;
            self.synchronous = synchronous;
        }
        Ok(())
    }

    /// 【X11输入法】【未处理按键】按context与event排入原生回退队列，不再次转发输入法
    fn handle_forward_event(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
        _flag: xim::ForwardEventFlag,
        event: KeyPressEvent,
    ) -> Result<(), ClientError> {
        if self.context == Some(context)
            && let Some((_, target)) = &self.target
        {
            self.messages.push_back(Message::Key {
                target: target.target_id,
                event,
            });
        }
        Ok(())
    }

    /// 【X11输入法】【文字确认】按context与text交付确认正文，清空候选，过期上下文不生效
    fn handle_commit(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
        text: &str,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) {
            self.preedit.clear();
            self.emit(text.to_owned());
        }
        Ok(())
    }

    /// 【X11输入法】【候选开始】按context准备新候选，其余参数为协议连接，不修改已确认正文
    fn handle_preedit_start(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) {
            self.preedit.clear();
            self.emit(String::new());
        }
        Ok(())
    }

    /// 【X11输入法】【候选更新】按context、字符范围、text和caret更新候选，忽略非法范围及旧上下文
    fn handle_preedit_draw(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
        caret: i32,
        first: i32,
        length: i32,
        _status: xim::PreeditDrawStatus,
        text: &str,
        _feedback: Vec<xim::Feedback>,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) && self.preedit.replace(first, length, text, caret) {
            self.emit(String::new());
        }
        Ok(())
    }

    /// 【X11输入法】【候选光标】按context与position处理绝对位置请求，返回服务器确认位置
    fn handle_preedit_caret(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
        position: &mut i32,
        direction: xim::CaretDirection,
        _style: xim::CaretStyle,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) && direction == xim::CaretDirection::AbsolutePosition {
            *position = self.preedit.move_caret(*position);
            self.emit(String::new());
        }
        Ok(())
    }

    /// 【X11输入法】【候选结束】按context清除临时候选，保留已确认正文
    fn handle_preedit_done(
        &mut self,
        _client: &mut Client,
        _method: u16,
        context: u16,
    ) -> Result<(), ClientError> {
        if self.context == Some(context) {
            self.preedit.clear();
            self.emit(String::new());
        }
        Ok(())
    }

    /// 【X11输入法】【连接终止】无参数，取消当前候选并恢复键盘直接输入
    fn handle_disconnect(&mut self) {
        self.method = None;
        self.context = None;
        self.preedit.clear();
        self.emit(String::new());
    }
}
