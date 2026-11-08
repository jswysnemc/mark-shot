//! 【滤镜】【区域模型】保存原图强度和未旋转矩形，模式编号与原版保持一致

use serde::{Deserialize, Serialize};
use tiny_skia::Rect;

pub type Point = (f32, f32);
pub const STRENGTH: (f32, f32) = (4.0, 48.0);
pub const DEFAULT_STRENGTH: f32 = 14.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum FilterMode {
    #[default]
    Pixelate = 0,
    Blur = 1,
    Grayscale = 2,
    Invert = 3,
    Brighten = 4,
}

impl FilterMode {
    pub const ALL: [Self; 5] = [
        Self::Pixelate,
        Self::Blur,
        Self::Grayscale,
        Self::Invert,
        Self::Brighten,
    ];

    /// 【滤镜】【模式标识】返回稳定配置名称，不以本地化文字保存模式
    pub fn key(self) -> &'static str {
        match self {
            Self::Pixelate => "pixelate",
            Self::Blur => "blur",
            Self::Grayscale => "grayscale",
            Self::Invert => "invert",
            Self::Brighten => "brighten",
        }
    }

    /// 【滤镜】【模式说明】返回紧凑属性弹层使用的说明文字
    pub fn label(self) -> &'static str {
        match self {
            Self::Pixelate => "Pixelate",
            Self::Blur => "Blur",
            Self::Grayscale => "Grayscale",
            Self::Invert => "Invert",
            Self::Brighten => "Brighten",
        }
    }

    /// 【滤镜】【模式解析】将 value 转为模式，接受原版编号和稳定名称；未知值返回空值
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase().replace('-', "_");
        Self::ALL
            .into_iter()
            .find(|mode| mode.key() == value)
            .or_else(|| {
                value
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| Self::ALL.get(index).copied())
            })
    }

    /// 【滤镜】【旧模式转换】把旧 pixelate 开关转换为模式；返回像素化或模糊
    pub fn from_pixelate(pixelate: bool) -> Self {
        if pixelate { Self::Pixelate } else { Self::Blur }
    }
}

impl<'de> Deserialize<'de> for FilterMode {
    /// 【滤镜】【模式配置】从 deserializer 读取名称或原版整数编号；返回模式，未知值报告字段错误
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Name(String),
            Index(u8),
        }
        let mode = match Value::deserialize(deserializer)? {
            Value::Name(value) => Self::parse(&value),
            Value::Index(index) => Self::ALL.get(index as usize).copied(),
        };
        mode.ok_or_else(|| serde::de::Error::custom("unknown filter mode"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionFilter {
    pub radius: f32,
    pub start: Point,
    pub end: Point,
    pub strength: f32,
    pub mode: FilterMode,
    /// 【滤镜】【旋转轴心】以矩形中心为轴保存弧度，原图采样使用未旋转矩形
    pub rotation: f32,
    /// 【滤镜】【编辑尺寸】只缩放控制点和最小尺寸，不改变原图强度
    pub style_scale: f32,
}

impl RegionFilter {
    /// 【滤镜】【区域创建】以 start、end、strength、mode 创建未旋转区域；非有限坐标返回空值
    pub fn new(start: Point, end: Point, strength: f32, mode: FilterMode) -> Option<Self> {
        if ![start.0, start.1, end.0, end.1]
            .into_iter()
            .all(f32::is_finite)
        {
            return None;
        }
        Some(Self {
            radius: 0.0,
            start,
            end,
            strength: clamped_strength(strength),
            mode,
            rotation: 0.0,
            style_scale: 1.0,
        })
    }

    /// 【滤镜】【区域校验】返回坐标、强度、旋转和控制点比例是否有效，不要求创建草稿已经具备面积
    pub fn valid(self) -> bool {
        [
            self.start.0,
            self.start.1,
            self.end.0,
            self.end.1,
            self.strength,
            self.rotation,
            self.style_scale,
        ]
        .into_iter()
        .all(f32::is_finite)
            && (STRENGTH.0..=STRENGTH.1).contains(&self.strength)
            && self.style_scale > 0.0
    }

    /// 【滤镜】【矩形读取】返回规范化未旋转矩形；无效数据或零面积返回空值
    pub fn rect(self) -> Option<Rect> {
        self.valid()
            .then(|| {
                Rect::from_ltrb(
                    self.start.0.min(self.end.0),
                    self.start.1.min(self.end.1),
                    self.start.0.max(self.end.0),
                    self.start.1.max(self.end.1),
                )
            })
            .flatten()
            .filter(|rect| !rect.is_empty())
    }

    /// 【滤镜】【中心读取】返回未旋转矩形的中心，同时作为图像和控制框旋转轴心
    pub fn center(self) -> Point {
        (
            self.start.0 + (self.end.0 - self.start.0) / 2.0,
            self.start.1 + (self.end.1 - self.start.1) / 2.0,
        )
    }

    /// 【滤镜】【导出映射】按 scale 缩放坐标和编辑尺寸；返回副本，强度仍为原图像素或提亮数值
    pub fn scaled(self, scale: f32) -> Self {
        Self {
            start: (self.start.0 * scale, self.start.1 * scale),
            end: (self.end.0 * scale, self.end.1 * scale),
            style_scale: self.style_scale * scale,
            ..self
        }
    }

    /// 【滤镜】【位置映射】按 delta 平移未旋转矩形和轴心；返回副本，不改变模式、强度和旋转
    pub fn translated(self, delta: Point) -> Self {
        Self {
            start: (self.start.0 + delta.0, self.start.1 + delta.1),
            end: (self.end.0 + delta.0, self.end.1 + delta.1),
            ..self
        }
    }
}

/// 【滤镜】【强度约束】将 strength 限定为四至四十八；非有限值使用十四，返回有效原图强度
pub fn clamped_strength(strength: f32) -> f32 {
    if strength.is_finite() {
        strength.clamp(STRENGTH.0, STRENGTH.1)
    } else {
        DEFAULT_STRENGTH
    }
}
