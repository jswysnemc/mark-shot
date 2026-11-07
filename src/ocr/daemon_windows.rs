//! 【Windows识别】【本地引擎】内置模型按需加载，当前平台不创建Unix识别服务

#[path = "daemon/protocol.rs"]
pub mod protocol;
pub const DAEMON_ARG: &str = "--ocr-daemon";

/// 【Windows识别】【模式选择】按settings保留按需或启动加载模式，常驻请求明确回退为按需模式，返回实际模式
pub fn resolve_mode(settings: &super::settings::EngineSettings) -> super::settings::Mode {
    if settings.mode == super::settings::Mode::Daemon {
        log::info!("【Windows识别】【模式选择】常驻识别服务尚未接入，使用内置按需引擎");
        super::settings::Mode::OnDemand
    } else {
        settings.mode
    }
}
pub mod client {
    /// 【Windows识别】【引擎连接】按files创建内置CPU后端，返回可取消的识别引擎，不启动常驻进程
    pub fn connect(
        files: &crate::ocr::models::ModelFiles,
    ) -> Result<Box<dyn crate::ocr::OcrBackend>, crate::ocr::OcrError> {
        crate::ocr::default_backend(files)
    }
}
/// 【Windows识别】【服务命令】按args拒绝尚未接入的常驻服务命令，返回明确错误，正常OCR仍使用内置引擎
pub fn cli(_args: impl Iterator<Item = String>) -> crate::AppResult {
    Err("the Windows persistent OCR service is not available; use on-demand OCR".into())
}
