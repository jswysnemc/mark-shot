//! 【Wayland窗口】【应用身份】KDE 使用安装桌面条目的应用标识，内部窗口职责仍由各自规格保存

pub(crate) const APP_ID: &str = "io.github.jswysnemc.MarkShot";

/// 【Wayland窗口】【桌面匹配】按 requested 与 desktop 返回协议应用标识，KDE 匹配桌面图标，其他桌面保持现有身份
fn for_desktop<'a>(requested: &'a str, desktop: &str) -> &'a str {
    if desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE") || name.eq_ignore_ascii_case("PLASMA"))
    {
        APP_ID
    } else {
        requested
    }
}

/// 【Wayland窗口】【当前身份】按 requested 返回当前桌面协议应用标识，不更改内部窗口编号或单实例标识
pub(crate) fn current(requested: &str) -> String {
    for_desktop(
        requested,
        &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
    )
    .into()
}

#[cfg(test)]
mod tests {
    /// 【Wayland窗口测试】【图标身份】无参数，KDE 各窗口匹配已安装桌面条目，其他桌面保留窗口职责标识，无返回值
    #[test]
    fn kde_windows_match_the_packaged_desktop_entry() {
        for desktop in ["KDE", "PLASMA", "ubuntu:plasma"] {
            for requested in ["mark-shot", "mark-shot-settings", "mark-shot-ocr-resources"] {
                assert_eq!(super::for_desktop(requested, desktop), super::APP_ID);
            }
        }
        assert_eq!(
            super::for_desktop("mark-shot-settings", "GNOME"),
            "mark-shot-settings"
        );
        assert!(
            include_str!("../../../packaging/desktop/io.github.jswysnemc.MarkShot.desktop")
                .contains(&format!("Icon={}", super::APP_ID))
        );
    }
}
