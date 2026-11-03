//! 【滤镜】【有界缓存】仅保留冻结原图区域，删除和取消立即释放，不创建工作区级模糊副本

use std::collections::{HashMap, HashSet, VecDeque};

use crate::editor::EditorState;
use crate::features::filter::{
    RegionFilter,
    raster::{FilterKey, MAX_CACHED_BYTES, Raster},
};
use crate::renderer::FrozenImage;
use crate::types::{Annotation, AnnotationShape};

pub const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub struct FilterCache {
    pub images: HashMap<u64, Raster>,
    order: VecDeque<u64>,
    bytes: usize,
}

impl FilterCache {
    /// 【滤镜】【缓存准备】按 id、filter、image 生成或复用原图区域；返回是否新增缓存，过大区域交给按需分块绘制
    pub fn prepare(&mut self, id: u64, filter: RegionFilter, image: FrozenImage<'_>) -> bool {
        let Some(key) = FilterKey::new(image, filter) else {
            self.remove(id);
            return false;
        };
        if self.images.get(&id).is_some_and(|raster| raster.key == key) {
            self.touch(id);
            return false;
        }
        self.remove(id);
        let Some(bytes) = key.bytes().filter(|bytes| *bytes <= MAX_CACHED_BYTES) else {
            return false;
        };
        // 1. 【滤镜】【缓存准备】生成新缓存前释放最早使用区域，存量不超过十六 MiB
        while self.bytes + bytes > MAX_CACHE_BYTES {
            let Some(oldest) = self.order.front().copied() else {
                break;
            };
            self.remove(oldest);
        }
        let Some(raster) = Raster::new(image, key) else {
            return false;
        };
        self.bytes += raster.pixels.data().len();
        self.images.insert(id, raster);
        self.order.push_back(id);
        true
    }

    /// 【滤镜】【缓存删除】移除 id 对应像素与使用记录；无返回值，不影响其他标注
    pub fn remove(&mut self, id: u64) {
        if let Some(raster) = self.images.remove(&id) {
            self.bytes -= raster.pixels.data().len();
        }
        self.order.retain(|other| *other != id);
    }

    /// 【滤镜】【存活区域】只保留 annotations 和 pending 中的滤镜标识；无返回值，拖动中仍保留移出图层的区域
    pub fn retain_live(&mut self, annotations: &[Annotation], pending: Option<&Annotation>) {
        let live: HashSet<_> = annotations
            .iter()
            .chain(pending)
            .filter(|annotation| matches!(annotation.shape, AnnotationShape::Redact { .. }))
            .map(|annotation| annotation.id)
            .collect();
        self.images.retain(|id, _| live.contains(id));
        self.order.retain(|id| self.images.contains_key(id));
        self.bytes = self
            .images
            .values()
            .map(|raster| raster.pixels.data().len())
            .sum();
    }

    /// 【滤镜】【缓存占用】返回当前保留的像素字节数，不包含临时分块缓冲
    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }

    /// 【滤镜】【使用记录】把 id 移到最新使用位置；无返回值，不分配像素
    fn touch(&mut self, id: u64) {
        self.order.retain(|other| *other != id);
        self.order.push_back(id);
    }
}

impl EditorState {
    /// 【滤镜】【帧缓存同步】按持久标注和草稿同步当前原图区域；无返回值，其他标注修改不会改变滤镜输入
    pub fn refresh_filters(&mut self) {
        self.filters
            .retain_live(&self.annotations, self.pending.as_ref());
        let image =
            FrozenImage::from_parts(self.file_canvas.as_ref(), &self.captures, &self.placements);
        for annotation in self.annotations.iter().chain(self.pending.as_ref()) {
            if let AnnotationShape::Redact { filter } = annotation.shape {
                self.filters.prepare(annotation.id, filter, image);
            }
        }
    }

    /// 【滤镜】【提交缓存】为 annotation 准备原图区域，其他标注不处理；无返回值
    pub fn cache_filter(&mut self, annotation: &Annotation) {
        if let AnnotationShape::Redact { filter } = annotation.shape {
            self.filters.prepare(
                annotation.id,
                filter,
                FrozenImage::from_parts(
                    self.file_canvas.as_ref(),
                    &self.captures,
                    &self.placements,
                ),
            );
        }
    }
}
