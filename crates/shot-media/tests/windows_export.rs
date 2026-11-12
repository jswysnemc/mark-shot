//! 【媒体导出回归】【系统编码器兼容】读取 Windows 原生编码器夹具，验证跨实现容器与音视频封装

#![cfg(target_os = "windows")]

/// 【媒体导出回归】【实际转换】无参数，使用环境指定的原生 MP4 生成 MKV 与 GIF，输出供独立解码
#[test]
#[ignore = "requires native Windows fixture and MARK_SHOT_MEDIA_LIBRARY"]
fn exports_accept_native_windows_recording() {
    let directory = std::path::PathBuf::from(std::env::var_os("MARK_SHOT_MEDIA_EVIDENCE").unwrap());
    let source = directory.join("native-encoding.mp4");
    let media = shot_media::Media::load().unwrap();
    media
        .export(&source, &directory.join("native-export.mkv"), None, || {
            false
        })
        .unwrap();
    media
        .export(
            &source,
            &directory.join("native-export.gif"),
            Some(12),
            || false,
        )
        .unwrap();
}
