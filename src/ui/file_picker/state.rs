//! 【文件选择】【状态】目录列表、文件名和保存对话框生命周期

use super::{layout::Layout, save::ExistingFile};
use crate::ui::fonts::UiFonts;
use crate::ui::single_line::SingleLine;
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

pub struct FileItem {
    pub name: OsString,
    pub path: PathBuf,
    pub directory: bool,
    pub bytes: u64,
}

pub struct Listing {
    pub directory: PathBuf,
    pub items: Vec<FileItem>,
    pub truncated: bool,
}

impl Listing {
    /// 【文件选择】【目录读取】读取 path 中目录和 PNG 普通文件，hidden 决定隐藏项；最多返回一万项
    pub fn read(path: &Path, hidden: bool) -> io::Result<Self> {
        let directory = fs::canonicalize(path)?;
        let mut items = Vec::new();
        let mut truncated = false;
        for (count, item) in fs::read_dir(&directory)?.enumerate() {
            if count >= 10_000 {
                truncated = true;
                break;
            }
            let item = item?;
            let name = item.file_name();
            if !hidden && name.to_string_lossy().starts_with('.') {
                continue;
            }
            let path = item.path();
            let Ok(metadata) = item.metadata() else {
                continue;
            };
            let folder = metadata.is_dir();
            if !folder
                && (!metadata.is_file()
                    || !path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("png")))
            {
                continue;
            }
            items.push(FileItem {
                name,
                path,
                directory: folder,
                bytes: metadata.len(),
            });
        }
        items.sort_by_cached_key(|item| {
            (!item.directory, item.name.to_string_lossy().to_lowercase())
        });
        Ok(Self {
            directory,
            items,
            truncated,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Path,
    Filename,
}

pub struct Picker {
    pub directory: PathBuf,
    pub path: SingleLine,
    pub filename: SingleLine,
    pub listing: Vec<FileItem>,
    pub layout: Layout,
    pub focus: Option<Field>,
    pub selected: Option<usize>,
    pub scroll: usize,
    pub hidden: bool,
    pub pending: Option<PathBuf>,
    pub status: Option<String>,
    pub overwrite: Option<(PathBuf, ExistingFile)>,
    pub confirm_replace: bool,
    pub creating_folder: bool,
    pub previous_filename: Option<String>,
    pub dragging: bool,
    pub busy: bool,
}

impl Picker {
    /// 【文件选择】【对话框构造】使用 directory、filename、size 初始化保存界面，fonts 用于输入；不读取文件或创建目录
    pub fn new(directory: PathBuf, filename: &str, size: (u32, u32), fonts: &mut UiFonts) -> Self {
        let path = SingleLine::new(&directory.to_string_lossy(), fonts, 4096);
        let mut filename = SingleLine::new(filename, fonts, 255);
        filename.select_all(fonts);
        Self {
            directory: directory.clone(),
            path,
            filename,
            listing: Vec::new(),
            layout: Layout::new(size),
            focus: Some(Field::Filename),
            selected: None,
            scroll: 0,
            hidden: false,
            pending: Some(directory),
            status: None,
            overwrite: None,
            confirm_replace: false,
            creating_folder: false,
            previous_filename: None,
            dragging: false,
            busy: false,
        }
    }

    /// 【文件选择】【目录应用】应用 request 对应 listing，fonts 用于路径字段；忽略已过期目录请求
    pub fn apply(&mut self, request: &Path, listing: io::Result<Listing>, fonts: &mut UiFonts) {
        if self.pending.as_deref() != Some(request) {
            return;
        }
        self.pending = None;
        match listing {
            Ok(listing) => {
                for item in &listing.items {
                    fonts.ensure_text(&item.name.to_string_lossy());
                }
                self.directory = listing.directory;
                self.path
                    .set(&self.directory.to_string_lossy(), fonts, false);
                self.listing = listing.items;
                self.scroll = 0;
                self.selected = None;
                self.status = listing
                    .truncated
                    .then(|| "Showing the first 10,000 directory entries".into());
            }
            Err(error) => self.status = Some(format!("Cannot open folder: {error}")),
        }
    }

    /// 【文件选择】【目标文件名】根据 filename 返回当前目录下 PNG 目标；拒绝空名称和路径穿越
    pub fn target(&self) -> io::Result<PathBuf> {
        let mut filename = self.filename.text();
        if filename.is_empty()
            || filename.trim().is_empty()
            || filename.contains(['/', '\\'])
            || matches!(filename.as_str(), "." | "..")
        {
            return Err(io::Error::other("enter a plain filename, not a path"));
        }
        if !filename.to_ascii_lowercase().ends_with(".png") {
            filename.push_str(".png");
        }
        if filename.len() > 255 {
            return Err(io::Error::other("filename is too long"));
        }
        Ok(self.directory.join(filename))
    }

    /// 【文件选择】【字段定位】返回 field 的输入控件和区域；用于共用键盘与鼠标操作
    pub fn field(&mut self, field: Field) -> (&mut SingleLine, tiny_skia::Rect) {
        match field {
            Field::Path => (&mut self.path, self.layout.path_text),
            Field::Filename => (&mut self.filename, self.layout.filename_text),
        }
    }

    /// 【文件选择】【滚动范围】把 scroll 限制到列表有效范围；返回当前可见行数
    pub fn clamp_scroll(&mut self) -> usize {
        let rows = (self.layout.list.height() / 28.0).floor().max(1.0) as usize;
        self.scroll = self.scroll.min(self.listing.len().saturating_sub(rows));
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【文件选择测试】【目录内容】目录排序优先，显示 PNG 文件并按开关处理隐藏项
    #[test]
    fn directory_listing_is_filtered_and_sorted() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("folder")).unwrap();
        for name in ["b.png", "A.PNG", ".hidden.png", "notes.txt"] {
            fs::write(directory.path().join(name), b"content").unwrap();
        }
        let listing = Listing::read(directory.path(), false).unwrap();
        assert_eq!(
            listing
                .items
                .iter()
                .map(|item| item.name.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["folder", "A.PNG", "b.png"]
        );
        assert_eq!(
            Listing::read(directory.path(), true).unwrap().items.len(),
            4
        );
    }
    /// 【文件选择测试】【目标校验】目标只接受当前目录内单个文件名，自动补充 PNG 后缀
    #[test]
    fn save_target_rejects_paths_and_defaults_to_png() {
        let mut fonts = UiFonts::new();
        let directory = PathBuf::from("/tmp");
        let mut picker = Picker::new(directory, "meeting", (760, 460), &mut fonts);
        assert_eq!(picker.target().unwrap(), PathBuf::from("/tmp/meeting.png"));
        for filename in ["", "../file", "/file", "..", "a\\b"] {
            picker.filename.set(filename, &mut fonts, false);
            assert!(picker.target().is_err(), "{filename}");
        }
    }
}
