//! 【贴图配置】【菜单偏好】按字段保存置顶与自动划词开关，不覆盖其他窗口的配置更新

use std::{io, path::Path};

#[derive(Clone, Copy)]
pub(crate) enum Preference {
    AlwaysOnTop,
    AutoTextSelection,
}

impl Preference {
    /// 【贴图配置】【字段路径】无参数，返回当前菜单偏好的稳定配置字段
    fn key(self) -> &'static str {
        match self {
            Self::AlwaysOnTop => "always_on_top",
            Self::AutoTextSelection => "auto_text_selection",
        }
    }
}

/// 【贴图配置】【指定保存】将 preference 和 enabled 写入 path 的单一字段，返回原子保存结果
pub(crate) fn save_at(path: &Path, preference: Preference, enabled: bool) -> io::Result<()> {
    super::preferences::write(path, "pinned", preference.key(), enabled.into())
}

/// 【贴图配置】【当前保存】将 preference 和 enabled 保存到当前配置并发布新快照，返回保存或重载错误
pub(crate) fn save(preference: Preference, enabled: bool) -> io::Result<()> {
    let path = super::preferences::path()?;
    save_at(&path, preference, enabled)?;
    super::preferences::reload(&path)
}
