//! 【界面字体】【按需栅格】用 FreeType 应用 Fontconfig 渲染设置，界面缓存独立于标注导出

mod bitmap;
pub(crate) mod blend;
mod variation;
pub(crate) use bitmap::{Content, Glyph};

use super::render_settings::{Config, Settings};
use cosmic_text::{CacheKey, CacheKeyFlags, FontSystem, SubpixelBin, fontdb::Source};
use freetype::{Face, Library, Matrix, RenderMode, Vector, face::LoadFlag};
use std::{borrow::Borrow, cell::RefCell, collections::HashMap, path::PathBuf, sync::Arc};
#[cfg(test)]
mod tests;

#[derive(Clone, Hash, PartialEq, Eq)]
struct FaceKey {
    file: PathBuf,
    index: u32,
}
#[derive(Clone, Hash, PartialEq, Eq)]
struct GlyphKey {
    face: FaceKey,
    glyph: u16,
    size: u32,
    weight: u16,
    flags: CacheKeyFlags,
    x: SubpixelBin,
    y: SubpixelBin,
    lcd: bool,
}

struct FaceBytes(Arc<dyn AsRef<[u8]> + Send + Sync>);
impl Borrow<[u8]> for FaceBytes {
    /// 【界面字体】【共享字节】无参数，借用排版已经映射的字体数据，无复制，返回字节切片
    fn borrow(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}

struct Raster {
    faces: HashMap<FaceKey, Face<FaceBytes>>,
    settings: HashMap<(FaceKey, u32, u16), Settings>,
    glyphs: HashMap<GlyphKey, Glyph>,
    bytes: usize,
    library: Library,
    config: Config,
}

thread_local! {
    static RASTER: RefCell<Option<Raster>> = const { RefCell::new(None) };
}

impl Raster {
    /// 【界面字体】【绘制器创建】无参数，建立当前线程的配置和 FreeType 库，失败返回空值，不新增线程
    fn new() -> Option<Self> {
        Some(Self {
            faces: HashMap::new(),
            settings: HashMap::new(),
            glyphs: HashMap::new(),
            bytes: 0,
            library: Library::init().ok()?,
            config: Config::new()?,
        })
    }

