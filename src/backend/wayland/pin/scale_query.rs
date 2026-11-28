//! 【贴图】【系统比例】读取首个 Wayland 输出的物理与逻辑尺寸，供未知来源剪贴板图片回退

use smithay_client_toolkit::{
    delegate_dispatch2, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{Connection, QueueHandle, globals::registry_queue_init, protocol::wl_output};

struct Outputs {
    registry: RegistryState,
    outputs: OutputState,
}

impl OutputHandler for Outputs {
    /// 【贴图】【输出状态】返回当前连接的输出状态，不创建新连接
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    /// 【贴图】【新增输出】比例查询不需要额外处理，无返回值
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    /// 【贴图】【输出更新】比例查询不需要额外处理，无返回值
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    /// 【贴图】【输出移除】比例查询不需要额外处理，无返回值
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for Outputs {
    /// 【贴图】【注册表状态】返回当前连接的全局对象注册表
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState];
}

delegate_registry!(Outputs);
delegate_dispatch2!(Outputs);

/// 【贴图】【Wayland比例】连接合成器并读取首个输出的显示比例；没有输出或连接失败时返回空值
pub(super) fn wayland() -> Option<f32> {
    let connection = Connection::connect_to_env().ok()?;
    let (globals, mut queue) = registry_queue_init(&connection).ok()?;
    let qh = queue.handle();
    let mut state = Outputs {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
    };
    queue.roundtrip(&mut state).ok()?;
    let output = state.outputs.outputs().next()?;
    let info = state.outputs.info(&output)?;
    Some(crate::types::linux_output::Info::from(info).scale())
}
