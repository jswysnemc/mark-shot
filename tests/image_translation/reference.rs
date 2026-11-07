//! 【图片翻译测试】【原版结果】固定 Qt 6 生产函数输出，差异报告保留完整矩形和显示行

use mark_shot::features::image_translation::{
    BoxRect, Token, TranslatedBlock, colors, exchange, layout, model, paragraphs,
};
use serde_json::{Value, json};
use tiny_skia::{Color, Pixmap};

/// 【图片翻译测试】【颜色编码】按 color 返回与原版参考一致的四通道数组，不使用配置颜色字符串
fn rgba(color: mark_shot::theme::Rgba) -> Value {
    json!([color.0, color.1, color.2, color.3])
}

/// 【图片翻译测试】【数字语义】按 value 统一 JSON 整数与浮点表示，返回语义相同的值，不允许数值偏差
fn numeric(value: Value) -> Value {
    match value {
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(array) => Value::Array(array.into_iter().map(numeric).collect()),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, numeric(value)))
                .collect(),
        ),
        other => other,
    }
}

/// 【图片翻译测试】【排版编码】按 blocks 返回原版比较格式，保留所有矩形与基线，仅转换颜色表示
fn translated(blocks: &[TranslatedBlock]) -> Value {
    let mut result = serde_json::to_value(blocks).unwrap();
    for (value, block) in result.as_array_mut().unwrap().iter_mut().zip(blocks) {
        value["background"] = rgba(block.background);
        value["foreground"] = rgba(block.foreground);
    }
    result
}

/// 【图片翻译测试】【固定输入】返回独立参考输入，无参数，不读取用户图像或配置
fn cases() -> Value {
    serde_json::from_str(include_str!("../fixtures/qt-image-translation/cases.json")).unwrap()
}

/// 【图片翻译测试】【固定输出】返回真实原版结果，无参数，不在验证时重建期望值
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../fixtures/qt-image-translation/reference.json"
    ))
    .unwrap()
}

/// 【图片翻译测试】【识别输入】按 value 保留 token 的原始顺序，返回独立识别结果
fn tokens(value: &Value) -> Vec<Token> {
    serde_json::from_value(value["tokens"].clone()).unwrap()
}

/// 【图片翻译测试】【矩形输入】按 value 返回四元原图矩形，非法测试输入直接失败
fn rect(value: &Value) -> BoxRect {
    serde_json::from_value(value.clone()).unwrap()
}

/// 【图片翻译测试】【颜色输入】按 value 返回带透明度测试颜色，无省略透明度的默认推断
fn color(value: &Value) -> Color {
    Color::from_rgba8(
        value[0].as_u64().unwrap() as u8,
        value[1].as_u64().unwrap() as u8,
        value[2].as_u64().unwrap() as u8,
        value[3].as_u64().unwrap() as u8,
    )
}

/// 【图片翻译测试】【原图重建】按 value 返回与 Qt 整数矩形填色对应的预乘图像，不加载外部图片
fn image(value: &Value) -> Pixmap {
    let mut image = Pixmap::new(
        value["width"].as_u64().unwrap() as u32,
        value["height"].as_u64().unwrap() as u32,
    )
    .unwrap();
    image.fill(color(&value["background"]));
    for mark in value["marks"].as_array().unwrap() {
        let rect = rect(&mark["rect"]);
        let pixel = color(&mark["color"]).premultiply().to_color_u8();
        for y in rect.top() as u32..rect.bottom() as u32 {
            for x in rect.left() as u32..rect.right() as u32 {
                let index = (y * image.width() + x) as usize;
                image.pixels_mut()[index] = pixel;
            }
        }
    }
    image
}

/// 【图片翻译测试】【分段对照】全部空白、列间隔、字号、句末、编号和标点边界与原版逐项一致
#[test]
fn original_paragraph_and_join_contracts() {
    let (cases, reference) = (cases(), reference());
    for (case, expected) in cases["groups"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["groups"].as_array().unwrap())
    {
        let mut mismatches = Vec::new();
        differences(
            &serde_json::to_value(paragraphs::group(&tokens(case))).unwrap(),
            &expected["blocks"],
            "blocks",
            1e-9,
            &mut mismatches,
        );
        assert!(
            mismatches.is_empty(),
            "{}: {}",
            case["name"],
            mismatches.join("\n")
        );
    }
    for (case, expected) in cases["joins"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["joins"].as_array().unwrap())
    {
        assert_eq!(
            paragraphs::join_line(case[0].as_str().unwrap(), case[1].as_str().unwrap()),
            expected.as_str().unwrap(),
            "{case}"
        );
    }
}