    /// 【界面字体】【字形缓存】按 key、family 返回字形，data 为现有排版映射，已有字体可传空；缺字体且为空时失败，不复制整文件
    fn image(
        &mut self,
        key: GlyphKey,
        family: &str,
        data: Option<Arc<dyn AsRef<[u8]> + Send + Sync>>,
    ) -> Option<&Glyph> {
        if !self.glyphs.contains_key(&key) {
            if !self.faces.contains_key(&key.face) {
                // 1. 【界面字体】【字体预算】只打开当前使用的字体，保留最多八个 FreeType 字体对象
                if self.faces.len() >= 8 {
                    self.faces.clear();
                }
                self.faces.insert(
                    key.face.clone(),
                    self.library
                        .new_memory_face2(FaceBytes(data?), key.face.index as isize)
                        .ok()?,
                );
            }
            let settings_key = (key.face.clone(), key.size, key.weight);
            if !self.settings.contains_key(&settings_key) {
                if self.settings.len() >= 128 {
                    self.settings.clear();
                }
                self.settings.insert(
                    settings_key.clone(),
                    self.config.settings(
                        &key.face.file,
                        key.face.index,
                        family,
                        f32::from_bits(key.size),
                        key.weight,
                    ),
                );
            }
            let settings = self.settings[&settings_key];
            let glyph = render(&self.library, &self.faces[&key.face], &key, settings)?;
            // 2. 【界面字体】【遮罩预算】最多保留四 MiB 字形像素及四千个键，达到预算时清除旧遮罩
            if self.bytes + glyph.data.len() > 4 * 1024 * 1024 || self.glyphs.len() >= 4096 {
                self.glyphs.clear();
                self.bytes = 0;
            }
            self.bytes += glyph.data.len();
            self.glyphs.insert(key.clone(), glyph);
        }
        self.glyphs.get(&key)
    }
}

/// 【界面字体】【遮罩访问】按 fonts、cache_key 与 lcd 访问系统字体字形，f 只借用位图；成功返回真，失败继续原有绘制
pub(crate) fn with_image(
    fonts: &FontSystem,
    cache_key: CacheKey,
    lcd: bool,
    f: impl FnOnce(&Glyph),
) -> bool {
    let Some(face) = fonts.db().face(cache_key.font_id) else {
        return false;
    };
    let (file, data) = match &face.source {
        Source::SharedFile(path, data) => (path, Arc::clone(data)),
        _ => return false,
    };
    let key = GlyphKey {
        face: FaceKey {
            file: file.clone(),
            index: face.index,
        },
        glyph: cache_key.glyph_id,
        size: cache_key.font_size_bits,
        weight: cache_key.font_weight.0,
        flags: cache_key.flags,
        x: cache_key.x_bin,
        y: cache_key.y_bin,
        lcd,
    };
    let family = face
        .families
        .first()
        .map_or("sans-serif", |family| family.0.as_str());
    RASTER.with(|state| {
        let mut state = state.borrow_mut();
        if state.is_none() {
            *state = Raster::new();
        }
        let Some(image) = state
            .as_mut()
            .and_then(|raster| raster.image(key, family, Some(data)))
        else {
            return false;
        };
        f(image);
        true
    })
}

/// 【界面字体】【灰度回调】按 fonts、key 与 color 输出界面编辑器像素，f 接收物理位置及颜色，失败返回假供原路径回退
pub(crate) fn with_pixels(
    fonts: &FontSystem,
    key: CacheKey,
    color: cosmic_text::Color,
    mut f: impl FnMut(i32, i32, cosmic_text::Color),
) -> bool {
    with_image(fonts, key, false, |image| {
        let channels = match image.content {
            Content::Mask => 1,
            Content::Subpixel => 3,
            Content::Color => 4,
        };
        for y in 0..image.height {
            for x in 0..image.width {
                let mask = &image.data[((y * image.width + x) as usize) * channels..][..channels];
                let pixel = if image.content == Content::Color {
                    if mask[3] == 0 {
                        continue;
                    }
                    let straight = |value: u8| {
                        ((u32::from(value) * 255 + u32::from(mask[3]) / 2) / u32::from(mask[3]))
                            .min(255) as u8
                    };
                    cosmic_text::Color::rgba(
                        straight(mask[0]),
                        straight(mask[1]),
                        straight(mask[2]),
                        ((u32::from(mask[3]) * u32::from(color.a()) + 127) / 255) as u8,
                    )
                } else {
                    let alpha =
                        mask.iter().map(|value| u32::from(*value)).sum::<u32>() / channels as u32;
                    cosmic_text::Color::rgba(
                        color.r(),
                        color.g(),
                        color.b(),
                        ((alpha * u32::from(color.a()) + 127) / 255) as u8,
                    )
                };
                f(x as i32 + image.left, y as i32 - image.top, pixel);
            }
        }
    })
}

/// 【界面字体】【FreeType 绘制】按 library、face、key 与 settings 返回真实字形，应用字号、字重、微调、抗锯齿和 LCD 滤镜
fn render<B>(
    library: &Library,
    face: &Face<B>,
    key: &GlyphKey,
    settings: Settings,
) -> Option<Glyph> {
    let size = f32::from_bits(key.size);
    if !size.is_finite() || !(0.0..=1024.0).contains(&size) || size == 0.0 {
        return None;
    }
    // 1. 【界面字体】【度量同步】字号、字重和亚像素位置保持与排版缓存一致
    variation::set_weight(library, face, key.weight);
    face.set_char_size(0, (size * 64.0).round() as isize, 72, 72)
        .ok()?;
    let italic = key.flags.contains(CacheKeyFlags::FAKE_ITALIC);
    let mut matrix = Matrix {
        xx: 65536,
        xy: if italic { 16341 } else { 0 },
        yx: 0,
        yy: 65536,
    };
    let mut offset = Vector {
        x: (key.x.as_float() * 64.0) as _,
        y: -(key.y.as_float() * 64.0) as i64,
    };
    face.set_transform(&mut matrix, &mut offset);
    let lcd = key.lcd && settings.antialias && matches!(settings.rgba, 1..=4);
    let vertical = matches!(settings.rgba, 3 | 4);
    let mode = if !settings.antialias {
        RenderMode::Mono
    } else if lcd && vertical {
        RenderMode::LcdV
    } else if lcd {
        RenderMode::Lcd
    } else {
        RenderMode::Normal
    };
    // 2. 【界面字体】【微调分离】轻微微调独立于 LCD 输出模式，保留 Fontconfig 对轮廓的选择
    let mut flags = if !settings.antialias {
        LoadFlag::TARGET_MONO | LoadFlag::MONOCHROME
    } else if settings.hintstyle == 1 {
        LoadFlag::TARGET_LIGHT
    } else if lcd && vertical {
        LoadFlag::TARGET_LCD_V
    } else if lcd {
        LoadFlag::TARGET_LCD
    } else {
        LoadFlag::TARGET_NORMAL
    };
    if !settings.hinting
        || settings.hintstyle == 0
        || key.flags.contains(CacheKeyFlags::DISABLE_HINTING)
    {
        flags |= LoadFlag::NO_HINTING;
    }
    if settings.autohint {
        flags |= LoadFlag::FORCE_AUTOHINT;
    }
    if !settings.embedded_bitmap || lcd {
        flags |= LoadFlag::NO_BITMAP;
    }
    if lcd {
        set_lcd_filter(library, settings.lcd_filter)?;
    }
    // 3. 【界面字体】【位图生成】只复制当前字形像素，不保留 FreeType 可变位图指针
    face.load_glyph(u32::from(key.glyph), flags | LoadFlag::COLOR)
        .ok()?;
    face.glyph().render_glyph(mode).ok()?;
    bitmap::convert(
        &face.glyph().bitmap(),
        face.glyph().bitmap_left(),
        face.glyph().bitmap_top(),
        matches!(settings.rgba, 2 | 4),
    )
}

/// 【界面字体】【LCD 滤镜】按 library 与 filter 设置原生滤镜，旧滤镜不可用时采用默认滤镜，返回成功或空值
fn set_lcd_filter(library: &Library, filter: i32) -> Option<()> {
    let native = match filter {
        0 => 0,
        2 => 2,
        3 => 16,
        _ => 1,
    };
    // 1. 【界面字体】【原生枚举】FreeType 的 LIGHT 正式值为 2，当前绑定误写为 LEGACY1 的 3
    let configured = unsafe { freetype::ffi::FT_Library_SetLcdFilter(library.raw(), native) };
    // 2. 【界面字体】【旧滤镜兼容】部分 FreeType 构建不启用旧滤镜，继续默认 LCD 滤镜，不能因此丢失字形
    (configured == 0 || unsafe { freetype::ffi::FT_Library_SetLcdFilter(library.raw(), 1) } == 0)
        .then_some(())
}
