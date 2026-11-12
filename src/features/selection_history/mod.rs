//! 【历史选区】【有界记录】保存完成选区的位置尺寸，同一显示器布局下按时间顺序前后切换

mod storage;

use crate::types::Placement;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tiny_skia::Rect;

const LIMIT: usize = 32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    rect: [f32; 4],
    displays: Vec<(i32, i32, i32, i32, f32)>,
}

#[derive(Default)]
pub(crate) struct History {
    entries: Vec<Entry>,
    cursor: Option<usize>,
    path: Option<PathBuf>,
}

/// 【历史选区】【显示器签名】按 placements 返回排序后的坐标尺寸与缩放，避免改变屏幕布局后恢复到错误位置
fn displays(placements: &[Placement]) -> Vec<(i32, i32, i32, i32, f32)> {
    let mut values: Vec<_> = placements
        .iter()
        .map(|p| (p.position.0, p.position.1, p.size.0, p.size.1, p.scale))
        .collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values
}

impl History {
    /// 【历史选区】【本地读取】按 path 读取有限记录，缺失文件使用空历史，损坏文件记录错误且不影响截图
    pub(crate) fn load(path: PathBuf) -> Self {
        let entries = match storage::read(&path) {
            Ok(entries) => entries,
            Err(error) => {
                log::error!("【历史选区】【读取记录】{error}");
                Vec::new()
            }
        };
        Self {
            entries,
            cursor: None,
            path: Some(path),
        }
    }

    /// 【历史选区】【记录完成】按 rect 和 placements 去重并记录，返回是否新增；只保存坐标和显示器布局
    pub(crate) fn record(&mut self, rect: Rect, placements: &[Placement]) -> bool {
        let entry = Entry {
            rect: [rect.left(), rect.top(), rect.width(), rect.height()],
            displays: displays(placements),
        };
        if self.entries.last() == Some(&entry) {
            self.cursor = self.entries.len().checked_sub(1);
            return false;
        }
        self.entries.retain(|previous| previous != &entry);
        self.entries.push(entry);
        if self.entries.len() > LIMIT {
            self.entries.remove(0);
        }
        self.cursor = self.entries.len().checked_sub(1);
        if let Some(path) = &self.path {
            match storage::record(path, self.entries.last().unwrap()) {
                Ok(entries) => {
                    self.entries = entries;
                    self.cursor = self.entries.len().checked_sub(1);
                }
                Err(error) => log::error!("【历史选区】【保存记录】{error}"),
            }
        }
        true
    }

    /// 【历史选区】【前后浏览】按 forward 与当前 placements 返回相邻有效矩形，到达首尾保持原位，不产生新的历史
    pub(crate) fn step(&mut self, forward: bool, placements: &[Placement]) -> Option<Rect> {
        if let Some(path) = &self.path
            && let Ok(entries) = storage::read(path)
        {
            let current = self.cursor.and_then(|index| self.entries.get(index));
            self.cursor =
                current.and_then(|current| entries.iter().position(|entry| entry == current));
            self.entries = entries;
        }
        let signature = displays(placements);
        let indices: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.displays == signature)
            .map(|(index, _)| index)
            .collect();
        let current = self
            .cursor
            .and_then(|cursor| indices.iter().position(|index| *index == cursor));
        let position = match current {
            Some(index) => {
                if forward {
                    (index + 1).min(indices.len().saturating_sub(1))
                } else {
                    index.saturating_sub(1)
                }
            }
            None => indices.len().checked_sub(1)?,
        };
        let index = *indices.get(position)?;
        self.cursor = Some(index);
        let [x, y, width, height] = self.entries[index].rect;
        let rect = Rect::from_xywh(x, y, width, height)?;
        let bounds = crate::utils::get_full_workspace_rect(placements)?;
        (rect.left() >= bounds.left()
            && rect.top() >= bounds.top()
            && rect.right() <= bounds.right()
            && rect.bottom() <= bounds.bottom())
        .then_some(rect)
    }
}
