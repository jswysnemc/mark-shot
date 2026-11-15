//! 【Windows控件识别】【原生适配】COM对象只属于工作线程，系统缓存批次交给独立查询算法

use crate::{
    backend::window_detection::uia::{
        Batch, Candidate, Policy, Provider, QueryClock, ResultPath, Stop, Tree,
    },
    features::region_detection::PixelRect,
};
use std::{collections::HashMap, time::Duration};
use windows::{
    Win32::{Foundation::HWND, System::Com::*, UI::Accessibility::*},
    core::{Error, Interface, Result},
};

struct Apartment;
impl Drop for Apartment {
    /// 【Windows控件识别】【COM释放】无参数，在所有控件接口销毁后释放本线程COM环境
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct NativeProvider {
    request: IUIAutomationCacheRequest,
    automation: IUIAutomation2,
    calls: usize,
    _apartment: Apartment,
}
pub(super) struct Automation {
    roots: HashMap<usize, Tree<NativeProvider>>,
    provider: NativeProvider,
    #[cfg(test)]
    pub(super) calls: usize,
}
impl Automation {
    /// 【Windows控件识别】【线程初始化】无参数，建立有超时限制的UIA控件缓存，返回独立线程资源或系统错误
    pub fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|error| context("initialize COM", error))?;
            let apartment = Apartment;
            let automation: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| context("create UI Automation", error))?;
            let request = automation
                .CreateCacheRequest()
                .map_err(|error| context("create cache", error))?;
            request
                .SetTreeScope(TreeScope(TreeScope_Element.0 | TreeScope_Children.0))
                .map_err(|error| context("cache tree scope", error))?;
            let filter = automation
                .ControlViewCondition()
                .map_err(|error| context("control view condition", error))?;
            request
                .SetTreeFilter(&filter)
                .map_err(|error| context("control view filter", error))?;
            request
                .SetAutomationElementMode(AutomationElementMode_Full)
                .map_err(|error| context("full element references", error))?;
            request
                .AddProperty(UIA_BoundingRectanglePropertyId)
                .map_err(|error| context("bounding rectangle property", error))?;
            request
                .AddProperty(UIA_IsOffscreenPropertyId)
                .map_err(|error| context("offscreen property", error))?;
            request
                .AddProperty(UIA_ControlTypePropertyId)
                .map_err(|error| context("control type property", error))?;
            Ok(Self {
                roots: HashMap::new(),
                provider: NativeProvider {
                    request,
                    automation,
                    calls: 0,
                    _apartment: apartment,
                },
                #[cfg(test)]
                calls: 0,
            })
        }
    }
    /// 【Windows控件识别】【预算查询】按handle、bounds、point与policy查询，cancelled终止旧目标，publish接收补充路径，返回可靠路径及停止原因
    pub fn query_with_policy(
        &mut self,
        handle: usize,
        bounds: PixelRect,
        point: (i32, i32),
        policy: Policy,
        cancelled: impl Fn() -> bool,
        mut publish: impl FnMut(Vec<PixelRect>),
    ) -> ResultPath {
        // 1. 【Windows控件识别】【缓存容量】会话内最多保存六十四个访问过的窗口，避免遍历整个桌面后持续积累原生对象
        if self.roots.len() >= 64 && !self.roots.contains_key(&handle) {
            self.roots.clear();
        }
        let tree = self
            .roots
            .entry(handle)
            .or_insert_with(|| Tree::new(handle, bounds));
        let result = tree.query(
            &mut self.provider,
            point,
            policy,
            &QueryClock::new(),
            &cancelled,
            &mut publish,
        );
        #[cfg(test)]
        {
            self.calls = self.provider.calls;
        }
        result
    }
}
impl NativeProvider {
    /// 【Windows控件识别】【调用超时】按limit设置本次跨进程调用限制，返回系统设置错误，旧系统仅在参数不兼容时尝试较大值
    fn timeouts(&self, limit: Duration) -> Result<()> {
        let milliseconds = limit.as_millis().clamp(1, u128::from(u32::MAX)) as u32;
        timeout("connection timeout", milliseconds, |value| unsafe {
            self.automation.SetConnectionTimeout(value)
        })?;
        timeout("transaction timeout", milliseconds, |value| unsafe {
            self.automation.SetTransactionTimeout(value)
        })
    }
}
impl Provider for NativeProvider {
    type Element = IUIAutomationElement;
    type Batch = NativeBatch;
    /// 【Windows控件识别】【根批次】按handle与limit获取对应窗口的Control View子节点，返回本地缓存或停止原因
    fn root(&mut self, handle: usize, limit: Duration) -> std::result::Result<NativeBatch, Stop> {
        self.calls += 1;
        self.timeouts(limit).map_err(stop)?;
        NativeBatch::new(
            unsafe {
                self.automation
                    .ElementFromHandleBuildCache(HWND(handle as *mut _), &self.request)
            }
            .map_err(stop)?,
        )
    }
    /// 【Windows控件识别】【后代批次】按element与limit更新其子节点缓存，返回本地批次或停止原因
    fn children(
        &mut self,
        element: &IUIAutomationElement,
        limit: Duration,
    ) -> std::result::Result<NativeBatch, Stop> {
        self.calls += 1;
        self.timeouts(limit).map_err(stop)?;
        NativeBatch::new(unsafe { element.BuildUpdatedCache(&self.request) }.map_err(stop)?)
    }
}
struct NativeBatch {
    array: Option<IUIAutomationElementArray>,
    count: usize,
}
impl NativeBatch {
    /// 【Windows控件识别】【空批次兼容】按element读取缓存子节点，返回本地批次，系统返回成功与空指针时视为空集合
    fn new(element: IUIAutomationElement) -> std::result::Result<Self, Stop> {
        let mut raw = std::ptr::null_mut();
        unsafe {
            (element.vtable().GetCachedChildren)(element.as_raw(), &mut raw)
                .ok()
                .map_err(stop)?;
        }
        let array = if raw.is_null() {
            None
        } else {
            Some(unsafe { IUIAutomationElementArray::from_raw(raw) })
        };
        let count = match &array {
            Some(array) => unsafe { array.Length().map_err(stop)? }.max(0) as usize,
            None => 0,
        };
        Ok(Self { array, count })
    }
}
impl Batch for NativeBatch {
    type Element = IUIAutomationElement;
    /// 【Windows控件识别】【缓存数量】无参数，返回已获取的兄弟节点数量
    fn len(&self) -> usize {
        self.count
    }
    /// 【Windows控件识别】【缓存属性】按index读取矩形、离屏属性及控件类型，返回候选或当前节点错误
    fn get(&self, index: usize) -> std::result::Result<Candidate<IUIAutomationElement>, Stop> {
        unsafe {
            let element = self
                .array
                .as_ref()
                .expect("nonempty cached batch")
                .GetElement(index as i32)
                .map_err(stop)?;
            let offscreen = element.CachedIsOffscreen().map_err(stop)?.as_bool();
            let bounds = if offscreen {
                Default::default()
            } else {
                element.CachedBoundingRectangle().map_err(stop)?
            };
            let rect = PixelRect {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            };
            let structural = element
                .CachedControlType()
                .is_ok_and(|kind| kind == UIA_PaneControlTypeId || kind == UIA_GroupControlTypeId);
            Ok(Candidate {
                element,
                rect,
                offscreen,
                structural,
            })
        }
    }
}
/// 【Windows控件识别】【停止分类】按error区分接口超时和失败，返回查询停止原因
fn stop(error: Error) -> Stop {
    if error.code().0 as u32 == UIA_E_TIMEOUT || error.code().0 as u32 == 0x800705b4 {
        Stop::Timeout
    } else {
        Stop::Failure
    }
}
/// 【Windows控件识别】【系统错误】按stage与error保留HRESULT并补充操作，返回可定位错误
fn context(stage: &str, error: Error) -> Error {
    Error::new(error.code(), format!("{stage}: {error}"))
}
/// 【Windows控件识别】【超时兼容】按stage、requested与apply设置超时，返回结果，参数不兼容才尝试系统接受的较大值
fn timeout(stage: &str, requested: u32, apply: impl Fn(u32) -> Result<()>) -> Result<()> {
    let mut failure = None;
    for milliseconds in std::iter::once(requested).chain(
        [250, 500, 1000, 2000]
            .into_iter()
            .filter(|value| *value > requested),
    ) {
        match apply(milliseconds) {
            Ok(()) => return Ok(()),
            Err(error) if error.code().0 as u32 == 0x80070057 => failure = Some(error),
            Err(error) => return Err(context(stage, error)),
        }
    }
    Err(context(
        stage,
        failure.expect("timeout candidates attempted"),
    ))
}
