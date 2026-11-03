//! 【截图历史】【浏览模型】搜索、稳定选择与有字节上限的预览缓存

use super::{Entry, png_metadata};
use std::collections::{HashSet, VecDeque};
use tiny_skia::Pixmap;

const CACHE_LIMIT: usize = 8 * 1024 * 1024;
const THUMBNAIL_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Model {
    pub entries: Vec<Entry>,
    pub visible: Vec<usize>,
    pub selected: Option<String>,
    pub scroll: usize,
    pub query: String,
    pub previews: VecDeque<(String, Pixmap)>,
    pub thumbnails: VecDeque<(String, Pixmap)>,
    pub failed: HashSet<String>,
    bytes: usize,
    thumbnail_bytes: usize,
}

impl Model {
    /// 【截图历史】【列表更新】以 entries 更新记录，保留可用选择；失效预览被释放，不改变搜索词
    pub fn refresh(&mut self, entries: Vec<Entry>) {
        let selected_index = self
            .visible
            .iter()
            .position(|index| self.selected.as_deref() == Some(self.entries[*index].name.as_str()))
            .unwrap_or(0);
        self.entries = entries;
        self.previews.clear();
        self.thumbnails.clear();
        self.failed.clear();
        self.bytes = 0;
        self.thumbnail_bytes = 0;
        self.filter();
        if !self
            .visible
            .iter()
            .any(|index| self.selected.as_deref() == Some(self.entries[*index].name.as_str()))
        {
            self.selected = self
                .visible
                .get(selected_index.min(self.visible.len().saturating_sub(1)))
                .map(|index| self.entries[*index].name.clone());
        }
    }

    /// 【截图历史】【搜索更新】使用 query 的空白分词进行 Unicode 不区分大小写匹配；保留仍可见选择
    pub fn search(&mut self, query: &str) {
        self.query = png_metadata::bounded_search(query);
        self.filter();
        self.scroll = 0;
        if !self
            .visible
            .iter()
            .any(|index| self.selected.as_deref() == Some(self.entries[*index].name.as_str()))
        {
            self.selected = self
                .visible
                .first()
                .map(|index| self.entries[*index].name.clone());
        }
    }

