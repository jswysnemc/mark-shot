//! 【上传配置】【兼容读取】沿用原版命令、超时和环境别名，凭据只在用户主动上传时读取

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, io, path::Path};

#[derive(Clone, PartialEq, Serialize)]
pub struct Upload {
    pub command: String,
    pub timeout_ms: u64,
    pub env: BTreeMap<String, String>,
    #[serde(skip)]
    pub command_shell: String,
}

impl Default for Upload {
    /// 【上传配置】【默认参数】返回原版六十秒超时与内置服务入口，无参数，不指定远程服务或读取凭据
    fn default() -> Self {
        Self {
            command: String::new(),
            timeout_ms: 60000,
            env: BTreeMap::new(),
            command_shell: "/bin/sh".into(),
        }
    }
}

impl fmt::Debug for Upload {
    /// 【上传配置】【调试脱敏】向 formatter 写入超时与命令存在性，返回格式结果，不输出命令和环境内容
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Upload")
            .field("has_command", &!self.command.is_empty())
            .field("timeout_ms", &self.timeout_ms)
            .field("env", &"[redacted]")
            .finish()
    }
}

impl<'de> Deserialize<'de> for Upload {
    /// 【上传配置】【字段解码】按 deserializer 读取原版兼容对象，返回独立配置；错误类型不隐式转换
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_value(&Value::deserialize(deserializer)?))
    }
}

impl Upload {
    /// 【上传配置】【对象读取】按 value 读取 command/path/helper、环境别名及超时，返回规范化配置
    pub fn from_value(value: &Value) -> Self {
        let command = ["command", "path", "helper"]
            .iter()
            .find_map(|key| value.get(key).and_then(Value::as_str))
            .unwrap_or_default()
            .trim()
            .to_owned();
        let environment = ["env", "environment", "envVars", "variables"]
            .iter()
            .find_map(|key| value.get(key).and_then(Value::as_object));
        let env = environment
            .into_iter()
            .flatten()
            .filter_map(|(key, value)| {
                let key = key.trim();
                (!key.is_empty())
                    .then(|| {
                        value
                            .as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .flatten()
            })
            .collect();
        let timeout_ms = value
            .get("timeout_ms")
            .or_else(|| value.get("timeoutMs"))
            .and_then(Value::as_f64)
            .filter(|value| {
                value.fract() == 0.0 && *value >= i32::MIN as f64 && *value <= i32::MAX as f64
            })
            .map(|value| (value as i32).max(1000) as u64)
            .unwrap_or(60000);
        Self {
            command,
            timeout_ms,
            env,
            command_shell: "/bin/sh".into(),
        }
    }

    /// 【上传配置】【顶层别名】按 root 读取原版四种上传分组，返回配置；首个实际对象优先，即使内容为空
    pub fn from_root(root: &Value) -> Self {
        ["upload", "imageUpload", "uploader", "imageHost"]
            .iter()
            .find_map(|key| root.get(key).filter(|value| value.is_object()))
            .map(Self::from_value)
            .unwrap_or_default()
    }

    /// 【上传配置】【环境覆盖】按 system 生成本次有效配置，返回快照；系统命令和超时优先，配置环境覆盖传输环境
    pub fn effective(&self, system: &BTreeMap<String, String>) -> Self {
        let mut result = self.clone();
        if let Some(command) = system
            .get("MARK_SHOT_UPLOAD_COMMAND")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            result.command = command.to_owned();
        }
        if let Some(timeout) = system
            .get("MARK_SHOT_UPLOAD_TIMEOUT_MS")
            .and_then(|value| value.trim().parse::<i32>().ok())
        {
            result.timeout_ms = timeout.max(1000) as u64;
        }
        result.env = system.clone();
        result.env.extend(self.env.clone());
        result.command_shell = system
            .get("SHELL")
            .filter(|value| !value.is_empty())
            .cloned()
            .unwrap_or_else(|| "/bin/sh".into());
        result
    }
}

/// 【上传配置】【独立文件】从 path 读取 TOML 或原版 JSON，返回上传分组，不修改用户文件
pub fn read_at(path: &Path) -> io::Result<Upload> {
    let text = super::persistence::read(path)?;
    let root = super::document::root(path, &text)?;
    Ok(Upload::from_root(&root))
}

/// 【上传配置】【主动读取】返回当前配置中的上传参数，无参数；文件缺失沿用启动配置，格式错误原样返回
pub fn current() -> io::Result<Upload> {
    let path = super::OVERRIDE_PATH
        .get()
        .cloned()
        .or_else(super::loading::config_path);
    match path.map(|path| read_at(&path)).transpose() {
        Ok(Some(config)) => Ok(config),
        Ok(None) => Ok(super::get().upload.clone()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(super::get().upload.clone()),
        Err(error) => Err(error),
    }
}
