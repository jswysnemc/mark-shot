//! 【Windows录制】【媒体描述】集中声明 H.264、NV12、AAC 与 PCM 格式

use windows::{
    Win32::Media::MediaFoundation::*,
    core::{GUID, Result},
};

/// 【Windows录制】【视频媒体类型】按 subtype、size、fps 构建逐行 SDR 视频描述，返回媒体对象
pub(super) fn video(subtype: &GUID, size: (u32, u32), fps: u32) -> Result<IMFMediaType> {
    unsafe {
        let media = MFCreateMediaType()?;
        media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        media.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        media.SetUINT64(
            &MF_MT_FRAME_SIZE,
            (u64::from(size.0) << 32) | u64::from(size.1),
        )?;
        media.SetUINT64(&MF_MT_FRAME_RATE, (u64::from(fps) << 32) | 1)?;
        media.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        media.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        media.SetUINT32(&MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32)?;
        media.SetUINT32(&MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32)?;
        media.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)?;
        Ok(media)
    }
}

/// 【Windows录制】【音频媒体类型】按 encoded 选择 AAC 或 48kHz 双声道浮点 PCM，返回媒体对象
pub(super) fn audio(encoded: bool) -> Result<IMFMediaType> {
    unsafe {
        let media = MFCreateMediaType()?;
        media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        media.SetGUID(
            &MF_MT_SUBTYPE,
            if encoded {
                &MFAudioFormat_AAC
            } else {
                &MFAudioFormat_Float
            },
        )?;
        media.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
        media.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, 48000)?;
        media.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, if encoded { 16 } else { 32 })?;
        media.SetUINT32(
            &MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
            if encoded { 24000 } else { 384000 },
        )?;
        if !encoded {
            media.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 8)?;
        }
        Ok(media)
    }
}

/// 【Windows录制】【属性分配】按 capacity 创建媒体属性容器，返回 COM 对象
pub(super) fn attributes(capacity: u32) -> Result<IMFAttributes> {
    unsafe {
        let mut attributes = None;
        MFCreateAttributes(&mut attributes, capacity)?;
        Ok(attributes.unwrap())
    }
}
