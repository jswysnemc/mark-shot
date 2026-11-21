//! 【Windows内部进程】【环境交接】继承完整Windows环境块，按不区分大小写的变量名应用显式覆盖

use super::command_line::wide;
use std::{cmp::Ordering, io, process::Command};
use windows_sys::Win32::{
    Globalization::{CSTR_EQUAL, CSTR_LESS_THAN, CompareStringOrdinal},
    System::Environment::{FreeEnvironmentStringsW, GetEnvironmentStringsW},
};

struct Inherited(*mut u16);

impl Drop for Inherited {
    /// 【Windows内部进程】【环境释放】无参数，归还系统环境快照，无返回值
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { FreeEnvironmentStringsW(self.0) };
        }
    }
}

/// 【Windows内部进程】【变量比较】按 left 与 right 返回Windows不区分大小写的排序结果，用于覆盖与环境块排序
fn compare(left: &[u16], right: &[u16]) -> Ordering {
    match unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        )
    } {
        CSTR_LESS_THAN => Ordering::Less,
        CSTR_EQUAL => Ordering::Equal,
        _ => Ordering::Greater,
    }
}

/// 【Windows内部进程】【变量名称】按 entry 返回等号之前的变量名；盘符工作目录保留首个等号
fn key(entry: &[u16]) -> &[u16] {
    let start = usize::from(entry.first() == Some(&u16::from(b'=')));
    let end = entry[start..]
        .iter()
        .position(|&unit| unit == u16::from(b'='))
        .map_or(entry.len(), |i| i + start);
    &entry[..end]
}

/// 【Windows内部进程】【环境装配】按 command 的显式变量变更更新继承环境，返回双零结尾UTF-16环境块
pub(super) fn build(command: &Command) -> io::Result<Vec<u16>> {
    let inherited = Inherited(unsafe { GetEnvironmentStringsW() });
    if inherited.0.is_null() {
        return Err(io::Error::last_os_error());
    }
    let mut entries = Vec::new();
    let mut cursor = inherited.0;
    // 1. 【Windows内部进程】【完整继承】系统快照包含盘符工作目录，复制时保留所有UTF-16代码单元
    unsafe {
        while *cursor != 0 {
            let begin = cursor;
            while *cursor != 0 {
                cursor = cursor.add(1);
            }
            entries.push(
                std::slice::from_raw_parts(begin, cursor.offset_from(begin) as usize).to_vec(),
            );
            cursor = cursor.add(1);
        }
    }
    // 2. 【Windows内部进程】【单次覆盖】大小写不同的同名变量只保留本次设置，显式移除不传播到子进程
    for (name, value) in command.get_envs() {
        let name = wide(name)?;
        let name = &name[..name.len() - 1];
        if name.is_empty() || name.contains(&u16::from(b'=')) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid environment variable name",
            ));
        }
        entries.retain(|entry| compare(key(entry), name) != Ordering::Equal);
        if let Some(value) = value {
            let value = wide(value)?;
            let mut entry = name.to_vec();
            entry.push(u16::from(b'='));
            entry.extend_from_slice(&value[..value.len() - 1]);
            entries.push(entry);
        }
    }
    entries.sort_by(|left, right| compare(key(left), key(right)));
    let mut block = Vec::new();
    for entry in entries {
        block.extend(entry);
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    Ok(block)
}
