//! 【截图另存为】【图像快照】冻结最终 PNG，不把原图像素转换为预览尺寸

use std::{io, sync::Arc};

#[derive(Clone)]
pub struct Snapshot {
    pub png: Arc<Vec<u8>>,
    pub scale: f32,
    pub search: String,
    pub(crate) record_history: bool,
}

impl Snapshot {
    /// 【截图另存为】【快照创建】校验并冻结 png、scale 和 search；返回共享快照，拒绝无效图像及比例
    pub fn new(png: Vec<u8>, scale: f32, search: String) -> io::Result<Self> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(io::Error::other("invalid screenshot scale"));
        }
        crate::features::capture_history::png_metadata::inspect(&mut io::Cursor::new(&png))?;
        crate::features::capture_history::png_metadata::verify(io::Cursor::new(&png))?;
        Ok(Self {
            png: Arc::new(png),
            scale,
            search,
            record_history: true,
        })
    }

    /// 【截图另存为】【成功记录】按最新历史策略记录已经保存的快照；取消或保存失败时不得调用
    pub(super) fn remember(&self) {
        if self.record_history {
            crate::features::capture_history::remember(&self.png, self.scale, &self.search);
        }
    }
}
