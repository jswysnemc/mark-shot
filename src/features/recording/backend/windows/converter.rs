//! 【Windows录制】【GPU颜色转换】D3D11 视频处理器裁剪 BGRA 并写入编码器持有的 NV12 surface

use super::{device::Device, media_type};
use windows::{
    Win32::{
        Foundation::RECT,
        Graphics::{Direct3D11::*, Dxgi::Common::*},
        Media::MediaFoundation::*,
    },
    core::{Interface, Result},
};

pub(super) struct Converter {
    allocator: IMFVideoSampleAllocatorEx,
    video: ID3D11VideoDevice,
    context: ID3D11VideoContext,
    enumerator: ID3D11VideoProcessorEnumerator,
    processor: ID3D11VideoProcessor,
}

impl Converter {
    /// 【Windows录制】【转换器创建】按 device、input、crop、fps 建立有界四张表面池和 GPU 裁剪，返回设备错误
    pub fn new(device: &Device, input: (u32, u32), crop: [u32; 4], fps: u32) -> Result<Self> {
        unsafe {
            let video: ID3D11VideoDevice = device.device.cast()?;
            let context: ID3D11VideoContext = device.context.cast()?;
            let [x, y, width, height] = crop;
            let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
                InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
                InputFrameRate: DXGI_RATIONAL {
                    Numerator: fps,
                    Denominator: 1,
                },
                InputWidth: input.0,
                InputHeight: input.1,
                OutputFrameRate: DXGI_RATIONAL {
                    Numerator: fps,
                    Denominator: 1,
                },
                OutputWidth: width,
                OutputHeight: height,
                Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
            };
            let enumerator = video.CreateVideoProcessorEnumerator(&desc)?;
            let processor = video.CreateVideoProcessor(&enumerator, 0)?;
            context.VideoProcessorSetStreamFrameFormat(
                &processor,
                0,
                D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            );
            context.VideoProcessorSetStreamAutoProcessingMode(&processor, 0, false);
            context.VideoProcessorSetStreamSourceRect(
                &processor,
                0,
                true,
                Some(&RECT {
                    left: x as i32,
                    top: y as i32,
                    right: (x + width) as i32,
                    bottom: (y + height) as i32,
                }),
            );
            context.VideoProcessorSetStreamDestRect(
                &processor,
                0,
                true,
                Some(&RECT {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                }),
            );
            let context1: ID3D11VideoContext1 = context.cast()?;
            context1.VideoProcessorSetStreamColorSpace1(
                &processor,
                0,
                DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
            );
            context1.VideoProcessorSetOutputColorSpace1(
                &processor,
                DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
            );
            let mut raw = std::ptr::null_mut();
            MFCreateVideoSampleAllocatorEx(&IMFVideoSampleAllocatorEx::IID, &mut raw)?;
            let allocator = IMFVideoSampleAllocatorEx::from_raw(raw);
            allocator.SetDirectXManager(&device.manager)?;
            let attributes = media_type::attributes(2)?;
            attributes.SetUINT32(&MF_SA_D3D11_BINDFLAGS, D3D11_BIND_RENDER_TARGET.0 as u32)?;
            allocator.InitializeSampleAllocatorEx(
                2,
                4,
                &attributes,
                &media_type::video(&MFVideoFormat_NV12, (width, height), fps)?,
            )?;
            Ok(Self {
                allocator,
                video,
                context,
                enumerator,
                processor,
            })
        }
    }

    /// 【Windows录制】【纹理提交】按 texture、pts、duration 生成拥有独立池租约的 NV12 样本，池耗尽时返回媒体错误
    pub fn convert(&self, texture: &ID3D11Texture2D, pts: i64, duration: i64) -> Result<IMFSample> {
        unsafe {
            let sample = self.allocator.AllocateSample()?;
            let buffer: IMFDXGIBuffer = sample.GetBufferByIndex(0)?.cast()?;
            let mut raw = std::ptr::null_mut();
            buffer.GetResource(&ID3D11Texture2D::IID, &mut raw)?;
            let destination = ID3D11Texture2D::from_raw(raw);
            let mut input = None;
            self.video.CreateVideoProcessorInputView(
                texture,
                &self.enumerator,
                &D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
                    FourCC: 0,
                    ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_VPIV {
                            MipSlice: 0,
                            ArraySlice: 0,
                        },
                    },
                },
                Some(&mut input),
            )?;
            let mut output = None;
            self.video.CreateVideoProcessorOutputView(
                &destination,
                &self.enumerator,
                &D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
                    ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 },
                    },
                },
                Some(&mut output),
            )?;
            let mut stream = D3D11_VIDEO_PROCESSOR_STREAM {
                Enable: true.into(),
                pInputSurface: std::mem::ManuallyDrop::new(input),
                ..Default::default()
            };
            let result = self.context.VideoProcessorBlt(
                &self.processor,
                output.as_ref().unwrap(),
                0,
                std::slice::from_ref(&stream),
            );
            std::mem::ManuallyDrop::drop(&mut stream.pInputSurface);
            result?;
            sample.SetSampleTime(pts)?;
            sample.SetSampleDuration(duration)?;
            Ok(sample)
        }
    }
}
