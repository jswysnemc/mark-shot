//! 【识别模型】【下载镜像】解析环境变量与配置的 GitHub 镜像前缀，生成镜像优先与直连兜底的下载候选列表

/// 识别资源下载镜像环境变量名
pub const ENV_VAR: &str = "MARK_SHOT_OCR_MIRROR";

/// 预设 GitHub 发行产物加速镜像一
pub const GHFAST: &str = "https://ghfast.top/";

/// 预设 GitHub 发行产物加速镜像二
pub const GHPROXY: &str = "https://gh-proxy.com/";

/// 【识别模型】【镜像前缀读取】优先读取环境变量，其次读取用户配置中的下载镜像前缀，返回去除空白后的前缀
pub fn configured_prefix() -> String {
    if let Ok(value) = std::env::var(ENV_VAR)
        && !value.trim().is_empty()
    {
        return value.trim().to_owned();
    }
    crate::config::preferences::current()
        .ocr
        .download_mirror
        .trim()
        .to_owned()
}

/// 【识别模型】【地址拼接】将合法的 HTTP/HTTPS 镜像前缀 prefix 应用于 GitHub 原始地址 original_url，返回转换后的地址
pub fn apply_prefix(prefix: &str, original_url: &str) -> String {
    let prefix = prefix.trim();
    if prefix.is_empty()
        || (!prefix.starts_with("https://") && !prefix.starts_with("http://"))
        || !original_url.starts_with("https://github.com/")
    {
        return original_url.to_owned();
    }
    if prefix.ends_with('/') {
        format!("{prefix}{original_url}")
    } else {
        format!("{prefix}/{original_url}")
    }
}

/// 【识别模型】【候选地址】按给定 prefix 与原始 GitHub 地址生成有序候选，启用镜像时自动将官方直连作为回退项
pub fn candidates_with_prefix(prefix: &str, original_url: &str) -> Vec<String> {
    let primary = apply_prefix(prefix, original_url);
    if primary == original_url {
        vec![original_url.to_owned()]
    } else {
        vec![primary, original_url.to_owned()]
    }
}

/// 【识别模型】【当前候选】按当前生效镜像配置为 original_url 生成下载候选地址列表
pub fn candidates(original_url: &str) -> Vec<String> {
    candidates_with_prefix(&configured_prefix(), original_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【镜像测试】【前缀拼接】验证空前缀、缺少斜杠前缀与非 GitHub 地址均生成确定且安全的候选顺序
    #[test]
    fn mirror_prefix_rewrites_github_urls_and_keeps_direct_fallback() {
        let direct = "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0/ppocrv5_dict.txt";
        assert_eq!(candidates_with_prefix("", direct), vec![direct.to_owned()]);
        assert_eq!(
            candidates_with_prefix("https://ghfast.top", direct),
            vec![format!("https://ghfast.top/{direct}"), direct.to_owned()]
        );
        assert_eq!(
            candidates_with_prefix(GHFAST, direct),
            vec![format!("{GHFAST}{direct}"), direct.to_owned()]
        );
        assert_eq!(
            apply_prefix(GHFAST, "http://127.0.0.1:8080/model.onnx"),
            "http://127.0.0.1:8080/model.onnx"
        );
        assert_eq!(apply_prefix("ftp://invalid", direct), direct);
    }
}
