use crate::types::Output;

/// 【Wayland】【显示器名称】取得合成器提供的显示器名称；缺失时使用稳定序号
pub fn name(output: &Output, index: usize) -> String {
    output
        .info
        .name
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("output-{}", index + 1))
}

/// 【Wayland】【显示器选择】根据名称、全桌面选项或活动序号返回捕获索引；未知名称返回错误
pub fn indices(
    names: &[String],
    requested: &[String],
    all: bool,
    active: Option<usize>,
) -> Result<Vec<usize>, String> {
    if names.is_empty() {
        return Err("the compositor reported no displays".into());
    }
    if !requested.is_empty() {
        return requested
            .iter()
            .map(|name| {
                names
                    .iter()
                    .position(|candidate| candidate == name)
                    .ok_or_else(|| {
                        format!("unknown display {name:?}; available: {}", names.join(", "))
                    })
            })
            .collect();
    }
    if all {
        return Ok((0..names.len()).collect());
    }
    Ok(vec![
        active.filter(|index| *index < names.len()).unwrap_or(0),
    ])
}
