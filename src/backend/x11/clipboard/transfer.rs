//! 【X11剪贴板】【请求载荷】根据服务器请求上限选择直接交付或增量传输，保留属性头与对齐空间

/// 【X11剪贴板】【载荷上限】按 request_bytes 扣除 ChangeProperty 与扩展长度头，返回四字节对齐载荷或无有效载荷错误
pub(super) fn chunk_size(request_bytes: usize) -> crate::AppResult<usize> {
    let payload = request_bytes.saturating_sub(28).min(super::LIMIT) & !3;
    if payload == 0 {
        return Err("X11 clipboard request limit cannot hold property data".into());
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【X11剪贴板测试】【协议边界】无参数，确认标准及扩展请求均扣除头部、按四字节对齐，并拒绝无法容纳数据的请求
    #[test]
    fn payload_reserves_headers_and_preserves_protocol_alignment() {
        assert_eq!(chunk_size(262_140).unwrap(), 262_112);
        assert_eq!(chunk_size(16_777_212).unwrap(), 16_777_184);
        assert_eq!(chunk_size(35).unwrap(), 4);
        assert_eq!(chunk_size(usize::MAX).unwrap(), super::super::LIMIT);
        for size in 0..32 {
            assert!(chunk_size(size).is_err());
        }
    }
}
