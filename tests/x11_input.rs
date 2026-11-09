//! 【X11输入回归】【私有协议】用独立XIM服务验证候选、确认、按键回退与旧正文隔离

#![cfg(target_os = "linux")]

use mark_shot::{
    backend::{ScreenOverlay, x11},
    types::{
        OverlayEvent,
        text_input::{ChangeCause, ContentType, Snapshot},
    },
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use x11rb::{connection::Connection, protocol::xproto::*};
use xim::{Server, ServerError, ServerHandler, UserInputContext};

struct Handler(mpsc::Sender<()>);

impl<Protocol: Server<XEvent = KeyPressEvent>> ServerHandler<Protocol> for Handler {
    type InputContextData = ();
    type InputStyleArray = [xim::InputStyle; 1];

    /// 【X11输入回归】【候选风格】无参数，返回测试服务提供的回调风格
    fn input_styles(&self) -> Self::InputStyleArray {
        [xim::InputStyle::PREEDIT_CALLBACKS | xim::InputStyle::STATUS_NOTHING]
    }
    /// 【X11输入回归】【事件范围】无参数，返回只转发按下事件的协议掩码
    fn filter_events(&self) -> u32 {
        u32::from(EventMask::KEY_PRESS)
    }
    /// 【X11输入回归】【上下文数据】按服务与风格创建空测试数据，返回成功
    fn new_ic_data(
        &mut self,
        _server: &mut Protocol,
        _style: xim::InputStyle,
    ) -> Result<(), ServerError> {
        Ok(())
    }
    /// 【X11输入回归】【连接确认】按server接受测试客户端，返回成功
    fn handle_connect(&mut self, _server: &mut Protocol) -> Result<(), ServerError> {
        Ok(())
    }
    /// 【X11输入回归】【上下文销毁】按server及context释放测试状态，返回成功
    fn handle_destroy_ic(
        &mut self,
        _server: &mut Protocol,
        _context: UserInputContext<()>,
    ) -> Result<(), ServerError> {
        Ok(())
    }
    /// 【X11输入回归】【候选重置】按server及context返回空候选，不产生确认文字
    fn handle_reset_ic(
        &mut self,
        _server: &mut Protocol,
        _context: &mut UserInputContext<()>,
    ) -> Result<String, ServerError> {
        Ok(String::new())
    }
    /// 【X11输入回归】【焦点撤销】按server及context接受失焦请求，返回成功
    fn handle_unset_focus(
        &mut self,
        _server: &mut Protocol,
        _context: &mut UserInputContext<()>,
    ) -> Result<(), ServerError> {
        Ok(())
    }
    /// 【X11输入回归】【候选位置】按server及context接受定位属性，返回成功
    fn handle_set_ic_values(
        &mut self,
        _server: &mut Protocol,
        _context: &mut UserInputContext<()>,
    ) -> Result<(), ServerError> {
        Ok(())
    }
    /// 【X11输入回归】【订阅设置】按server及context声明需要的按键，返回协议结果
    fn handle_create_ic(
        &mut self,
        server: &mut Protocol,
        context: &mut UserInputContext<()>,
    ) -> Result<(), ServerError> {
        server.set_event_mask(&context.ic, u32::from(EventMask::KEY_PRESS), 0)
    }
    /// 【X11输入回归】【焦点确认】按上下文收到焦点后通知测试，不以窗口映射推测输入法就绪
    fn handle_set_focus(
        &mut self,
        _server: &mut Protocol,
        _context: &mut UserInputContext<()>,
    ) -> Result<(), ServerError> {
        self.0.send(()).unwrap();
        Ok(())
    }
    /// 【X11输入回归】【候选协议】按event发送真实候选及确认，其他按键交还客户端，返回消费状态
    fn handle_forward_event(
        &mut self,
        server: &mut Protocol,
        context: &mut UserInputContext<()>,
        event: &KeyPressEvent,
    ) -> Result<bool, ServerError> {
        match event.detail {
            38 => server.preedit_draw(&mut context.ic, "中文")?,
            56 => server.preedit_draw(&mut context.ic, "你好")?,
            36 => {
                server.preedit_draw(&mut context.ic, "")?;
                server.commit(&context.ic, "你好")?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

struct RunningServer {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    focused: mpsc::Receiver<()>,
}

impl RunningServer {
    /// 【X11输入回归】【服务启动】无参数，启动当前私有显示上的XIM夹具，返回持有退出责任的句柄
    fn start() -> Self {
        let (ready, receive) = mpsc::channel();
        let (focused, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let thread = std::thread::spawn(move || {
            let (connection, screen) = x11rb::connect(None).unwrap();
            let mut server = xim::x11rb::X11rbServer::init(
                &connection,
                screen,
                "mark-shot-test",
                xim::ALL_LOCALES,
            )
            .unwrap();
            let mut clients = xim::XimConnections::new();
            let mut handler = Handler(focused);
            connection.flush().unwrap();
            ready.send(()).unwrap();
            while !cancelled.load(Ordering::Relaxed) {
                while let Some(event) = connection.poll_for_event().unwrap() {
                    server
                        .filter_event(&event, &mut clients, &mut handler)
                        .unwrap();
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        Self {
            stop,
            thread: Some(thread),
            focused: receiver,
        }
    }
}

impl Drop for RunningServer {
    /// 【X11输入回归】【服务关闭】无参数，仅停止本测试线程，不操作其他输入法进程
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

/// 【X11输入回归】【事件等待】按overlay及accept推进协议，返回匹配业务事件，超时报告失败
fn until(
    overlay: &mut x11::overlay::Overlay,
    accept: impl Fn(&OverlayEvent) -> bool,
) -> OverlayEvent {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "X11 input event timed out");
        let event = overlay.next_event(Some(Duration::from_millis(20))).unwrap();
        if accept(&event) {
            return event;
        }
    }
}

/// 【X11输入回归】【按键投递】按display及detail投递当前私有焦点窗口的按键，返回服务器确认
fn key(display: &x11::connection::Display, detail: u8) {
    let window = display
        .connection
        .get_input_focus()
        .unwrap()
        .reply()
        .unwrap()
        .focus;
    display
        .connection
        .send_event(
            false,
            window,
            EventMask::KEY_PRESS,
            KeyPressEvent {
                response_type: KEY_PRESS_EVENT,
                detail,
                event: window,
                root: display.screen().root,
                same_screen: true,
                ..Default::default()
            },
        )
        .unwrap()
        .check()
        .unwrap();
}

/// 【X11输入回归】【完整协议】验证中文候选更新与确认、未处理键回退和取消后旧正文不再接收输入，无参数
#[test]
#[ignore = "requires isolated Xvfb and XMODIFIERS=@im=mark-shot-test"]
fn xim_preedit_commit_and_fallback_follow_the_current_text_target() {
    assert_eq!(
        std::env::var("MARK_SHOT_TEST_X11_DISPLAY").unwrap(),
        std::env::var("DISPLAY").unwrap()
    );
    assert_eq!(std::env::var("XMODIFIERS").unwrap(), "@im=mark-shot-test");
    assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
    let server = RunningServer::start();
    let display = x11::connection::Display::connect().unwrap();
    let mut overlay = x11::overlay::Overlay::new(display.clone()).unwrap();
    overlay.configure_image_window((400, 300));
    overlay.present(&[0]).unwrap();
    overlay
        .stage_frame(0, &vec![255; 400 * 300 * 4], None)
        .unwrap();
    let OverlayEvent::TextInputFocus(focus) = until(&mut overlay, |event| {
        matches!(event, OverlayEvent::TextInputFocus(_))
    }) else {
        unreachable!()
    };
    let snapshot = Snapshot {
        target_id: 100,
        focus_id: focus.id,
        monitor_idx: 0,
        surrounding: None,
        cursor_rect: Some((20, 20, 1, 20)),
        cause: ChangeCause::Other,
        content: ContentType::Multiline,
    };
    overlay.set_text_input(Some(&snapshot)).unwrap();
    until(&mut overlay, |_| server.focused.try_recv().is_ok());
    key(&display, 38);
    until(
        &mut overlay,
        |event| matches!(event, OverlayEvent::TextInput(update) if update.target_id == 100 && update.batch.preedit.text == "中文"),
    );
    key(&display, 56);
    until(
        &mut overlay,
        |event| matches!(event, OverlayEvent::TextInput(update) if update.batch.preedit.text == "你好"),
    );
    key(&display, 36);
    until(
        &mut overlay,
        |event| matches!(event, OverlayEvent::TextInput(update) if update.batch.commit == "你好" && update.batch.preedit.text.is_empty()),
    );
    key(&display, 26);
    until(
        &mut overlay,
        |event| matches!(event, OverlayEvent::Key { text: Some(text), .. } if text == "e"),
    );
    key(&display, 38);
    overlay.set_text_input(None).unwrap();
    key(&display, 26);
    until(&mut overlay, |event| {
        assert!(
            !matches!(event, OverlayEvent::TextInput(_)),
            "stale XIM edit reached a closed target"
        );
        matches!(event, OverlayEvent::Key { text: Some(text), .. } if text == "e")
    });
    let next = Snapshot {
        target_id: 101,
        ..snapshot
    };
    overlay.set_text_input(Some(&next)).unwrap();
    until(&mut overlay, |_| server.focused.try_recv().is_ok());
    key(&display, 36);
    until(
        &mut overlay,
        |event| matches!(event, OverlayEvent::TextInput(update) if update.target_id == 101 && update.batch.commit == "你好"),
    );
}
