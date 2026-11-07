//! 【Wayland主题】【事件接入】独立窗口和贴图复用原生目录通知，不增加线程或空闲期限

use crate::theme::{apply_saved, config_watch::Watch};
use smithay_client_toolkit::reexports::calloop::{
    Interest, LoopHandle, Mode, PostAction, generic::Generic,
};

/// 【Wayland主题】【首次同步】创建当前监听并处理启动配置读取与来源登记之间的变化，返回可选来源
pub(super) fn source() -> Option<Watch> {
    let watch = Watch::current()?;
    match watch.changed() {
        Ok(Some(mode)) => {
            apply_saved(mode);
        }
        Ok(None) => {}
        Err(error) => log::error!("【Wayland主题】【首次同步】无法读取保存主题: {error}"),
    }
    Some(watch)
}

/// 【Wayland主题】【循环登记】向 handle 登记原生来源，changed 仅在主题实际变化时刷新状态，返回登记结果
pub(super) fn install<State: 'static>(
    handle: &LoopHandle<'static, State>,
    mut changed: impl FnMut(&mut State) + 'static,
) -> crate::AppResult {
    let Some(watch) = source() else { return Ok(()) };
    handle
        .insert_source(
            Generic::new(watch, Interest::READ, Mode::Level),
            move |_, source, state| {
                match source.changed() {
                    Ok(Some(mode)) => {
                        apply_saved(mode);
                        changed(state);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("【Wayland主题】【通知读取】无法读取保存主题: {error}")
                    }
                }
                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| error.error)?;
    Ok(())
}
