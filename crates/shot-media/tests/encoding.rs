//! 【媒体集成测试】【真实文件】通过公开接口生成音视频、导出与取消，不在测试中模拟编码器

use shot_media::{Encoder, Media, Settings};

/// 【媒体集成测试】【编码参数】无参数，返回确定性小尺寸软件编码配置
fn settings() -> Settings {
    Settings {
        device: None,
        width: 320,
        height: 240,
        fps: 30,
        quality: 1,
        mode: 2,
        audio: true,
    }
}

/// 【媒体集成测试】【完整链路】无参数，写入运动色块和音轨并转换为 MKV/GIF，产物交给独立解码器验证
#[test]
#[ignore = "requires MARK_SHOT_MEDIA_LIBRARY and MARK_SHOT_MEDIA_EVIDENCE"]
fn encoding_and_exports_use_the_in_process_library() {
    let root = std::path::PathBuf::from(std::env::var_os("MARK_SHOT_MEDIA_EVIDENCE").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let media = Media::load().unwrap();
    let path = root.join("native.mp4");
    let mut encoder = Encoder::open(&media, &path, &settings()).unwrap();
    assert_eq!(encoder.name(), "libopenh264");
    let mut pixels = vec![0; 320 * 240 * 4];
    for frame in 0..60 {
        for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel.copy_from_slice(&[
                if index % 320 >= 160 { 255 } else { 0 },
                if index / 320 >= 120 {
                    220
                } else {
                    frame as u8 * 2
                },
                if index % 320 < 160 { 255 } else { 0 },
                255,
            ]);
        }
        encoder
            .video(&pixels, 320 * 4, frame * 1_000_000 / 30)
            .unwrap();
        let pcm: Vec<f32> = (0..1600)
            .flat_map(|sample| {
                let value = (((frame * 1600 + sample) as f32 * std::f32::consts::TAU * 440.0)
                    / 48000.0)
                    .sin()
                    * 0.15;
                [value, value]
            })
            .collect();
        encoder.audio(&pcm, frame * 1_000_000 / 30).unwrap();
    }
    encoder.finish().unwrap();
    media
        .export(&path, &root.join("native.mkv"), None, || false)
        .unwrap();
    media
        .export(&path, &root.join("native.gif"), Some(12), || false)
        .unwrap();
    assert!(
        media
            .export(&path, &root.join("cancelled.gif"), Some(12), || true)
            .is_err()
    );
    assert!(encoder.video(&pixels, 1280, 2_000_000).is_err());
    assert!(encoder.video(&pixels[..30], 1280, 2_000_000).is_err());
    assert!(encoder.audio(&[0.0], 0).is_err());
}

/// 【媒体集成测试】【异常收尾】无参数，不显式 finish 也保留可解码的分片文件
#[test]
#[ignore = "requires MARK_SHOT_MEDIA_LIBRARY and MARK_SHOT_MEDIA_EVIDENCE"]
fn dropping_an_encoder_flushes_the_file() {
    let root = std::path::PathBuf::from(std::env::var_os("MARK_SHOT_MEDIA_EVIDENCE").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let media = Media::load().unwrap();
    let mut options = settings();
    options.audio = false;
    let mut encoder = Encoder::open(&media, &root.join("dropped.mp4"), &options).unwrap();
    let pixels = vec![128; 320 * 240 * 4];
    for frame in 0..60 {
        encoder
            .video(&pixels, 1280, frame * 1_000_000 / 30)
            .unwrap();
    }
    drop(encoder);
    options.width = 3;
    assert!(Encoder::open(&media, &root.join("invalid.mp4"), &options).is_err());
    assert!(!root.join("invalid.mp4").exists());
}

/// 【媒体集成测试】【音频间断】无参数，生成带半秒采集中断的音轨，独立解码必须保留静音间隔
#[test]
#[ignore = "requires MARK_SHOT_MEDIA_LIBRARY and MARK_SHOT_MEDIA_EVIDENCE"]
fn audio_timestamps_preserve_capture_gaps() {
    let root = std::path::PathBuf::from(std::env::var_os("MARK_SHOT_MEDIA_EVIDENCE").unwrap());
    let media = Media::load().unwrap();
    let mut encoder = Encoder::open(&media, &root.join("audio-gap.mp4"), &settings()).unwrap();
    let pixels = vec![128; 320 * 240 * 4];
    for frame in 0..60 {
        encoder
            .video(&pixels, 1280, frame * 1_000_000 / 30)
            .unwrap();
        if !(15..30).contains(&frame) {
            let samples: Vec<_> = (0..1600)
                .flat_map(|sample| {
                    let value =
                        (sample as f32 * std::f32::consts::TAU * 440.0 / 48000.0).sin() * 0.15;
                    [value, value]
                })
                .collect();
            encoder.audio(&samples, frame * 1_000_000 / 30).unwrap();
        }
    }
    encoder.finish().unwrap();
}
