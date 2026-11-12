//! 【前景缓存】【变化写入】所有平台只写入变化的字节块，透明前景不触碰相同的零页

/// 【前景缓存】【字节更新】将 source 中变化的块复制到同长度 target，返回值为空，不修改未变化的字节或分配新缓冲
pub(crate) fn copy_changed(target: &mut [u8], source: &[u8]) {
    assert_eq!(target.len(), source.len());
    // 1. 【前景缓存】【零页保留】只读比较不会将共享零页转为自有页，已变化的非零前景仍完整复制
    for (target, source) in target.chunks_mut(256).zip(source.chunks(256)) {
        if target != source {
            target.copy_from_slice(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【前景缓存测试】【字节一致】无参数，验证非对齐片段、重复更新及旧非零内容清除与普通复制一致
    #[test]
    fn changed_blocks_keep_all_pixels_and_clear_previous_content() {
        let mut target = vec![71; 1031];
        let mut source = vec![0; target.len()];
        for offset in [0, 255, 256, 1027, 1030] {
            source[offset] = 139;
        }
        for _ in 0..3 {
            copy_changed(&mut target[3..1029], &source[3..1029]);
            assert_eq!(&target[3..1029], &source[3..1029]);
            assert_eq!(&target[..3], &[71; 3]);
            assert_eq!(&target[1029..], &[71; 2]);
        }
        source.fill(0);
        copy_changed(&mut target, &source);
        assert_eq!(target, source);
    }
}
