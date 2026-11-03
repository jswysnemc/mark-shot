use std::fmt;
use std::path::{Path, PathBuf};

use super::models::{Asset, DETECTOR, MODELS, ModelFiles};

#[derive(Debug)]
pub struct IntegrityError {
    pub path: PathBuf,
    pub reason: String,
}

impl IntegrityError {
    pub const PREFIX: &str = "OCR model verification failed:";
}

impl fmt::Display for IntegrityError {
    /// 【识别模型】【校验错误】将损坏文件路径和原因格式化为可诊断信息
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            Self::PREFIX,
            self.path.display(),
            self.reason
        )
    }
}

impl std::error::Error for IntegrityError {}

/// 【识别模型】【文件校验】检查 path 的长度与 asset 校验和；返回具体损坏文件及原因
pub fn verify_asset(path: &Path, asset: &Asset) -> Result<(), IntegrityError> {
    let fail = |reason: String| IntegrityError {
        path: path.to_path_buf(),
        reason,
    };
    let mut file = std::fs::File::open(path).map_err(|error| fail(error.to_string()))?;
    let size = file
        .metadata()
        .map_err(|error| fail(error.to_string()))?
        .len();
    if size != asset.size {
        return Err(fail(format!("expected {} bytes, found {size}", asset.size)));
    }
    if !super::download::matches_sha256(&mut file, asset.sha256)
        .map_err(|error| fail(error.to_string()))?
    {
        return Err(fail("SHA-256 mismatch; download the model again".into()));
    }
    Ok(())
}

/// 【识别模型】【引擎校验】首次创建识别引擎时校验三个模型资源；普通截图仅读取文件元信息
pub fn verify(files: &ModelFiles) -> Result<(), IntegrityError> {
    let model = MODELS
        .iter()
        .find(|model| {
            files
                .recognizer
                .file_name()
                .is_some_and(|name| name == model.recognizer.file)
        })
        .ok_or_else(|| IntegrityError {
            path: files.recognizer.clone(),
            reason: "unknown bundled recognition model".into(),
        })?;
    verify_asset(&files.detector, &DETECTOR)?;
    verify_asset(&files.recognizer, &model.recognizer)?;
    verify_asset(&files.dict, &model.dict)
}
