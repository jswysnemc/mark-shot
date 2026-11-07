//! 【区域识别】【连通域】确定性扫描标签和外接矩形

use super::Gray;
use crate::features::region_detection::PixelRect;

pub(in crate::features::region_detection) struct Component {
    pub rect: PixelRect,
    pub pixels: u32,
}

struct Labels(Vec<u32>);

impl Labels {
    /// 【区域识别】【根标签】按 label 压缩路径，返回当前连通域根编号
    fn root(&mut self, label: u32) -> u32 {
        let mut root = label;
        while self.0[root as usize] != root {
            root = self.0[root as usize];
        }
        let mut current = label;
        while current != root {
            let next = self.0[current as usize];
            self.0[current as usize] = root;
            current = next;
        }
        root
    }

    /// 【区域识别】【标签合并】按 a、b 合并连通域，无返回值，保持扫描侧标签为根
    fn join(&mut self, a: u32, b: u32) {
        let a = self.root(a);
        let b = self.root(b);
        if a != b {
            self.0[b as usize] = a;
        }
    }
}

/// 【区域识别】【连通域提取】按 image 和 eight 选择四邻域或八邻域，返回按首次扫描位置排序的非空域
pub(in crate::features::region_detection) fn connected(
    image: &Gray,
    eight: bool,
) -> Vec<Component> {
    let (width, height) = (image.width, image.height);
    let mut labels = Labels(vec![0]);
    let mut map = vec![0u32; width * height];
    // 1. 【区域识别】【标签扫描】先连接左侧和上方，再按八邻域连接左上与右上
    for y in 0..height {
        for x in 0..width {
            if image.at(x, y) == 0 {
                continue;
            }
            let mut current = if x > 0 { map[y * width + x - 1] } else { 0 };
            if y > 0 {
                let previous = (y - 1) * width;
                let mut consider = |label| {
                    if label == 0 {
                        return;
                    }
                    if current == 0 {
                        current = label;
                    } else if current != label {
                        labels.join(current, label);
                    }
                };
                consider(map[previous + x]);
                if eight {
                    if x > 0 {
                        consider(map[previous + x - 1]);
                    }
                    if x + 1 < width {
                        consider(map[previous + x + 1]);
                    }
                }
            }
            if current == 0 {
                current = labels.0.len() as u32;
                labels.0.push(current);
            }
            map[y * width + x] = current;
        }
    }
    // 2. 【区域识别】【矩形累计】按首次出现顺序压缩标签并累计完整外接边界和前景像素数
    let mut remap = vec![u32::MAX; labels.0.len()];
    let mut bounds: Vec<Component> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let label = map[y * width + x];
            if label == 0 {
                continue;
            }
            let root = labels.root(label) as usize;
            let index = if remap[root] != u32::MAX {
                remap[root] as usize
            } else {
                let index = bounds.len();
                bounds.push(Component {
                    rect: PixelRect::new(x as i32, y as i32, 1, 1),
                    pixels: 0,
                });
                remap[root] = index as u32;
                index
            };
            let item = &mut bounds[index];
            item.rect.left = item.rect.left.min(x as i32);
            item.rect.top = item.rect.top.min(y as i32);
            item.rect.right = item.rect.right.max(x as i32 + 1);
            item.rect.bottom = item.rect.bottom.max(y as i32 + 1);
            item.pixels += 1;
        }
    }
    bounds
}
