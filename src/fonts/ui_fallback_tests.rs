//! 【界面字体回归】【按需解析】合并系统目录后，中文缺字检查不得读取无关家族的字体数据

use cosmic_text::{FontSystem, fontdb};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct TrackedFont {
    reads: Arc<AtomicUsize>,
}

impl AsRef<[u8]> for TrackedFont {
    /// 【界面字体回归】【数据探针】无参数，记录实际字体数据访问并返回内置字体字节，不依赖进程常驻页估算
    fn as_ref(&self) -> &[u8] {
        self.reads.fetch_add(1, Ordering::Relaxed);
        super::super::BUNDLED[0]
    }
}

/// 【界面字体回归】【目录构造】无参数，返回已合并多个无关家族的字体系统和数据访问计数，不主动解析这些家族
fn catalog() -> (FontSystem, Arc<AtomicUsize>) {
    let mut fonts = super::super::bundled();
    let reads = Arc::new(AtomicUsize::new(0));
    let prototype = fonts.db().faces().next().unwrap().clone();
    let mut database = fontdb::Database::new();
    for index in 0..32 {
        let mut face = prototype.clone();
        face.families = vec![(
            format!("Unused Test Family {index}"),
            fontdb::Language::English_UnitedStates,
        )];
        face.post_script_name = format!("UnusedTestFamily{index}");
        face.source = fontdb::Source::Binary(Arc::new(TrackedFont {
            reads: reads.clone(),
        }));
        database.push_face_info(face);
    }
    super::super::add_system(&mut fonts, database);
    reads.store(0, Ordering::Relaxed);
    (fonts, reads)
}

/// 【界面字体回归】【目录隔离】无参数，验证中文滚动状态和混合正文只加载实际回退，重复调用也不解析无关家族
#[test]
fn chinese_preparation_does_not_parse_the_system_catalog() {
    let prepare: [fn(&mut FontSystem, &str); 3] = [
        super::ensure_text,
        super::super::ocr_ui::ensure_text,
        super::super::selection_ui::prepare,
    ];
    for prepare in prepare {
        let (mut fonts, reads) = catalog();
        prepare(&mut fonts, "Waiting for scroll / Copy");
        assert_eq!(reads.load(Ordering::Relaxed), 0);
        for _ in 0..3 {
            prepare(
                &mut fonts,
                "等待滚动正在捕获已暂停捕获错误未找到重叠区域区域已调整 / 日本語",
            );
            assert_eq!(
                reads.load(Ordering::Relaxed),
                0,
                "preparing Chinese text parsed unrelated catalog fonts"
            );
        }
    }
}

/// 【界面字体回归】【显示字形】无参数，验证定向加载后实际排版仍提供中文和西文字形，常规与半粗体都不能出现缺字
#[test]
fn targeted_fallback_preserves_chinese_glyphs_and_weight() {
    for bold in [false, true] {
        let mut fonts = super::super::bundled();
        let text = "等待滚动 已暂停 Capture";
        super::ensure_family(&mut fonts, text, "Sans Serif", bold);
        let mut buffer =
            cosmic_text::Buffer::new(&mut fonts, cosmic_text::Metrics::new(14.0, 20.0));
        buffer.set_size(Some(500.0), Some(100.0));
        buffer.set_text(
            text,
            &cosmic_text::Attrs::new().weight(if bold {
                cosmic_text::Weight(600)
            } else {
                cosmic_text::Weight::NORMAL
            }),
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let glyphs = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .collect::<Vec<_>>();
        assert!(
            glyphs.len()
                >= text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .count()
        );
        assert!(glyphs.iter().all(|glyph| glyph.glyph_id != 0));
    }
}
