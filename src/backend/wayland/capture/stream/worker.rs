//! 【PipeWire】【持续循环】一个连接按需激活并复制一帧，暂停后关闭活跃流和定时工作

use super::{Command, Request, format, pixels};
use pipewire::{self as pw, properties::properties, spa};
use std::{
    cell::RefCell,
    os::fd::OwnedFd,
    rc::Rc,
    sync::{Arc, Mutex},
};

struct UserData {
    format: spa::param::video::VideoInfoRaw,
}
type Pending = Rc<RefCell<Option<Request>>>;

/// 【PipeWire】【结果完成】用 pending 交付 result 并清除当前请求，无返回值，已取消请求不再拥有帧
fn complete(pending: &Pending, result: super::FrameResult) {
    if let Some(request) = pending.borrow_mut().take() {
        let _ = request.reply.send(result);
    }
}

/// 【PipeWire】【错误记忆】以 error 保存终止原因并交付 pending，failure跨线程提供下一次请求诊断，无返回值
fn fail(pending: &Pending, failure: &Arc<Mutex<Option<String>>>, error: String) {
    *failure.lock().unwrap() = Some(error.clone());
    complete(pending, Err(error));
}

/// 【PipeWire】【流运行】用 node_id、fd、commands 和 failure 运行唯一流，返回设置或协议失败，不创建周期轮询
pub(super) fn run(
    node_id: u32,
    fd: OwnedFd,
    commands: pw::channel::Receiver<Command>,
    failure: Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|error| format!("cannot create PipeWire main loop: {error}"))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|error| format!("cannot create PipeWire context: {error}"))?;
    let core = context
        .connect_fd_rc(fd, None)
        .map_err(|error| format!("cannot connect PipeWire remote: {error}"))?;
    let stream = pw::stream::StreamRc::new(core.clone(), "mark-shot", properties! {
        *pw::keys::MEDIA_TYPE => "Video", *pw::keys::MEDIA_CATEGORY => "Capture", *pw::keys::MEDIA_ROLE => "Screen",
    }).map_err(|error| format!("cannot create PipeWire stream: {error}"))?;
    let pending: Pending = Rc::new(RefCell::new(None));
    let _core_listener = core
        .add_listener_local()
        .error({
            let pending = pending.clone();
            let failure = failure.clone();
            let mainloop = mainloop.clone();
            move |_, _, _, message| {
                fail(
                    &pending,
                    &failure,
                    format!("PipeWire remote error: {message}"),
                );
                mainloop.quit();
            }
        })
        .register();
    let _listener = stream
        .add_local_listener_with_user_data(UserData {
            format: Default::default(),
        })
        .state_changed({
            let pending = pending.clone();
            let failure = failure.clone();
            let mainloop = mainloop.clone();
            move |_, _, _, new| {
                if let pw::stream::StreamState::Error(error) = new {
                    fail(
                        &pending,
                        &failure,
                        format!("PipeWire stream error: {error}"),
                    );
                    mainloop.quit();
                }
            }
        })
        .param_changed(|_, data, id, parameter| {
            let Some(parameter) = parameter else {
                return;
            };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            if let Ok((media, subtype)) = spa::param::format_utils::parse_format(parameter)
                && media == spa::param::format::MediaType::Video
                && subtype == spa::param::format::MediaSubtype::Raw
            {
                let _ = data.format.parse(parameter);
            }
        })
        .process({
            let pending = pending.clone();
            move |stream, data| {
                // 1. 【PipeWire】【按需复制】没有存活请求时只归还缓冲，不能按合成器刷新率复制整幅图片
                let requested = pending
                    .borrow()
                    .as_ref()
                    .is_some_and(|request| !request.reply.is_closed());
                let result = {
                    let Some(mut buffer) = stream.dequeue_buffer() else {
                        return;
                    };
                    if !requested {
                        None
                    } else {
                        let Some(raw) = buffer.datas_mut().first_mut() else {
                            return;
                        };
                        let chunk = (
                            raw.chunk().offset() as usize,
                            raw.chunk().size() as usize,
                            raw.chunk().stride(),
                        );
                        if chunk.1 == 0 {
                            return;
                        }
                        let dimensions = (data.format.size().width, data.format.size().height);
                        Some(match raw.data() {
                            Some(bytes) => pixels::read(
                                bytes,
                                chunk.0,
                                chunk.1,
                                chunk.2,
                                dimensions,
                                data.format.format(),
                            ),
                            None => {
                                Err("PipeWire buffer cannot be mapped into shared memory".into())
                            }
                        })
                    }
                };
                if let Some(result) = result {
                    complete(&pending, result);
                }
                // 2. 【PipeWire】【暂停资源】归还当前缓冲后停用流，直到下一次明确请求
                if let Err(error) = stream.set_active(false) {
                    complete(
                        &pending,
                        Err(format!("cannot deactivate PipeWire stream: {error}")),
                    );
                }
            }
        })
        .register()
        .map_err(|error| format!("cannot register PipeWire listener: {error}"))?;
    let _commands = commands.attach(mainloop.loop_(), {
        let pending = pending.clone();
        let stream = stream.clone();
        let mainloop = mainloop.clone();
        move |command| match command {
            Command::Frame(request) => {
                if request.reply.is_closed() {
                    return;
                }
                complete(
                    &pending,
                    Err("PipeWire request replaced before completion".into()),
                );
                // 3. 【PipeWire】【旧帧清理】重新激活前归还旧队列，恢复不能使用暂停前的画面
                while let Some(buffer) = stream.dequeue_buffer() {
                    drop(buffer);
                }
                *pending.borrow_mut() = Some(request);
                if let Err(error) = stream.set_active(true) {
                    complete(
                        &pending,
                        Err(format!("cannot activate PipeWire stream: {error}")),
                    );
                }
            }
            Command::Cancel(id) => {
                if pending
                    .borrow()
                    .as_ref()
                    .is_some_and(|request| request.id == id)
                {
                    pending.borrow_mut().take();
                    let _ = stream.set_active(false);
                }
            }
            Command::Stop => {
                complete(&pending, Err("PipeWire stream closed".into()));
                mainloop.quit();
            }
        }
    });
    let mut buffer = Vec::new();
    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::INACTIVE,
            &mut [format::build_format_pod(&mut buffer)],
        )
        .map_err(|error| format!("cannot connect PipeWire stream: {error}"))?;
    mainloop.run();
    Ok(())
}
