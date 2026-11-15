//! 【标注渲染】【稀疏合成】按预乘像素合成有色片段，大范围透明孔洞整段跳过

/// 【标注渲染】【片段合成】将 source 预乘 RGBA 合成到等长 target，无返回值；全透明段不读取或写入目标
pub(super) fn blend(source: &[u8], target: &mut [u8]) {
    let (source_blocks, source_tail) = source.as_chunks::<64>();
    let (target_blocks, target_tail) = target.as_chunks_mut::<64>();
    // 1. 【标注渲染】【透明段跳过】一次检查十六个像素，透明源不写入，透明目标直接复制预乘源
    for (source, target) in source_blocks.iter().zip(target_blocks) {
        if *source != [0; 64] {
            if *target == [0; 64] {
                target.copy_from_slice(source);
            } else {
                pixels(source, target);
            }
        }
    }
    // 2. 【标注渲染】【边界保留】不足十六个像素的尾段沿用相同舍入与合成顺序
    pixels(source_tail, target_tail);
}

/// 【标注渲染】【非零像素】按 source 与 target 执行原有预乘 SourceOver 合成，零透明度保留目标，无返回值
fn pixels(source: &[u8], target: &mut [u8]) {
    for (source, target) in source
        .as_chunks::<4>()
        .0
        .iter()
        .zip(target.as_chunks_mut::<4>().0)
    {
        if source[3] == 0 {
            continue;
        }
        let inverse = 255 - u32::from(source[3]);
        for channel in 0..4 {
            target[channel] = (u32::from(source[channel])
                + (u32::from(target[channel]) * inverse + 127) / 255)
                .min(255) as u8;
        }
    }
}
