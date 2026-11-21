//! 【截图另存为】【桌面门户】通过系统文件选择器门户获取保存路径，不使用自绘文件浏览组件

use std::path::{Path, PathBuf};

/// 【截图另存为】【门户目录】从 initial_path 向上查找最近存在的目录，不存在时回退图片目录或主目录
pub(super) fn resolve_initial_folder(initial_path: &Path) -> Option<PathBuf> {
    initial_path
        .parent()
        .and_then(|parent| {
            parent
                .ancestors()
                .find(|candidate| !candidate.as_os_str().is_empty() && candidate.is_dir())
        })
        .map(Path::to_path_buf)
        .or_else(|| dirs::picture_dir().filter(|dir| dir.is_dir()))
        .or_else(|| dirs::home_dir().filter(|dir| dir.is_dir()))
}

/// 【截图另存为】【初始文件名】提取 initial_path 的文件名，缺失时提供标准 PNG 默认名
pub(super) fn resolve_initial_name(initial_path: &Path) -> String {
    initial_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "screenshot.png".to_owned())
}

/// 【截图另存为】【文档门户识别】判断 path 是否位于 XDG 文档门户 FUSE 挂载下，避免改名破坏门户授权句柄
pub(super) fn is_document_portal_path(path: &Path) -> bool {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let doc_root = Path::new(&runtime_dir).join("doc");
        if path.starts_with(&doc_root) {
            return true;
        }
    }
    path.components()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair[0].as_os_str() == "user" && pair[1].as_os_str() != "doc")
        && path.to_string_lossy().contains("/doc/")
}

/// 【截图另存为】【URI转路径】解析门户返回的 file URI 并补齐缺失的 png 后缀，文档门户路径保持原名
#[cfg(any(target_os = "linux", test))]
pub(super) fn parse_selected_uri(uri: &str) -> crate::AppResult<PathBuf> {
    let parsed = url::Url::parse(uri).map_err(|err| format!("invalid portal URI: {err}"))?;
    if parsed.scheme() != "file" {
        return Err(format!("unsupported portal URI scheme: {}", parsed.scheme()).into());
    }
    let path = parsed
        .to_file_path()
        .map_err(|()| "cannot convert portal file URI to local path")?;
    Ok(normalize_selected_path(path))
}

/// 【截图另存为】【后缀规范化】为非文档门户且无扩展名的目标路径自动补充 png 扩展名，返回最终路径
pub(super) fn normalize_selected_path(path: PathBuf) -> PathBuf {
    if path.extension().is_none() && !is_document_portal_path(&path) {
        path.with_extension("png")
    } else {
        path
    }
}

/// 【截图另存为】【Linux门户调用】通过 org.freedesktop.portal.FileChooser SaveFile 请求用户选择保存路径
#[cfg(target_os = "linux")]
pub fn choose_save_path(initial_path: &Path) -> crate::AppResult<Option<PathBuf>> {
    use ashpd::{
        Error, PortalError,
        desktop::{
            ResponseError,
            file_chooser::{FileFilter, SelectedFiles},
        },
    };

    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let title = if chinese {
        "保存截图"
    } else {
        "Save Screenshot"
    };
    let accept = if chinese { "保存" } else { "Save" };
    let filter_label = if chinese {
        "PNG 图像 (*.png)"
    } else {
        "PNG Image (*.png)"
    };
    let folder = resolve_initial_folder(initial_path);
    let current_name = resolve_initial_name(initial_path);

    // 1. 【截图另存为】【运行时隔离】在独立线程驱动门户请求，允许调用方已有 Tokio 运行时
    let outcome = super::runtime::block_on(async move {
        // 1. 【文件门户】【独立连接】临时运行时独立持有总线连接，关闭对话框后不污染其他门户请求
        let connection = zbus::Connection::session().await?;
        let png_filter = FileFilter::new(filter_label)
            .glob("*.png")
            .mimetype("image/png");
        let mut request = SelectedFiles::save_file()
            .connection(Some(connection))
            .title(title)
            .accept_label(accept)
            .modal(true)
            .current_name(current_name.as_str())
            .filter(png_filter.clone())
            .current_filter(png_filter);
        if let Some(dir) = folder.as_deref() {
            request = request.current_folder(dir)?;
        }
        request.send().await?.response()
    })?;

    // 2. 区分用户主动取消与门户异常，成功时转换首个授权 URI 为本地路径
    match outcome {
        Ok(files) => {
            let Some(uri) = files.uris().first() else {
                return Ok(None);
            };
            let path = parse_selected_uri(uri.as_str())?;
            log::info!("【截图另存为】【门户选择】目标路径={}", path.display());
            Ok(Some(path))
        }
        Err(Error::Response(ResponseError::Cancelled))
        | Err(Error::Portal(PortalError::Cancelled(_))) => Ok(None),
        Err(err) => Err(format!("desktop file chooser portal error: {err}").into()),
    }
}

