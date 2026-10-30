use serde::{Deserialize, Serialize};

pub const MAX_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: i32,
    pub tool: i32,
    pub rect: [f64; 4],
    pub points: Vec<[f64; 2]>,
    pub text: String,
    pub number: i32,
    pub color: u32,
    pub background_color: u32,
    pub width: f64,
    pub filled: bool,
    pub corner_radius: f64,
    pub arrow_style: i32,
    pub arrow_line_style: i32,
    pub highlighter_style: i32,
    pub rotation_degrees: f64,
    pub magnifier_scale: f64,
    pub magnifier_shape: i32,
    pub number_style: i32,
    pub font_family: String,
    pub font_weight: i32,
    pub text_italic: bool,
    pub rectangle_style: i32,
    pub marker_shape: i32,
    pub mosaic_style: i32,
    pub ruler_mode: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u32,
    pub annotations: Vec<Annotation>,
    pub selected_annotation_id: Option<i32>,
    pub selected_annotation_ids: Vec<i32>,
    pub next_number: i32,
    pub next_annotation_id: i32,
}

impl Default for Snapshot {
    /// 返回空编辑文档；无参数，编号从 1 开始。
    fn default() -> Self {
        Self {
            version: 1,
            annotations: Vec::new(),
            selected_annotation_id: None,
            selected_annotation_ids: Vec::new(),
            next_number: 1,
            next_annotation_id: 1,
        }
    }
}

impl Snapshot {
    /// 估算快照占用的内存字节数，无参数，返回包含向量和字符串的大小。
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.selected_annotation_ids.capacity() * std::mem::size_of::<i32>()
            + self.annotations.capacity() * std::mem::size_of::<Annotation>()
            + self
                .annotations
                .iter()
                .map(|a| {
                    a.points.capacity() * std::mem::size_of::<[f64; 2]>()
                        + a.text.capacity()
                        + a.font_family.capacity()
                })
                .sum::<usize>()
    }

    /// 校验快照版本、尺寸和浮点数据；无参数，返回校验结果，不修改文档。
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1 {
            return Err("unsupported annotation snapshot version");
        }
        if self.bytes() > MAX_SNAPSHOT_BYTES {
            return Err("annotation snapshot exceeds memory limit");
        }
        for annotation in &self.annotations {
            if !annotation
                .rect
                .iter()
                .chain(annotation.points.iter().flatten())
                .copied()
                .chain([
                    annotation.width,
                    annotation.corner_radius,
                    annotation.rotation_degrees,
                    annotation.magnifier_scale,
                ])
                .all(f64::is_finite)
            {
                return Err("annotation contains non-finite coordinates");
            }
        }
        Ok(())
    }

    /// 从 bytes 解码并校验快照，返回完整标注文档或错误，不接受无限大的输入。
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err("annotation snapshot exceeds input limit".into());
        }
        let snapshot: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        snapshot.validate()?;
        Ok(snapshot)
    }
}
