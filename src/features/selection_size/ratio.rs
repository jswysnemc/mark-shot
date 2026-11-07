//! 【选区尺寸】【宽高联动】比例只属于截图会话，整数尺寸遵循原版取整与上限反推规则

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Width,
    Height,
}

pub const PRESETS: [(&str, f64); 6] = [
    ("Free", 0.0),
    ("1:1", 1.0),
    ("4:3", 4.0 / 3.0),
    ("16:9", 16.0 / 9.0),
    ("9:16", 9.0 / 16.0),
    ("21:9", 21.0 / 9.0),
];

/// 【选区尺寸】【预设识别】按 ratio 返回六种预设下标，自由比例返回零，自定义比例返回空值
pub fn preset(ratio: f64) -> Option<usize> {
    if ratio <= 0.0 {
        return Some(0);
    }
    PRESETS
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, (_, value))| ((value - ratio).abs() < 1e-6).then_some(index))
}

/// 【选区尺寸】【另一侧尺寸】按 value、ratio 和 from_width 返回联动整数尺寸，非法比例返回零
pub fn related(value: i32, ratio: f64, from_width: bool) -> i32 {
    if !ratio.is_finite() || ratio <= 0.0 {
        return 0;
    }
    let result = if from_width {
        f64::from(value) / ratio
    } else {
        f64::from(value) * ratio
    };
    (result.round() as i32).max(1)
}

/// 【选区尺寸】【尺寸联动】按 edited 一侧和 ratio 调整 requested，maximum 为图像上限，返回范围内整数宽高
pub fn linked(requested: (i32, i32), edited: Field, ratio: f64, maximum: (i32, i32)) -> (i32, i32) {
    // 1. 【选区尺寸】【输入限制】先限制被输入的宽高，自由比例直接返回
    let maximum = (maximum.0.max(1), maximum.1.max(1));
    let mut width = requested.0.clamp(1, maximum.0);
    let mut height = requested.1.clamp(1, maximum.1);
    if !ratio.is_finite() || ratio <= 0.0 {
        return (width, height);
    }
    // 2. 【选区尺寸】【比例计算】按实际修改的一侧计算另一侧
    match edited {
        Field::Width => height = related(width, ratio, true),
        Field::Height => width = related(height, ratio, false),
    }
    // 3. 【选区尺寸】【上限反推】超出图像时从受限一侧反推，避免截断后失去联动关系
    if height > maximum.1 {
        height = maximum.1;
        width = related(height, ratio, false);
    }
    if width > maximum.0 {
        width = maximum.0;
        height = related(width, ratio, true);
    }
    (width.clamp(1, maximum.0), height.clamp(1, maximum.1))
}
