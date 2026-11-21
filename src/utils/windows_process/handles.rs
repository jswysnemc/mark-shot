//! 【Windows内部进程】【句柄交接】匿名管道父端不继承，子端白名单只包含三条标准流

use super::{Stream, Streams};
use std::{
    fs::File,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows_sys::Win32::{
    Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE},
    System::{
        Pipes::CreatePipe,
        Threading::{
            DeleteProcThreadAttributeList, GetCurrentProcess, InitializeProcThreadAttributeList,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            UpdateProcThreadAttribute,
        },
    },
};

pub(super) struct Prepared {
    pub input: OwnedHandle,
    pub output: OwnedHandle,
    pub error: OwnedHandle,
    pub stdin: Option<File>,
    pub stdout: Option<File>,
}

/// 【Windows内部进程】【可继承副本】按 handle 创建可继承的同权限句柄，返回自动关闭副本，不修改原句柄继承状态
fn duplicate(handle: HANDLE) -> io::Result<OwnedHandle> {
    let mut copy = std::ptr::null_mut();
    let process = unsafe { GetCurrentProcess() };
    if unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut copy,
            0,
            1,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedHandle::from_raw_handle(copy) })
}

/// 【Windows内部进程】【单流准备】按 stream 与 input 返回子端句柄和可选父端文件；输入管道的父端负责写入
fn prepare(stream: Stream, input: bool) -> io::Result<(OwnedHandle, Option<File>)> {
    match stream {
        Stream::Null => {
            let file = File::options().read(true).write(true).open("NUL")?;
            Ok((duplicate(file.as_raw_handle())?, None))
        }
        Stream::File(file) => Ok((duplicate(file.as_raw_handle())?, None)),
        Stream::Piped => {
            let mut read = std::ptr::null_mut();
            let mut write = std::ptr::null_mut();
            if unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 0) } == 0 {
                return Err(io::Error::last_os_error());
            }
            let read = unsafe { OwnedHandle::from_raw_handle(read) };
            let write = unsafe { OwnedHandle::from_raw_handle(write) };
            let (child, parent) = if input { (read, write) } else { (write, read) };
            Ok((duplicate(child.as_raw_handle())?, Some(File::from(parent))))
        }
    }
}

impl Prepared {
    /// 【Windows内部进程】【标准流准备】按 streams 创建三条标准流，返回子端白名单和父端管道；失败自动回收已创建句柄
    pub fn new(streams: Streams) -> io::Result<Self> {
        let (input, stdin) = prepare(streams.stdin, true)?;
        let (output, stdout) = prepare(streams.stdout, false)?;
        let (error, _) = prepare(streams.stderr.map_or(Stream::Null, Stream::File), false)?;
        Ok(Self {
            input,
            output,
            error,
            stdin,
            stdout,
        })
    }
}

pub(super) struct Attributes {
    storage: Vec<usize>,
    initialized: bool,
}

impl Attributes {
    /// 【Windows内部进程】【白名单创建】按 handles 创建启动属性列表，返回列表；句柄数组须保留到CreateProcessW完成
    pub fn new(handles: &[HANDLE]) -> io::Result<Self> {
        let mut size = 0;
        unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size) };
        if size == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut result = Self {
            storage: vec![0; size.div_ceil(std::mem::size_of::<usize>())],
            initialized: false,
        };
        if unsafe { InitializeProcThreadAttributeList(result.as_mut_ptr(), 1, 0, &mut size) } == 0 {
            return Err(io::Error::last_os_error());
        }
        result.initialized = true;
        if unsafe {
            UpdateProcThreadAttribute(
                result.as_mut_ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(handles),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(result)
    }

    /// 【Windows内部进程】【列表地址】无参数，返回存储期间固定的启动属性列表地址
    pub fn as_mut_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }
}

impl Drop for Attributes {
    /// 【Windows内部进程】【列表释放】无参数，释放系统属性资源后由字段回收存储，无返回值
    fn drop(&mut self) {
        if self.initialized {
            unsafe { DeleteProcThreadAttributeList(self.as_mut_ptr()) };
        }
    }
}
