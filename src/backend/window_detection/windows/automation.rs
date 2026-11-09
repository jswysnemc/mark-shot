//! 【Windows控件识别】【缓存探测】COM对象只属于工作线程，按命中路径展开控件并缓存完整子节点批次

use crate::features::region_detection::PixelRect;
use std::time::{Duration, Instant};
use windows::{
    Win32::{Foundation::HWND, System::Com::*, UI::Accessibility::*},
    core::{Interface, Result},
};

struct Apartment;
impl Drop for Apartment {
    /// 【Windows控件识别】【COM释放】无参数，在全部控件接口销毁后释放本线程COM环境
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

struct Node {
    rect: PixelRect,
    element: Option<IUIAutomationElement>,
    children: Option<Vec<Node>>,
}

pub(super) struct Automation {
    roots: std::collections::HashMap<usize, Node>,
    request: IUIAutomationCacheRequest,
    automation: IUIAutomation2,
    _apartment: Apartment,
    #[cfg(test)]
    pub(super) calls: usize,
}

impl Automation {
    /// 【Windows控件识别】【线程初始化】无参数，建立有超时限制的UIA缓存请求，返回独立线程资源或系统错误
    pub fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let automation: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
            automation.SetConnectionTimeout(40)?;
            automation.SetTransactionTimeout(40)?;
            let request = automation.CreateCacheRequest()?;
            request.SetTreeScope(TreeScope(TreeScope_Element.0 | TreeScope_Children.0))?;
            request.SetTreeFilter(&automation.ControlViewCondition()?)?;
            request.SetAutomationElementMode(AutomationElementMode_Full)?;
            request.AddProperty(UIA_BoundingRectanglePropertyId)?;
            request.AddProperty(UIA_IsOffscreenPropertyId)?;
            Ok(Self {
                roots: Default::default(),
                request,
                automation,
                _apartment: apartment,
                #[cfg(test)]
                calls: 0,
            })
        }
    }

    /// 【Windows控件识别】【路径查询】按窗口handle、bounds和point逐层探测，cancelled终止旧请求，publish交付已确认路径
    pub fn query(
        &mut self,
        handle: usize,
        bounds: PixelRect,
        point: (i32, i32),
        cancelled: impl Fn() -> bool,
        mut publish: impl FnMut(Vec<PixelRect>),
    ) -> bool {
        let deadline = Instant::now() + Duration::from_millis(180);
        let root = self.roots.entry(handle).or_insert_with(|| Node {
            rect: bounds,
            element: None,
            children: None,
        });
        let mut node = root;
        let mut path = vec![bounds];
        for depth in 0..48 {
            if cancelled() || Instant::now() >= deadline {
                return false;
            }
            if node.children.is_none() {
                #[cfg(test)]
                {
                    self.calls += 1;
                }
                let loaded = unsafe {
                    match &node.element {
                        Some(element) => element.BuildUpdatedCache(&self.request),
                        None => self
                            .automation
                            .ElementFromHandleBuildCache(HWND(handle as *mut _), &self.request),
                    }
                };
                let Ok(loaded) = loaded else {
                    return false;
                };
                let Ok(children) = children(&loaded, node.rect, &cancelled, deadline) else {
                    return false;
                };
                node.children = Some(children);
                node.element = None;
            }
            let Some(child) = node.children.as_mut().and_then(|children| {
                children
                    .iter_mut()
                    .rev()
                    .find(|child| child.rect.contains(point))
            }) else {
                if !cancelled() {
                    publish(path.into_iter().rev().collect());
                }
                return true;
            };
            if !path.contains(&child.rect) && node.rect.contains_rect(child.rect) {
                path.push(child.rect);
                publish(path.iter().rev().copied().collect());
            }
            node = child;
            if depth == 47 {
                log::info!("【Windows控件识别】【路径限制】控件树达到最大深度");
            }
        }
        if !cancelled() {
            publish(path.into_iter().rev().collect());
        }
        false
    }
}

/// 【Windows控件识别】【完整批次】按element读取本地缓存子节点，bounds裁切窗口，取消或超时不发布不完整兄弟顺序
fn children(
    element: &IUIAutomationElement,
    bounds: PixelRect,
    cancelled: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Vec<Node>> {
    let mut raw = std::ptr::null_mut();
    unsafe {
        (element.vtable().GetCachedChildren)(element.as_raw(), &mut raw).ok()?;
    }
    if raw.is_null() {
        return Ok(Vec::new());
    }
    let array = unsafe { IUIAutomationElementArray::from_raw(raw) };
    let length = unsafe { array.Length()? }.max(0);
    if length > 8192 {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004004u32 as i32,
        )));
    }
    let mut result = Vec::new();
    for index in 0..length {
        if cancelled() || Instant::now() >= deadline {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80004004u32 as i32,
            )));
        }
        let element = unsafe { array.GetElement(index)? };
        if unsafe { element.CachedIsOffscreen()? }.as_bool() {
            continue;
        }
        let rect = unsafe { element.CachedBoundingRectangle()? };
        let Some(width) = rect.right.checked_sub(rect.left) else {
            continue;
        };
        let Some(height) = rect.bottom.checked_sub(rect.top) else {
            continue;
        };
        if width <= 0 || height <= 0 {
            continue;
        }
        let Some(rect) = PixelRect::new(rect.left, rect.top, width, height).intersect(bounds)
        else {
            continue;
        };
        result.push(Node {
            rect,
            element: Some(element),
            children: None,
        });
    }
    Ok(result)
}
