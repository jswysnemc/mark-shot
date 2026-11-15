//! 【KDE窗口识别】【会话身份】验证直接显示服务与 KDE 包装器转交的显示连接

use super::super::Result;
use std::path::Path;

/// 【KDE窗口识别】【父进程解析】按 stat 提取内核记录的父进程，返回有效进程号，进程名称中的括号不参与字段解析
fn parent(stat: &str) -> Option<u32> {
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// 【KDE窗口识别】【显示归属】按服务进程 pid、对端 peer_pid 与两端用户编号校验直接连接或 KWin 包装器父子链，返回是否匹配
pub(super) fn matches(pid: u32, peer_pid: i32, uid: u32, peer_uid: u32) -> Result<bool> {
    if peer_pid <= 0 || uid != peer_uid {
        return Ok(false);
    }
    if pid == peer_pid as u32 {
        return Ok(true);
    }
    // 1. 【KDE窗口识别】【包装器交接】KDE 的监听套接字由父包装器创建，必须确认总线 KWin 是该包装器的子进程
    let process = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    if parent(&process) != Some(peer_pid as u32) {
        return Ok(false);
    }
    let executable = std::fs::read_link(format!("/proc/{peer_pid}/exe"))?;
    Ok(wrapper(&executable))
}

/// 【KDE窗口识别】【包装器名称】按 executable 判断真实包装器路径，返回匹配结果；升级后内核添加的删除标记不改变运行身份
fn wrapper(executable: &Path) -> bool {
    executable.file_name().is_some_and(|name| {
        name == "kwin_wayland_wrapper" || name == "kwin_wayland_wrapper (deleted)"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【KDE窗口识别回归】【身份拒绝】无参数，验证直接身份、跨用户和无效对端，返回值为空
    #[test]
    fn direct_peer_requires_the_same_user() {
        assert!(matches(20, 20, 1000, 1000).unwrap());
        assert!(!matches(20, 20, 1001, 1000).unwrap());
        assert!(!matches(20, 0, 1000, 1000).unwrap());
        assert!(
            !matches(
                std::process::id(),
                std::process::id() as i32 + 1,
                1000,
                1000
            )
            .unwrap()
        );
    }

    /// 【KDE窗口识别回归】【内核字段】无参数，验证包含空格和括号的进程名仍正确解析父进程，返回值为空
    #[test]
    fn process_parent_uses_fields_after_the_last_name_parenthesis() {
        assert_eq!(parent("874 (kwin (wayland)) S 867 826 826"), Some(867));
        assert_eq!(parent("broken"), None);
        assert_eq!(parent("874 (kwin) S invalid"), None);
        assert!(wrapper(Path::new("/usr/bin/kwin_wayland_wrapper")));
        assert!(wrapper(Path::new(
            "/usr/bin/kwin_wayland_wrapper (deleted)"
        )));
        assert!(!wrapper(Path::new("/usr/bin/other_wrapper")));
    }
}
