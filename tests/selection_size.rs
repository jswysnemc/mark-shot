//! 【选区尺寸测试】【原版对照】生产 Qt 控件和当前原版 Rust 几何生成的独立参考，不以本地实现生成预期

use mark_shot::{
    features::selection_size::{
        geometry::{self, Handle, Rect},
        number::{self, Validation},
        placement, ratio,
    },
    theme::{Mode, Rgba},
    ui::{
        language::Language,
        selection_size::{layout::Layout, palette::Colors},
        text_field::validation::Rule,
    },
};
use serde_json::Value;

#[path = "selection_size/field_state.rs"]
mod field_state;
#[path = "selection_size/navigation.rs"]
mod navigation;
#[path = "selection_size/ratio_popup.rs"]
mod ratio_popup;

/// 【选区尺寸测试】【生产参考】返回固定 Qt 参考，不访问用户桌面或现有配置
fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/qt-selection-size/reference.json")).unwrap()
}

/// 【选区尺寸测试】【整数尺寸】按 value 读取两个原版整数像素，返回宽高或边界
fn size(value: &Value) -> (i32, i32) {
    (
        value[0].as_i64().unwrap() as i32,
        value[1].as_i64().unwrap() as i32,
    )
}

/// 【选区尺寸测试】【图像坐标】按 value 返回双精度端点
fn point(value: &Value) -> (f64, f64) {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

/// 【选区尺寸测试】【图像矩形】按 value 返回原版双精度矩形
fn rect(value: &Value) -> Rect {
    Rect {
        x: value[0].as_f64().unwrap(),
        y: value[1].as_f64().unwrap(),
        width: value[2].as_f64().unwrap(),
        height: value[3].as_f64().unwrap(),
    }
}

/// 【选区尺寸测试】【窗口矩形】按 value 返回原版窗口逻辑矩形
fn window_rect(value: &Value) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
        value[2].as_f64().unwrap() as f32,
        value[3].as_f64().unwrap() as f32,
    )
    .unwrap()
}

/// 【选区尺寸测试】【创建约束】对照二十四个原版四方向创建样本，双精度结果严格一致
#[test]
fn creation_matches_production_geometry() {
    let reference = reference();
    for sample in reference["creations"].as_array().unwrap() {
        assert_eq!(
            geometry::create(
                point(&sample["anchor"]),
                point(&sample["pointer"]),
                sample["ratio"].as_f64().unwrap(),
                size(&sample["maximum"]),
            ),
            rect(&sample["result"]),
            "{sample}"
        );
    }
}

/// 【选区尺寸测试】【八方向调整】对照四十八个原版边角调整样本，不把移动约束混入缩放
#[test]
fn adjustment_matches_production_geometry() {
    let handles = [
        Handle::Left,
        Handle::Right,
        Handle::Top,
        Handle::Bottom,
        Handle::TopLeft,
        Handle::TopRight,
        Handle::BottomLeft,
        Handle::BottomRight,
    ];
    let reference = reference();
    for sample in reference["adjustments"].as_array().unwrap() {
        let handle = handles[(sample["handle"].as_u64().unwrap() - 10) as usize];
        assert_eq!(
            geometry::adjust(
                rect(&sample["before"]),
                handle,
                rect(&sample["adjusted"]),
                sample["ratio"].as_f64().unwrap(),
                size(&sample["maximum"]),
            ),
            rect(&sample["result"]),
            "{sample}"
        );
    }
}

/// 【选区尺寸测试】【尺寸与比例】对照上限反推、精确尺寸平移及六种预设，结果严格一致
#[test]
fn linked_and_exact_sizes_match_production_geometry() {
    let reference = reference();
    for sample in reference["linked"].as_array().unwrap() {
        let field = if sample["width"].as_bool().unwrap() {
            ratio::Field::Width
        } else {
            ratio::Field::Height
        };
        assert_eq!(
            ratio::linked(
                size(&sample["requested"]),
                field,
                sample["ratio"].as_f64().unwrap(),
                size(&sample["maximum"]),
            ),
            size(&sample["result"]),
            "{sample}"
        );
    }
    for sample in reference["resized"].as_array().unwrap() {
        assert_eq!(
            geometry::resize(
                rect(&sample["current"]),
                size(&sample["requested"]),
                size(&sample["maximum"]),
            ),
            rect(&sample["result"]),
            "{sample}"
        );
    }
    for sample in reference["presets"].as_array().unwrap() {
        assert_eq!(
            ratio::preset(sample["ratio"].as_f64().unwrap()),
            Some(sample["index"].as_u64().unwrap() as usize)
        );
    }
}

