//! 【原生音频】【兼容来源】动态加载系统 PulseAudio 客户端，所有调用非阻塞，不启动音频服务
use crate::Media;
use std::{
    ffi::c_void,
    ptr::NonNull,
    time::{Duration, Instant},
};

pub(crate) struct Api {
    open: unsafe extern "C" fn(i32) -> *mut c_void,
    destroy: unsafe extern "C" fn(*mut c_void),
    read: unsafe extern "C" fn(*mut c_void, *mut f32, i32, *mut u64) -> i32,
    pause: unsafe extern "C" fn(*mut c_void, i32) -> i32,
}
impl Api {
    /// 【原生音频】【接口加载】按 library 获取私有音频入口，返回完整函数表或缺符号错误
    pub fn load(library: &libloading::Library) -> Result<Self, String> {
        unsafe {
            Ok(Self {
                open: *library.get(b"ms_pulse_open\0").map_err(|e| e.to_string())?,
                destroy: *library
                    .get(b"ms_pulse_destroy\0")
                    .map_err(|e| e.to_string())?,
                read: *library.get(b"ms_pulse_read\0").map_err(|e| e.to_string())?,
                pause: *library
                    .get(b"ms_pulse_pause\0")
                    .map_err(|e| e.to_string())?,
            })
        }
    }
}
pub struct Pulse {
    media: Media,
    handle: NonNull<c_void>,
    samples: Vec<f32>,
}
impl Pulse {
    /// 【原生音频】【会话创建】按 media 和 microphone 建立异步音频连接，返回独占句柄
    pub fn new(media: &Media, microphone: bool) -> Result<Self, String> {
        let handle = NonNull::new(unsafe { (media.0.pulse.open)(i32::from(microphone)) })
            .ok_or("PulseAudio unavailable")?;
        Ok(Self {
            media: media.clone(),
            handle,
            samples: vec![0.0; 48000],
        })
    }
    /// 【原生音频】【样本读取】无参数，推进服务器事件并返回借用的双声道样本及首样本时刻，空值表示尚无数据
    pub fn read(&mut self) -> Result<Option<(Instant, &[f32])>, String> {
        let mut latency = 0;
        let count = unsafe {
            (self.media.0.pulse.read)(
                self.handle.as_ptr(),
                self.samples.as_mut_ptr(),
                (self.samples.len() / 2) as i32,
                &mut latency,
            )
        };
        if count < 0 || latency > 10_000_000 {
            return Err("PulseAudio capture failed".into());
        }
        if count == 0 {
            return Ok(None);
        }
        let at = Instant::now()
            .checked_sub(Duration::from_micros(latency))
            .ok_or("PulseAudio timestamp overflow")?;
        Ok(Some((at, &self.samples[..count as usize * 2])))
    }
    /// 【原生音频】【暂停请求】按 paused 切换服务器队列，返回控制失败
    pub fn pause(&mut self, paused: bool) -> Result<(), String> {
        if unsafe { (self.media.0.pulse.pause)(self.handle.as_ptr(), i32::from(paused)) } < 0 {
            return Err("PulseAudio pause failed".into());
        }
        Ok(())
    }
}
impl Drop for Pulse {
    /// 【原生音频】【连接销毁】无参数，销毁句柄并保留媒体库直至所有原生资源释放
    fn drop(&mut self) {
        unsafe { (self.media.0.pulse.destroy)(self.handle.as_ptr()) };
    }
}
