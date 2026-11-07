//! 【字体弹层】【有界预览】只保留当前可见行字库，页面变化时释放旧 FontSystem 和字形缓存

use crate::{fonts::font_catalog::FontCatalog, renderer::Painter, theme::toolbar::Palette};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, fontdb};
use tiny_skia::{Pixmap, Rect};

pub struct Preview {
    pub key: Vec<usize>,
    fonts: Option<FontSystem>,
    cache: SwashCache,
}
impl Default for Preview {
    /// 【字体弹层】【缓存初始化】返回未加载字库和字形的预览缓存
    fn default() -> Self {
        Self {
            key: Vec::new(),
            fonts: None,
            cache: SwashCache::new(),
        }
    }
}

impl Preview {
    /// 【字体弹层】【预览释放】释放可见字体与字形缓存，无参数和返回值，关闭面板后不保留外部字体
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 【字体弹层】【页面字库】按 catalog 和 visible 更新预览字库，重复页面不重复加载，无返回值
    pub fn prepare(&mut self, catalog: &FontCatalog, visible: &[usize]) {
        if self.key == visible && self.fonts.is_some() {
            return;
        }
        let mut db = fontdb::Database::new();
        for index in visible {
            let Some(family) = catalog.families.get(*index) else {
                continue;
            };
            let mut source = fontdb::Database::new();
            for face in &family.faces {
                if !source.faces().any(|loaded| matches!(&loaded.source, fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) if path == std::path::Path::new(&face.file))) {
                    let _ = source.load_font_file(&face.file);
                }
            }
            for face in source.faces() {
                if !family.faces.iter().any(|allowed| allowed.index & 0xffff == face.index && matches!(&face.source, fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) if path == std::path::Path::new(&allowed.file))) { continue; }
                let mut face = face.clone();
                face.families = vec![(
                    format!("Font Preview {index}"),
                    fontdb::Language::English_UnitedStates,
                )];
                db.push_face_info(face);
            }
        }
        // 1. 【字体弹层】【回退隔离】通用字体与缺字回退只影响预览字库，不改动正文或其他界面的字体
        let mut fonts = crate::fonts::bundled();
        crate::fonts::add_system(&mut fonts, db);
        self.fonts = Some(fonts);
        self.cache = SwashCache::new();
        self.key = visible.to_vec();
    }

    /// 【字体弹层】【可见文字】按 catalog、rows、list、scale 与 palette 绘制列表文字，active 指定选中家族，返回裁切图像
    pub fn render(
        &mut self,
        catalog: &FontCatalog,
        rows: &[(usize, Rect)],
        list: Rect,
        scale: f32,
        active: &str,
        palette: Palette,
    ) -> Option<Pixmap> {
        let visible = rows.iter().map(|(index, _)| *index).collect::<Vec<_>>();
        self.prepare(catalog, &visible);
        let mut image = Pixmap::new(
            (list.width() * scale).ceil() as u32,
            (list.height() * scale).ceil() as u32,
        )?;
        let fonts = self.fonts.as_mut()?;
        for (index, row) in rows {
            let family = &catalog.families[*index];
            let name = if family.faces.is_empty() {
                crate::fonts::text_faces::family(fonts, &family.name, false, false)
            } else {
                format!("Font Preview {index}")
            };
            let attrs = Attrs::new().family(cosmic_text::Family::Name(&name));
            let mut text = family.name.clone();
            let width = row.width() - 42.0;
            let mut buffer = Buffer::new(fonts, Metrics::new(17.0, 24.0));
            buffer.set_wrap(cosmic_text::Wrap::None);
            buffer.set_text(&text, &attrs, Shaping::Advanced, None);
            buffer.shape_until_scroll(fonts, false);
            if buffer
                .layout_runs()
                .next()
                .is_some_and(|run| run.line_w > width)
            {
                while text.chars().count() > 1 {
                    text.pop();
                    buffer.set_text(&format!("{text}\u{2026}"), &attrs, Shaping::Advanced, None);
                    buffer.shape_until_scroll(fonts, false);
                    if buffer
                        .layout_runs()
                        .next()
                        .is_none_or(|run| run.line_w <= width)
                    {
                        break;
                    }
                }
            }
            let color = if family.name == active {
                palette.hover_text
            } else {
                palette.text
            };
            Painter::new(&mut image)
                .with_scale(scale)
                .with_text(fonts, &mut self.cache)
                .text_buffer(&buffer, (32.0, row.top() - list.top() + 2.0), color.color());
        }
        Some(image)
    }
}