/// 【选区尺寸测试】【面板避让】对照十二个原版窗口位置与隐藏样本，不容许几何误差
#[test]
fn hud_placement_matches_production_layout() {
    let reference = reference();
    for sample in reference["placements"].as_array().unwrap() {
        let dimensions = size(&sample["size"]);
        let obstacles = sample["obstacles"]
            .as_array()
            .unwrap()
            .iter()
            .map(window_rect)
            .collect::<Vec<_>>();
        let expected = (!sample["result"].is_null()).then(|| window_rect(&sample["result"]));
        assert_eq!(
            placement::hud(
                window_rect(&sample["selection"]),
                (dimensions.0 as f32, dimensions.1 as f32),
                window_rect(&sample["bounds"]),
                &obstacles,
            ),
            expected,
            "{sample}"
        );
    }
}

/// 【选区尺寸测试】【整数字段验证】对照八十七个实际 Qt 验证状态，通用原生字段规则接受相同中间值
#[test]
fn integer_validation_matches_real_qt_validator() {
    let reference = reference();
    for sample in reference["validation"].as_array().unwrap() {
        let maximum = sample["maximum"].as_i64().unwrap() as i32;
        let text = sample["text"].as_str().unwrap();
        let state = match sample["state"].as_i64().unwrap() {
            0 => Validation::Invalid,
            1 => Validation::Intermediate,
            2 => Validation::Acceptable,
            _ => unreachable!(),
        };
        assert_eq!(number::validate(text, maximum), state, "{sample}");
        let rule = Rule::SelectionSize { maximum };
        assert_eq!(rule.accepts(text), state != Validation::Invalid);
        assert_eq!(rule.acceptable(text), state == Validation::Acceptable);
    }
}

/// 【选区尺寸测试】【字体布局】使用原版参考家族对照全部字段与图标区域，数值变化不能改变面板尺寸
#[test]
fn compact_hud_dimensions_match_real_qt_widget() {
    let reference = reference();
    let mut fonts = cosmic_text::FontSystem::new();
    fonts
        .db_mut()
        .set_sans_serif_family(reference["font"].as_str().unwrap());
    let layout = Layout::new(&mut fonts, Language::English);
    let expected = size(&reference["hud_size"]);
    assert_eq!(layout.size, (expected.0 as f32, expected.1 as f32));
    for (name, actual) in [
        ("width", layout.fields[0]),
        ("height", layout.fields[1]),
        ("times", layout.times),
        ("unit", layout.unit),
        ("separator", layout.separator),
        ("lock", layout.lock),
        ("ratio", layout.ratio),
    ] {
        assert_eq!(actual, window_rect(&reference["hud_rects"][name]), "{name}");
    }
}

/// 【选区尺寸测试】【明暗配色】逐通道对照原版十一种面板与字段颜色，不用截图近似代替颜色数值
#[test]
fn hud_colors_match_both_production_palettes() {
    let reference = reference();
    for (name, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
        let palette = Colors::new(mode);
        for (key, Rgba(r, g, b, a)) in [
            ("text", palette.text),
            ("muted", palette.muted),
            ("hover", palette.hover),
            ("fill", palette.fill),
            ("border", palette.border),
            ("separator", palette.separator),
            ("accent", palette.accent),
            ("editor_text", palette.editor_text),
            ("editor_fill", palette.editor_fill),
            ("editor_border", palette.editor_border),
            ("editor_selection", palette.editor_selection),
        ] {
            assert_eq!(
                serde_json::json!([r, g, b, a]),
                reference["palettes"][name][key],
                "{name}: {key}"
            );
        }
    }
}