    /// 【截图历史】【过滤列表】按当前搜索词匹配文件名、日期、尺寸和标注文字；不执行自动文字识别
    fn filter(&mut self) {
        let tokens: Vec<_> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.visible = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let haystack = format!(
                    "{} {} {} {}x{} {}",
                    entry.name,
                    entry
                        .created
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S"),
                    entry.created.format("%Y-%m-%d %H:%M:%S"),
                    entry.width,
                    entry.height,
                    entry.search
                )
                .to_lowercase();
                tokens
                    .iter()
                    .all(|token| haystack.contains(token))
                    .then_some(index)
            })
            .collect();
    }

    /// 【截图历史】【活动记录】返回当前选择的记录；无选择或记录已经删除时返回空值
    pub fn selected(&self) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| Some(entry.name.as_str()) == self.selected.as_deref())
    }

    /// 【截图历史】【选择移动】将选择移到可见 index，rows 为视口行数；返回选择是否变化
    pub fn select(&mut self, index: usize, rows: usize) -> bool {
        let selected = self
            .visible
            .get(index)
            .map(|index| self.entries[*index].name.clone());
        let changed = self.selected != selected;
        self.selected = selected;
        self.scroll = self
            .scroll
            .min(index)
            .max(index.saturating_sub(rows.saturating_sub(1)));
        changed
    }

    /// 【截图历史】【选择索引】返回当前选择在搜索结果中的位置；无选择时返回零
    pub fn index(&self) -> usize {
        self.visible
            .iter()
            .position(|index| Some(self.entries[*index].name.as_str()) == self.selected.as_deref())
            .unwrap_or(0)
    }

    /// 【截图历史】【预览查询】返回 name 的缓存图像；不克隆像素，不触发磁盘读取
    pub fn preview(&self, name: &str) -> Option<&Pixmap> {
        self.previews
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, image)| image)
    }

    /// 【截图历史】【缩略图查询】返回 name 的列表缩略图；独立小图避免大预览淘汰后反复解码
    pub fn thumbnail(&self, name: &str) -> Option<&Pixmap> {
        self.thumbnails
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, image)| image)
    }

    /// 【截图历史】【缓存插入】保存 name 的 image 并淘汰最旧缓存；总像素字节不超过十二 MiB
    pub fn cache(&mut self, name: String, image: Pixmap) {
        if !self.entries.iter().any(|entry| entry.name == name) {
            return;
        }
        let fit = (80.0 / image.width() as f32)
            .min(48.0 / image.height() as f32)
            .min(1.0);
        let mut thumbnail = Pixmap::new(
            (image.width() as f32 * fit).round().max(1.0) as u32,
            (image.height() as f32 * fit).round().max(1.0) as u32,
        )
        .unwrap();
        thumbnail.draw_pixmap(
            0,
            0,
            image.as_ref(),
            &tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            tiny_skia::Transform::from_scale(fit, fit),
            None,
        );
        if let Some(index) = self.thumbnails.iter().position(|(key, _)| *key == name) {
            self.thumbnail_bytes -= self.thumbnails.remove(index).unwrap().1.data().len();
        }
        while self.thumbnail_bytes + thumbnail.data().len() > THUMBNAIL_LIMIT {
            if let Some((_, image)) = self.thumbnails.pop_front() {
                self.thumbnail_bytes -= image.data().len();
            } else {
                break;
            }
        }
        self.thumbnail_bytes += thumbnail.data().len();
        self.thumbnails.push_back((name.clone(), thumbnail));
        if let Some(index) = self.previews.iter().position(|(key, _)| *key == name) {
            self.bytes -= self.previews.remove(index).unwrap().1.data().len();
        }
        let bytes = image.data().len();
        if bytes > CACHE_LIMIT {
            return;
        }
        while self.bytes + bytes > CACHE_LIMIT {
            if let Some((_, image)) = self.previews.pop_front() {
                self.bytes -= image.data().len();
            } else {
                break;
            }
        }
        self.bytes += bytes;
        self.previews.push_back((name, image));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【截图历史测试】【搜索选择】日期、尺寸与中文标注参与搜索；刷新、删除保留相邻选择
    #[test]
    fn searching_and_refresh_keep_valid_selections() {
        let directory = tempfile::tempdir().unwrap();
        let store = super::super::Store::new(directory.path(), 1024 * 1024);
        let a = store
            .append(&super::super::tests::png(16, 9), 1.0, "会议 Alpha", 50)
            .unwrap();
        let b = store
            .append(&super::super::tests::png(17, 9), 1.0, "Other", 50)
            .unwrap();
        let mut model = Model::default();
        model.refresh(store.entries().unwrap());
        assert_eq!(model.selected.as_deref(), Some(b.name.as_str()));
        model.search("会议 ALPHA 16x9");
        assert_eq!(model.visible.len(), 1);
        assert_eq!(model.selected.as_deref(), Some(a.name.as_str()));
        model.refresh(store.entries().unwrap());
        assert_eq!(model.selected.as_deref(), Some(a.name.as_str()));
        model.search("");
        model.select(0, 3);
        store.remove(&b.name).unwrap();
        model.refresh(store.entries().unwrap());
        assert_eq!(model.selected.as_deref(), Some(a.name.as_str()));
        model.search("not found");
        assert!(model.selected.is_none());
    }

    /// 【截图历史测试】【缓存分离】大预览淘汰后仍保留可见缩略图，两个像素缓存分别满足字节上限
    #[test]
    fn evicting_large_previews_keeps_small_thumbnails() {
        let directory = tempfile::tempdir().unwrap();
        let store = super::super::Store::new(directory.path(), 1024 * 1024);
        for _ in 0..9 {
            store
                .append(&super::super::tests::png(1, 1), 1.0, "", 50)
                .unwrap();
        }
        let mut model = Model::default();
        model.refresh(store.entries().unwrap());
        let names: Vec<_> = model
            .entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        for name in &names {
            model.cache(name.clone(), Pixmap::new(800, 600).unwrap());
        }
        assert!(model.preview(&names[0]).is_none());
        assert!(names.iter().all(|name| model.thumbnail(name).is_some()));
        assert!(model.bytes <= CACHE_LIMIT && model.thumbnail_bytes <= THUMBNAIL_LIMIT);
    }
}