/// 【截图另存为】【Windows原生对话框】通过系统 IFileSaveDialog 选择 PNG 保存路径，取消返回空值
#[cfg(target_os = "windows")]
pub fn choose_save_path(initial_path: &Path) -> crate::AppResult<Option<PathBuf>> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::ERROR_CANCELLED,
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize,
            },
            UI::Shell::{
                Common::COMDLG_FILTERSPEC, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT,
                FOS_PATHMUSTEXIST, FileSaveDialog, IFileSaveDialog, IShellItem,
                SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
            },
        },
        core::{HRESULT, PCWSTR},
    };

    fn wide_null(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    let chinese =
        crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let title = wide_null(std::ffi::OsStr::new(if chinese {
        "保存截图"
    } else {
        "Save Screenshot"
    }));
    let filter_name = wide_null(std::ffi::OsStr::new(if chinese {
        "PNG 图像 (*.png)"
    } else {
        "PNG Image (*.png)"
    }));
    let filter_spec = wide_null(std::ffi::OsStr::new("*.png"));
    let default_ext = wide_null(std::ffi::OsStr::new("png"));
    let current_name = wide_null(std::ffi::OsStr::new(&resolve_initial_name(initial_path)));
    let folder = resolve_initial_folder(initial_path);

    // 1. 初始化当前线程单套间 COM 环境并配置系统保存对话框过滤与覆盖确认策略
    let hr_init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninit = hr_init.is_ok();

    let result = (|| -> crate::AppResult<Option<PathBuf>> {
        let dialog: IFileSaveDialog =
            unsafe { CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER) }?;
        let options = unsafe { dialog.GetOptions() }?;
        unsafe {
            dialog.SetOptions(
                options | FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM,
            )?;
            dialog.SetTitle(PCWSTR(title.as_ptr()))?;
            dialog.SetFileName(PCWSTR(current_name.as_ptr()))?;
            dialog.SetDefaultExtension(PCWSTR(default_ext.as_ptr()))?;
            let specs = [COMDLG_FILTERSPEC {
                pszName: PCWSTR(filter_name.as_ptr()),
                pszSpec: PCWSTR(filter_spec.as_ptr()),
            }];
            dialog.SetFileTypes(&specs)?;
            dialog.SetFileTypeIndex(1)?;
            if let Some(dir) = folder.as_deref() {
                let dir_wide = wide_null(dir.as_os_str());
                if let Ok(item) =
                    SHCreateItemFromParsingName::<_, _, IShellItem>(PCWSTR(dir_wide.as_ptr()), None)
                {
                    let _ = dialog.SetFolder(&item);
                }
            }
        }

        // 2. 显示系统模态文件对话框并转换用户选定的文件系统路径
        let cancelled_hr = HRESULT::from_win32(ERROR_CANCELLED.0);
        if let Err(err) = unsafe { dialog.Show(None) } {
            if err.code() == cancelled_hr {
                return Ok(None);
            }
            return Err(format!("system file save dialog error: {err}").into());
        }
        let item = unsafe { dialog.GetResult() }?;
        let raw_path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }?;
        let text = unsafe { raw_path.to_string() }
            .map_err(|err| format!("invalid selected file path: {err}"))?;
        unsafe { CoTaskMemFree(Some(raw_path.0 as *const _)) };
        Ok(Some(normalize_selected_path(PathBuf::from(text))))
    })();

    if should_uninit {
        unsafe { CoUninitialize() };
    }
    result
}
