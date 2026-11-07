//! 【打开方式】【应用图标】只为可见行加载二十二像素图标，有限缓存避免保存完整桌面图标库

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tiny_skia::Pixmap;

#[derive(Default)]
pub(super) struct IconCache {
    roots: Option<Vec<PathBuf>>,
    entries: HashMap<String, Option<Pixmap>>,
}

impl IconCache {
    /// 【打开方式】【图标准备】按 name 返回已栅格化图标，首次查询才读取主题目录；缓存最多保留六十四项
    pub fn get(&mut self, name: &str) -> Option<&Pixmap> {
        if name.is_empty() {
            return None;
        }
        if !self.entries.contains_key(name) {
            if self.entries.len() >= 64 {
                self.entries.clear();
            }
            let roots = self.roots.get_or_insert_with(search_roots);
            let image = resolve(name, roots).and_then(|path| render(&path));
            self.entries.insert(name.to_owned(), image);
        }
        self.entries.get(name).and_then(Option::as_ref)
    }
}

/// 【打开方式】【主题根目录】返回用户和系统图标根目录，不扫描目录内容；图片应用常用主题优先，无参数
fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".icons"));
    }
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("icons"));
    }
    for directory in super::super::super::features::external_actions::desktop_apps::search_dirs() {
        if let Some(parent) = directory.parent() {
            roots.push(parent.join("icons"));
            roots.push(parent.join("pixmaps"));
        }
    }
    roots
}

/// 【打开方式】【图标路径】按 name 和 roots 定位绝对图标、常见主题和无主题图片，返回已有文件，不执行外部主题工具
fn resolve(name: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if Path::new(name).is_absolute() {
        return Path::new(name).is_file().then(|| PathBuf::from(name));
    }
    if name.contains('/') {
        return None;
    }
    let theme = std::env::var("XDG_ICON_THEME").unwrap_or_else(|_| "Adwaita".into());
    for root in roots {
        for theme in [theme.as_str(), "hicolor", "Adwaita", "breeze"] {
            for size in [
                "22x22", "24x24", "32x32", "48x48", "scalable", "symbolic", "16x16", "64x64",
                "256x256",
            ] {
                for extension in ["png", "svg", "xpm"] {
                    let path = root
                        .join(theme)
                        .join(size)
                        .join("apps")
                        .join(format!("{name}.{extension}"));
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
        for filename in [
            name.to_owned(),
            format!("{name}.png"),
            format!("{name}.svg"),
        ] {
            let path = root.join(filename);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

/// 【打开方式】【图标栅格化】把 path 图标有界解码到二十二像素方块，返回预乘像素；失败时由界面使用统一图标
fn render(path: &Path) -> Option<Pixmap> {
    let mut result = Pixmap::new(22, 22)?;
    if path.extension().is_some_and(|extension| extension == "svg") {
        if std::fs::metadata(path).ok()?.len() > 1024 * 1024 {
            return None;
        }
        let tree =
            usvg::Tree::from_data(&std::fs::read(path).ok()?, &usvg::Options::default()).ok()?;
        let size = tree.size();
        let scale = (22.0 / size.width()).min(22.0 / size.height());
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale).post_translate(
                (22.0 - size.width() * scale) / 2.0,
                (22.0 - size.height() * scale) / 2.0,
            ),
            &mut result.as_mut(),
        );
    } else {
        let mut reader = image::ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(16 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().ok()?.thumbnail(22, 22).to_rgba8();
        let (x, y) = ((22 - image.width()) / 2, (22 - image.height()) / 2);
        for (px, py, pixel) in image.enumerate_pixels() {
            let offset = ((py + y) * 22 + px + x) as usize * 4;
            for channel in 0..3 {
                result.data_mut()[offset + channel] =
                    ((u16::from(pixel[channel]) * u16::from(pixel[3]) + 127) / 255) as u8;
            }
            result.data_mut()[offset + 3] = pixel[3];
        }
    }
    Some(result)
}
