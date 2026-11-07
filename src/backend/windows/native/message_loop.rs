//! 【Windows运行时】【共同等待】窗口消息与内核事件共用阻塞等待，空闲不安排周期绘制

use std::{io, time::Duration};
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
        PeekMessageW, QS_ALLINPUT, TranslateMessage,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    Messages,
    Source(usize),
    Timeout,
}

/// 【Windows运行时】【消息等待】按handles和timeout阻塞，分派当前窗口消息后返回唤醒来源，不持有业务状态借用
pub fn dispatch(handles: &[isize], timeout: Option<Duration>) -> io::Result<Wait> {
    if handles.len() > 62 {
        return Err(io::Error::other("too many Windows event sources"));
    }
    let sources: Vec<HANDLE> = handles.iter().map(|handle| *handle as HANDLE).collect();
    let timeout = timeout.map_or(u32::MAX, |duration| {
        duration
            .as_millis()
            .saturating_add(u128::from(duration.subsec_nanos() % 1_000_000 != 0))
            .min(u128::from(u32::MAX - 1)) as u32
    });
    let result = unsafe {
        MsgWaitForMultipleObjectsEx(
            sources.len() as u32,
            sources.as_ptr(),
            timeout,
            QS_ALLINPUT,
            MWMO_INPUTAVAILABLE,
        )
    };
    if result == WAIT_FAILED {
        return Err(super::last_error());
    }
    // 1. 【Windows运行时】【消息消费】处理所有已经到达的消息，后台来源同时到达时也不延迟输入或帧绘制
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    if result == WAIT_TIMEOUT {
        Ok(Wait::Timeout)
    } else if result - WAIT_OBJECT_0 < handles.len() as u32 {
        Ok(Wait::Source((result - WAIT_OBJECT_0) as usize))
    } else {
        Ok(Wait::Messages)
    }
}
