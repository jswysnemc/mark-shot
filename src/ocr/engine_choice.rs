//! 【识别引擎】【用户选择】持久化明确选择，系统识别与下载模型使用不同输入，不隐式切换引擎

use super::{
    OcrBackend, OcrError,
    models::{ModelFiles, OcrModels},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Ask,
    Paddle,
    System,
}

impl Default for Engine {
    /// 【识别引擎】【平台默认】无参数，Windows 首次要求选择，其他平台显示本地模型安装入口
    fn default() -> Self {
        if cfg!(target_os = "windows") {
            Self::Ask
        } else {
            Self::Paddle
        }
    }
}

#[derive(Clone, Debug)]
pub enum Source {
    Paddle(ModelFiles),
    System,
}

impl From<ModelFiles> for Source {
    /// 【识别引擎】【模型输入】将 files 转为明确的本地模型来源，返回任务输入
    fn from(files: ModelFiles) -> Self {
        Self::Paddle(files)
    }
}

/// 【识别引擎】【当前选择】无参数，返回当前有效配置，不读取或安装推理库
pub fn current() -> Engine {
    crate::config::preferences::current().ocr.engine
}

/// 【识别引擎】【保存选择】保存 engine 并发布当前配置，返回持久化错误，不触发下载
pub fn select(engine: Engine) -> Result<(), String> {
    if engine == Engine::System && !cfg!(target_os = "windows") {
        return Err("当前平台不支持 Windows OCR".into());
    }
    let path = crate::config::preferences::path().map_err(|error| error.to_string())?;
    crate::config::preferences::write(
        &path,
        "ocr",
        "engine",
        serde_json::to_value(engine).map_err(|error| error.to_string())?,
    )
    .and_then(|()| crate::config::preferences::reload(&path))
    .map_err(|error| error.to_string())
}

/// 【识别引擎】【任务准备】从 models 与当前用户选择返回已准备的任务来源，缺少资源或尚未选择时返回空值
pub fn source(models: &OcrModels) -> Option<Source> {
    match current() {
        Engine::System if cfg!(target_os = "windows") => Some(Source::System),
        Engine::Paddle if models.library.status == super::models::ModelStatus::Installed => models
            .active()
            .and_then(|index| models.files(index))
            .map(Source::Paddle),
        _ => None,
    }
}

/// 【识别引擎】【后端创建】按 source 和 mode 创建所选后端，系统后端不读取模型与第三方运行库
pub fn build(
    source: &Source,
    mode: super::settings::Mode,
) -> Result<Box<dyn OcrBackend>, OcrError> {
    match source {
        Source::Paddle(files) => super::build_backend(mode, files),
        Source::System => {
            #[cfg(target_os = "windows")]
            {
                super::windows_backend::languages()?;
                Ok(Box::new(super::windows_backend::NativeBackend))
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("Windows system OCR is unavailable on this platform".into())
            }
        }
    }
}
