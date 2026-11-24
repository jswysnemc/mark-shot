//! 【PipeWire】【缓冲协商】显式声明共享内存缓冲的数量、步长与类型，避免服务端等待缓冲协商导致首帧超时

use pipewire::spa;

/// 【PipeWire】【共享缓冲参数】按 width、height 序列化 SPA_PARAM_Buffers，允许 MemFd 与 MemPtr 共享内存映射
pub(super) fn shared(width: u32, height: u32) -> Result<Vec<u8>, String> {
    use spa::{
        pod::{ChoiceValue, Object, Property, PropertyFlags, Value},
        sys::*,
        utils::{Choice, ChoiceEnum, ChoiceFlags},
    };
    let stride = width
        .checked_mul(4)
        .ok_or("PipeWire frame stride overflow")?;
    let length = stride
        .checked_mul(height)
        .ok_or("PipeWire frame size overflow")?;
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > shot_image::MAX_PIXELS as u64
    {
        return Err("PipeWire frame dimensions are empty or exceed the pixel limit".into());
    }
    let stride = i32::try_from(stride).map_err(|_| "PipeWire frame stride exceeds i32")?;
    let length = i32::try_from(length).map_err(|_| "PipeWire frame size exceeds i32")?;
    // 1. 【PipeWire】【缓冲类型】同时允许 MemFd 与 MemPtr，兼顾 Mutter、KWin 与 wlroots 门户实现
    let data_types = (1 << SPA_DATA_MemFd) | (1 << SPA_DATA_MemPtr);
    let mut bytes = Vec::new();
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(&mut bytes),
        &Value::Object(Object {
            type_: SPA_TYPE_OBJECT_ParamBuffers,
            id: SPA_PARAM_Buffers,
            properties: vec![
                Property {
                    key: SPA_PARAM_BUFFERS_buffers,
                    flags: PropertyFlags::empty(),
                    value: Value::Choice(ChoiceValue::Int(Choice(
                        ChoiceFlags::empty(),
                        ChoiceEnum::Range {
                            default: 4,
                            min: 1,
                            max: 8,
                        },
                    ))),
                },
                Property {
                    key: SPA_PARAM_BUFFERS_blocks,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(1),
                },
                Property {
                    key: SPA_PARAM_BUFFERS_size,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(length),
                },
                Property {
                    key: SPA_PARAM_BUFFERS_stride,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(stride),
                },
                Property {
                    key: SPA_PARAM_BUFFERS_dataType,
                    flags: PropertyFlags::empty(),
                    value: Value::Int(data_types),
                },
            ],
        }),
    )
    .map_err(|error| format!("PipeWire buffer parameters: {error}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【PipeWire测试】【缓冲参数】验证常规分辨率生成有效 SPA Pod，零尺寸和超限尺寸返回错误，无参数
    #[test]
    fn shared_buffer_params_serialize_for_valid_sizes_and_reject_invalid() {
        let pod_bytes = shared(1920, 1080).unwrap();
        assert!(spa::pod::Pod::from_bytes(&pod_bytes).is_some());
        assert!(shared(0, 1080).is_err());
        assert!(shared(1920, 0).is_err());
        assert!(shared(u32::MAX, 2).is_err());
    }
}
