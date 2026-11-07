//! 【区域识别】【Canny 边缘】平方梯度、方向抑制与八邻域滞后连接

use super::{Gray, gaussian};

/// 【区域识别】【梯度方向】按 gx、gy 量化方向，返回零、四十五、九十或一百三十五度档位
fn direction(gx: f32, gy: f32) -> u8 {
    let (ax, ay) = (gx.abs(), gy.abs());
    if ay <= ax * 0.414_213_56 {
        0
    } else if ax <= ay * 0.414_213_56 {
        2
    } else if gx * gy >= 0.0 {
        1
    } else {
        3
    }
}

/// 【区域识别】【边缘连接】对 image 应用 low、high 双阈值，返回二值边缘图，不保留临时浮点缓冲
pub(in crate::features::region_detection) fn detect(image: &Gray, low: f32, high: f32) -> Gray {
    let (width, height) = (image.width, image.height);
    let mut edges = Gray::new(width, height);
    if width < 3 || height < 3 {
        return edges;
    }
    let blurred = gaussian::blur(image);
    let mut magnitude = vec![0.0f32; width * height];
    let mut directions = vec![0; width * height];
    // 1. 【区域识别】【Sobel 梯度】使用平方幅值和四档方向，避免每像素角度与平方根运算
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let at = |x, y| f32::from(blurred.at(x, y));
            let gx = -at(x - 1, y - 1) + at(x + 1, y - 1) - 2.0 * at(x - 1, y) + 2.0 * at(x + 1, y)
                - at(x - 1, y + 1)
                + at(x + 1, y + 1);
            let gy = -at(x - 1, y - 1) - 2.0 * at(x, y - 1) - at(x + 1, y - 1)
                + at(x - 1, y + 1)
                + 2.0 * at(x, y + 1)
                + at(x + 1, y + 1);
            let index = y * width + x;
            magnitude[index] = gx * gx + gy * gy;
            directions[index] = direction(gx, gy);
        }
    }
    let mut classified = vec![0u8; width * height];
    let mut stack = Vec::with_capacity(width + height);
    // 2. 【区域识别】【非极大抑制】相同幅值时保留原版方向侧的像素，强边进入连接栈
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            let (a, b) = match directions[index] {
                0 => (index - 1, index + 1),
                1 => ((y - 1) * width + x + 1, (y + 1) * width + x - 1),
                2 => (index - width, index + width),
                _ => ((y - 1) * width + x - 1, (y + 1) * width + x + 1),
            };
            let value = magnitude[index];
            if value <= magnitude[a] || value < magnitude[b] {
                continue;
            }
            if value >= high * high {
                classified[index] = 255;
                stack.push(index as u32);
            } else if value >= low * low {
                classified[index] = 128;
            }
        }
    }
    // 3. 【区域识别】【滞后连接】只有与强边八邻域相连的弱边才保留，图像外缘保持为空
    while let Some(index) = stack.pop() {
        let index = index as usize;
        edges.pixels[index] = 255;
        let (x, y) = ((index % width) as i32, (index / width) as i32);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny) = (x + dx, y + dy);
                if dx == 0 && dy == 0
                    || nx <= 0
                    || ny <= 0
                    || nx as usize >= width - 1
                    || ny as usize >= height - 1
                {
                    continue;
                }
                let neighbor = ny as usize * width + nx as usize;
                if classified[neighbor] == 128 {
                    classified[neighbor] = 255;
                    stack.push(neighbor as u32);
                }
            }
        }
    }
    edges
}
