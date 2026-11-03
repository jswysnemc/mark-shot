//! 【画笔缓存】【像素备份】按触及的像素块保存原始图层，提交和取消时精确恢复

use std::collections::HashSet;

use tiny_skia::{IntRect, Pixmap};

const TILE_SIZE: u32 = 64;

struct Tile {
    monitor_idx: usize,
    original_size: (u32, u32),
    rect: IntRect,
    data_offset: usize,
}

#[derive(Default)]
pub struct PenLayerBackup {
    tiles: Vec<Tile>,
    recorded: HashSet<(usize, u32, u32, u32, u32)>,
    pixels: Vec<u8>,
}

impl PenLayerBackup {
    /// 【画笔缓存】【像素备份】首次保存 monitor_idx 的 layer 中 rect 触及的像素块；返回新增字节数，不覆盖旧备份
    pub fn snapshot(&mut self, monitor_idx: usize, layer: &Pixmap, rect: IntRect) -> usize {
        let size = (layer.width(), layer.height());
        let Some(bounds) = IntRect::from_xywh(0, 0, size.0, size.1) else {
            return 0;
        };
        let Some(rect) = rect.intersect(&bounds) else {
            return 0;
        };
        let before = self.pixels.len();
        let stride = size.0 as usize * 4;

        // 1. 【画笔缓存】【像素备份】同一手势只保存每个像素块第一次写入前的内容
        for tile_y in rect.y() as u32 / TILE_SIZE..=(rect.bottom() as u32 - 1) / TILE_SIZE {
            for tile_x in rect.x() as u32 / TILE_SIZE..=(rect.right() as u32 - 1) / TILE_SIZE {
                if !self
                    .recorded
                    .insert((monitor_idx, size.0, size.1, tile_x, tile_y))
                {
                    continue;
                }
                let x = tile_x * TILE_SIZE;
                let y = tile_y * TILE_SIZE;
                let width = TILE_SIZE.min(size.0 - x);
                let height = TILE_SIZE.min(size.1 - y);
                let data_offset = self.pixels.len();

                // 2. 【画笔缓存】【像素备份】共享连续字节缓冲，避免每块像素单独分配长期内存
                self.pixels.reserve(width as usize * height as usize * 4);
                for row in y..y + height {
                    let offset = row as usize * stride + x as usize * 4;
                    self.pixels
                        .extend_from_slice(&layer.data()[offset..offset + width as usize * 4]);
                }
                self.tiles.push(Tile {
                    monitor_idx,
                    original_size: size,
                    rect: IntRect::from_xywh(x as i32, y as i32, width, height).unwrap(),
                    data_offset,
                });
            }
        }
        self.pixels.len() - before
    }

    /// 【画笔缓存】【像素恢复】恢复 layers 中尺寸未变的原始像素并清空备份；返回显示器索引和合并后的像素损坏区域
    pub fn restore(&mut self, layers: &mut [Pixmap]) -> Vec<(usize, IntRect)> {
        let mut damage = Vec::new();
        for tile in &self.tiles {
            let Some(layer) = layers.get_mut(tile.monitor_idx) else {
                continue;
            };
            if (layer.width(), layer.height()) != tile.original_size {
                continue;
            }
            let stride = layer.width() as usize * 4;
            let row_bytes = tile.rect.width() as usize * 4;

            // 1. 【画笔缓存】【像素恢复】直接复制预乘像素，不重新光栅化其他标注
            for row in 0..tile.rect.height() as usize {
                let source = tile.data_offset + row * row_bytes;
                let target = (tile.rect.y() as usize + row) * stride + tile.rect.x() as usize * 4;
                layer.data_mut()[target..target + row_bytes]
                    .copy_from_slice(&self.pixels[source..source + row_bytes]);
            }
            merge_damage(&mut damage, tile.monitor_idx, tile.rect);
        }

        // 2. 【画笔缓存】【像素恢复】释放大字节缓冲，空闲状态只保留少量索引容量
        self.discard();
        damage
    }

    /// 【画笔缓存】【备份释放】丢弃本次手势的备份内容，不修改图层；无返回值
    pub fn discard(&mut self) {
        self.tiles.clear();
        self.recorded.clear();
        self.pixels = Vec::new();
    }

    /// 【画笔缓存】【备份判断】返回当前是否没有需要恢复的原始像素
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// 【画笔缓存】【字节统计】返回当前保存的原始像素字节数，不包含少量索引容量
    pub fn snapshot_bytes(&self) -> usize {
        self.pixels.len()
    }
}

/// 【画笔缓存】【损坏合并】将 monitor_idx 的 rect 并入 damage，同一显示器只保留一个像素区域；无返回值
fn merge_damage(damage: &mut Vec<(usize, IntRect)>, monitor_idx: usize, rect: IntRect) {
    if let Some((_, existing)) = damage.iter_mut().find(|(index, _)| *index == monitor_idx) {
        *existing = IntRect::from_ltrb(
            existing.left().min(rect.left()),
            existing.top().min(rect.top()),
            existing.right().max(rect.right()),
            existing.bottom().max(rect.bottom()),
        )
        .unwrap();
    } else {
        damage.push((monitor_idx, rect));
    }
}
