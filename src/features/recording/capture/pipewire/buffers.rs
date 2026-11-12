//! 【录制流】【缓冲协商】显式提供共享内存的数量和布局，不能只协商颜色格式后等待服务端猜测
use pipewire::spa;

/// 【录制流】【共享约束】按 size 返回四个 MemFd 缓冲的序列化参数，拒绝超过内存预算的尺寸
pub(super) fn shared(size: (u32, u32)) -> Result<Vec<u8>, String> {
    use spa::{
        pod::{Object, Property, PropertyFlags, Value},
        sys::*,
    };
    let stride = size.0.checked_mul(4).ok_or("PipeWire stride overflow")?;
    let length = stride
        .checked_mul(size.1)
        .ok_or("PipeWire buffer overflow")?;
    if size.0 == 0 || size.1 == 0 || length > 128 * 1024 * 1024 {
        return Err("PipeWire frame exceeds budget".into());
    }
    let property = |key, value| Property {
        key,
        flags: PropertyFlags::empty(),
        value: Value::Int(value),
    };
    let mut bytes = Vec::new();
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(&mut bytes),
        &Value::Object(Object {
            type_: SPA_TYPE_OBJECT_ParamBuffers,
            id: SPA_PARAM_Buffers,
            properties: vec![
                property(SPA_PARAM_BUFFERS_buffers, 4),
                property(SPA_PARAM_BUFFERS_blocks, 1),
                property(SPA_PARAM_BUFFERS_size, length as i32),
                property(SPA_PARAM_BUFFERS_stride, stride as i32),
                property(SPA_PARAM_BUFFERS_dataType, 1 << SPA_DATA_MemFd),
            ],
        }),
    )
    .map_err(|e| format!("PipeWire buffer parameters: {e}"))?;
    Ok(bytes)
}