/// 【图片翻译测试】【取色对照】亮暗、彩色、低对比、透明图、边缘及大步长与原版四通道颜色一致
#[test]
fn original_image_color_sampling_contracts() {
    let (cases, reference) = (cases(), reference());
    for (case, expected) in cases["colors"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["colors"].as_array().unwrap())
    {
        let actual = colors::sample(
            &image(case),
            rect(&case["rect"]),
            case["line_height"].as_f64().unwrap(),
        );
        assert_eq!(
            rgba(actual.background),
            expected["background"],
            "{} background",
            case["name"]
        );
        assert_eq!(
            rgba(actual.foreground),
            expected["foreground"],
            "{} foreground",
            case["name"]
        );
    }
}

/// 【图片翻译测试】【交换对照】行身份、面积回退、多边形、负大小规范化、排序及边界裁切与原版一致
#[test]
fn original_coordinate_exchange_contracts() {
    let (cases, reference) = (cases(), reference());
    for (case, expected) in cases["exchanges"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["exchanges"].as_array().unwrap())
    {
        let blocks = paragraphs::group(&tokens(case));
        assert_eq!(
            numeric(exchange::input(&blocks, "English")),
            numeric(expected["input"].clone()),
            "{} input",
            case["name"]
        );
        let parsed = exchange::tokens(&case["output"], BoxRect::new(0.0, 0.0, 400.0, 200.0));
        assert_eq!(
            serde_json::to_value(exchange::texts(&parsed, &blocks)).unwrap(),
            expected["texts"],
            "{} output",
            case["name"]
        );
    }
}

/// 【图片翻译测试】【差异记录】按 actual、expected、path 返回全部差异，数值允许六十四分之一像素量化舍入，不改变期望值
fn differences(
    actual: &Value,
    expected: &Value,
    path: &str,
    tolerance: f64,
    result: &mut Vec<String>,
) {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right))
            if (left.as_f64().unwrap() - right.as_f64().unwrap()).abs() <= tolerance => {}
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                differences(left, right, &format!("{path}/{index}"), tolerance, result);
            }
        }
        (Value::Object(left), Value::Object(right)) if left.len() == right.len() => {
            for (key, right) in right {
                differences(
                    left.get(key).unwrap_or(&Value::Null),
                    right,
                    &format!("{path}/{key}"),
                    tolerance,
                    result,
                );
            }
        }
        _ if actual == expected => {}
        _ => result.push(format!("{path}: actual={actual}, expected={expected}")),
    }
}

/// 【图片翻译测试】【排版对照】字号、换行、避让、省略、全文、行框及基线与真实 Qt 样本对照，失败时保存完整差异
#[test]
fn original_layout_and_seed_coordinates() {
    let (cases, reference) = (cases(), reference());
    let mut fonts = mark_shot::fonts::bundled();
    let mut actual = Vec::new();
    for case in cases["layouts"].as_array().unwrap() {
        let texts = serde_json::from_value::<Vec<String>>(case["texts"].clone()).unwrap();
        let blocks = layout::translated(
            &image(case),
            &paragraphs::group(&tokens(case)),
            &texts,
            &mut fonts,
        );
        let shifted =
            model::offset_blocks(&blocks, (-15.0, 7.0), BoxRect::new(0.0, 0.0, 100.0, 100.0));
        actual.push(json!({"name":case["name"],"text":model::text(&blocks),"blocks":translated(&blocks),"shifted":translated(&shifted)}));
    }
    let actual = Value::Array(actual);
    let mut mismatches = Vec::new();
    differences(
        &actual,
        &reference["layouts"],
        "layouts",
        1e-4,
        &mut mismatches,
    );
    if !mismatches.is_empty() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/image-translation-layout-diff.json");
        std::fs::write(
            &root,
            serde_json::to_vec_pretty(
                &json!({"actual":actual,"expected":reference["layouts"],"differences":mismatches}),
            )
            .unwrap(),
        )
        .unwrap();
        panic!(
            "{} layout differences; complete report: {}\n{}",
            mismatches.len(),
            root.display(),
            mismatches
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
