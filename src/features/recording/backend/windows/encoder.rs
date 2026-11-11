//! 【Windows录制】【媒体封装】系统 H.264/AAC 编码与 MP4 输出，实际查询编码转换器属性

use super::{device::Device, media_type};
use crate::config::recording::Recording;
use std::path::Path;
use windows::{
    Win32::Media::MediaFoundation::*,
    core::{Interface, PCWSTR, Result},
};

pub(super) struct Encoder {
    writer: IMFSinkWriter,
    video: u32,
    audio: Option<u32>,
    pub hardware: bool,
}

impl Encoder {
    /// 【Windows录制】【编码器建立】按设备、路径、尺寸与配置建立编码，require_hardware 控制硬件必要性
    pub fn new(
        device: Option<&Device>,
        path: &Path,
        size: (u32, u32),
        config: &Recording,
        hardware: bool,
    ) -> Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        let filename: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let attributes = media_type::attributes(5)?;
            attributes.SetUINT32(
                &MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,
                u32::from(hardware),
            )?;
            attributes.SetUINT32(&MF_LOW_LATENCY, 1)?;
            if let Some(device) = device {
                attributes.SetUnknown(&MF_SINK_WRITER_D3D_MANAGER, &device.manager)?;
            }
            let writer = MFCreateSinkWriterFromURL(PCWSTR(filename.as_ptr()), None, &attributes)?;
            let output = media_type::video(&MFVideoFormat_H264, size, config.frame_rate())?;
            let bits_per_pixel = match config.quality.as_str() {
                "high" => 0.24,
                "efficient" => 0.08,
                _ => 0.14,
            };
            let bitrate = (f64::from(size.0)
                * f64::from(size.1)
                * f64::from(config.frame_rate())
                * bits_per_pixel)
                .clamp(1_000_000.0, 80_000_000.0) as u32;
            output.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
            output.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Main.0 as u32)?;
            let video = writer.AddStream(&output)?;
            let input = media_type::video(
                if device.is_some() {
                    &MFVideoFormat_NV12
                } else {
                    &MFVideoFormat_RGB32
                },
                size,
                config.frame_rate(),
            )?;
            if device.is_none() {
                // 1. 【Windows录制】【像素方向】GDI 提供自上而下 BGRA；显式正步长避免系统按 RGB 默认方向翻转
                input.SetUINT32(&MF_MT_DEFAULT_STRIDE, size.0 * 4)?;
                input.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_0_255.0 as u32)?;
            }
            writer.SetInputMediaType(video, &input, None)?;
            let audio = if config.audio != "off" && config.format != "gif" {
                let stream = writer.AddStream(&media_type::audio(true)?)?;
                writer.SetInputMediaType(stream, &media_type::audio(false)?, None)?;
                Some(stream)
            } else {
                None
            };
            writer.BeginWriting()?;
            let hardware = Self::hardware_encoder(&writer, video);
            Ok(Self {
                writer,
                video,
                audio,
                hardware,
            })
        }
    }

    /// 【Windows录制】【硬件核查】按 writer 和 stream 检查实际视频编码转换器的硬件属性，未知时返回 false
    fn hardware_encoder(writer: &IMFSinkWriter, stream: u32) -> bool {
        let Ok(writer) = writer.cast::<IMFSinkWriterEx>() else {
            return false;
        };
        unsafe {
            for index in 0..16 {
                let mut transform = None;
                if writer
                    .GetTransformForStream(stream, index, None, &mut transform)
                    .is_err()
                {
                    break;
                }
                if let Some(transform) = transform
                    && let Ok(attributes) = transform.GetAttributes()
                    && attributes
                        .GetStringLength(&MFT_ENUM_HARDWARE_URL_Attribute)
                        .is_ok()
                {
                    return true;
                }
            }
        }
        false
    }

    /// 【Windows录制】【视频写入】按 sample 提交 GPU NV12 样本，媒体框架持有表面租约直到消费完成
    pub fn video(&self, sample: &IMFSample) -> Result<()> {
        unsafe { self.writer.WriteSample(self.video, sample) }
    }

    /// 【Windows录制】【音频写入】按 sample 提交已对齐时间轴的 PCM，未启用音轨时不写入
    pub fn audio(&self, sample: &IMFSample) -> Result<()> {
        if let Some(stream) = self.audio {
            unsafe {
                self.writer.WriteSample(stream, sample)?;
            }
        }
        Ok(())
    }

    /// 【Windows录制】【文件收尾】无参数，排空编码与写出 MP4 索引，错误必须传给保存状态
    pub fn finish(&self) -> Result<()> {
        unsafe { self.writer.Finalize() }
    }
}
