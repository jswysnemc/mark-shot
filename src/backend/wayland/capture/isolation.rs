//! 【持续捕获】【连接隔离】后台取帧独占连接，避免覆盖层与捕获线程同时等待同一个读取屏障

use std::collections::HashSet;

use async_trait::async_trait;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_output, wl_registry},
};

use crate::{
    backend::{
        CaptureMethod,
        capture_session::{self, CaptureSession, Result},
    },
    types::{CaptureResult, Output},
};

struct State;

struct Binding {
    connection: Connection,
    queue: EventQueue<State>,
    outputs: Vec<Output>,
}

struct Session {
    inner: Option<Box<dyn CaptureSession>>,
    binding: Option<Binding>,
}

/// 【持续捕获】【独立会话】按 outputs 绑定独立连接，factory 创建对应原生后端，返回可取消、可关闭的会话
pub(super) async fn open(
    outputs: &[Output],
    factory: impl FnOnce(Connection) -> Box<dyn CaptureMethod>,
) -> Result<Box<dyn CaptureSession>> {
    let outputs = outputs.to_vec();
    let binding = tokio::task::spawn_blocking(move || bind(outputs)).await??;
    let method = factory(binding.connection.clone());
    let inner = capture_session::repeated(method, &binding.outputs);
    Ok(Box::new(Session {
        inner: Some(inner),
        binding: Some(binding),
    }))
}

/// 【持续捕获】【输出重绑】按 outputs 的注册表全局标识重新绑定同一输出，返回持有连接和事件队列的资源
fn bind(outputs: Vec<Output>) -> Result<Binding> {
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let qh = queue.handle();
    let mut seen = HashSet::new();
    let mut bound = Vec::with_capacity(outputs.len());
    // 1. 【持续捕获】【身份校验】输出全局标识来自同一合成器的注册表，不能使用列表次序或协议对象编号猜测
    for output in outputs {
        let id = output.info.id;
        if !seen.insert(id) {
            return Err("continuous capture has a duplicate output identity".into());
        }
        let version = globals.contents().with_list(|list| {
            list.iter()
                .find(|global| global.name == id && global.interface == "wl_output")
                .map(|global| global.version.min(4))
        });
        let version = version.ok_or("the continuous capture output is gone")?;
        let wl_output = globals.registry().bind(id, version, &qh, ());
        bound.push(Output {
            wl_output,
            info: output.info,
        });
    }
    queue.roundtrip(&mut State)?;
    Ok(Binding {
        connection,
        queue,
        outputs: bound,
    })
}

#[async_trait]
impl CaptureSession for Session {
    /// 【持续捕获】【独占取帧】分派本连接的输出通知后请求一帧，无参数，关闭后的调用返回明确错误
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        let binding = self.binding.as_mut().ok_or("capture session is closed")?;
        binding.queue.dispatch_pending(&mut State)?;
        let result = self
            .inner
            .as_mut()
            .ok_or("capture session is closed")?
            .next_frame()
            .await;
        binding.queue.dispatch_pending(&mut State)?;
        result
    }

    /// 【持续捕获】【完整关闭】关闭后端并释放输出、事件队列与独立连接，无参数，重复关闭无副作用
    async fn close(&mut self) -> Result<()> {
        let result = match self.inner.as_mut() {
            Some(inner) => inner.close().await,
            None => Ok(()),
        };
        self.inner = None;
        self.binding = None;
        result
    }
}

impl Drop for Binding {
    /// 【持续捕获】【输出释放】释放支持 release 的绑定并提交销毁请求，无参数，最终连接关闭回收其余对象
    fn drop(&mut self) {
        for output in &self.outputs {
            if output.wl_output.version() >= 3 {
                output.wl_output.release();
            }
        }
        let _ = self.connection.flush();
    }
}

wayland_client::delegate_noop!(State: ignore wl_output::WlOutput);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    /// 【持续捕获】【注册表通知】由 GlobalListContents 维护全局列表，本回调不创建额外目标，无返回值
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
