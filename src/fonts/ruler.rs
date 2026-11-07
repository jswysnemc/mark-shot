//! 【尺子】【字体度量】按需加载原版无衬线半粗体，窗口定点度量独立于全局界面字体

use cosmic_text::{Attrs, Buffer, FontSystem, LayoutLine, Metrics, Shaping, Weight, Wrap};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Measure {
    pub width: f32,
    pub ascent: f32,
    pub height: f32,
}

struct Cache {
    fonts: FontSystem,
    values: HashMap<(u32, String), Measure>,
}

/// 【尺子】【指定字库】按fonts定向加载Sans Serif半粗体，返回实际家族，不改变默认无衬线家族
fn family(fonts: &mut FontSystem) -> String {
    super::text_faces::family(fonts, "Sans Serif", true, false)
}

/// 【尺子】【定点排版】按fonts、text和pixels返回原版整数窗口字号的缓冲与定点布局
pub(crate) fn shape(fonts: &mut FontSystem, text: &str, pixels: f32) -> (Buffer, LayoutLine) {
    let family = family(fonts);
    let mut buffer = Buffer::new(fonts, Metrics::new(pixels, pixels * 2.0));
    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &Attrs::new()
            .family(cosmic_text::Family::Name(&family))
            .weight(Weight(600)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let line = super::fixed_layout::lines(fonts, &buffer, pixels, None, Wrap::None)
        .into_iter()
        .next()
        .expect("ruler text has one layout line");
    (buffer, line)
}

/// 【尺子】【缓存测量】按text与整数pixels返回窗口字宽、上升部和行高，最多缓存256项，首次使用前不读取系统字体
pub(crate) fn measure(text: &str, pixels: f32) -> Measure {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| {
            Mutex::new(Cache {
                fonts: super::bundled(),
                values: HashMap::new(),
            })
        })
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let key = (pixels.to_bits(), text.to_owned());
    if let Some(value) = cache.values.get(&key) {
        return *value;
    }
    let (_, line) = shape(&mut cache.fonts, text, pixels);
    let fixed = |value: f32| ((f64::from(value) * 64.0 + 0.0001).floor() / 64.0) as f32;
    let value = Measure {
        width: line.w,
        ascent: fixed(line.max_ascent),
        height: fixed(line.max_ascent) + fixed(line.max_descent),
    };
    // 1. 【尺子】【缓存限制】连续拖动的测量值变化不增加永久缓存容量
    if cache.values.len() >= 256 {
        cache.values.clear();
    }
    cache.values.insert(key, value);
    value
}
