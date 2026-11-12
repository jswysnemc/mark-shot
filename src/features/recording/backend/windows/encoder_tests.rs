//! 【Windows录制回归】【实际编码】验证异常收尾及 GPU NV12 样本提交，产物供解码验证

use super::super::{converter::Converter, device, software};
use super::*;

/// 【Windows录制回归】【恢复文件】无参数，提前释放编码器仍写出 MP4，返回产物由验证脚本完整解码
#[test]
#[ignore = "requires Windows Media Foundation runtime"]
fn media_foundation_finalizes_on_early_exit() {
    let directory =
        std::env::var_os("MARK_SHOT_RECORDING_EVIDENCE").expect("private evidence directory");
    std::fs::create_dir_all(&directory).unwrap();
    let path = Path::new(&directory).join("early-exit.mp4");
    let _runtime = device::Runtime::new().unwrap();
    let config = Recording {
        audio: "off".into(),
        fps: 30,
        ..Default::default()
    };
    let encoder = Encoder::new(None, &path, (320, 240), &config, false).unwrap();
    for frame in 0..60 {
        let pixels = vec![128; 320 * 240 * 4];
        encoder
            .video(&software::sample(&pixels, frame * 10_000_000 / 30, 10_000_000 / 30).unwrap())
            .unwrap();
    }
    drop(encoder);
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.windows(4).any(|value| value == b"moov"));
}

/// 【Windows录制回归】【GPU样本】无参数，在真实视频设备把 BGRA 转为池化 NV12 并提交，与软件夹具覆盖不同路径
#[test]
#[ignore = "requires Windows D3D11 video hardware and Media Foundation runtime"]
fn media_foundation_encodes_gpu_nv12_samples() {
    use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*};
    let directory =
        std::env::var_os("MARK_SHOT_RECORDING_EVIDENCE").expect("private evidence directory");
    std::fs::create_dir_all(&directory).unwrap();
    let path = Path::new(&directory).join("gpu-nv12.mp4");
    let _runtime = device::Runtime::new().unwrap();
    let device = device::Device::new().unwrap();
    let config = Recording {
        audio: "off".into(),
        fps: 30,
        ..Default::default()
    };
    let converter = Converter::new(&device, (320, 240), [0, 0, 320, 240], 30).unwrap();
    let encoder = Encoder::new(Some(&device), &path, (320, 240), &config, true).unwrap();
    let pixels = vec![128u8; 320 * 240 * 4];
    let mut texture = None;
    unsafe {
        device
            .device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 320,
                    Height: 240,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: pixels.as_ptr().cast(),
                    SysMemPitch: 320 * 4,
                    SysMemSlicePitch: 0,
                }),
                Some(&mut texture),
            )
            .unwrap();
    }
    for frame in 0..60 {
        let sample = converter
            .convert(
                texture.as_ref().unwrap(),
                frame * 10_000_000 / 30,
                10_000_000 / 30,
            )
            .unwrap();
        encoder.video(&sample).unwrap();
    }
    encoder.finish().unwrap();
    assert!(std::fs::metadata(path).unwrap().len() > 1000);
}
