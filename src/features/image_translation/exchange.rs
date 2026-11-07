//! 【图片翻译】【请求交换】段落带实际原图坐标，译文优先按行身份映射，失配时按最大相交面积恢复

use super::{BoxRect, SourceBlock, Token, paragraphs};
use serde_json::{Value, json};

/// 【图片翻译】【请求正文】按 blocks、target 返回原版段落 token JSON，不使用虚拟行框
pub fn input(blocks: &[SourceBlock], target: &str) -> Value {
    json!({"targetLanguage":target,"tokens":blocks.iter().enumerate().map(|(index,block)| json!({"text":block.text,"box":block.rect.0,"line":index,"index":0,"confidence":1.0})).collect::<Vec<_>>()})
}

/// 【图片翻译】【相交面积】按 token、block 返回原图坐标相交面积，仅相接边缘返回零
fn area(token: &Token, block: &SourceBlock) -> f64 {
    let overlap = token.rect.intersect(block.rect);
    if overlap.valid() {
        overlap.width() * overlap.height()
    } else {
        0.0
    }
}

/// 【图片翻译】【段落匹配】按 token 和 blocks 返回行号对应的相交段落，行号被改写时采用最大相交面积，未相交返回空值
fn matching(token: &Token, blocks: &[SourceBlock]) -> Option<usize> {
    if let Ok(index) = usize::try_from(token.line)
        && blocks
            .get(index)
            .is_some_and(|block| area(token, block) > 0.0)
    {
        return Some(index);
    }
    let mut best = None;
    let mut maximum = 0.0;
    for (index, block) in blocks.iter().enumerate() {
        let value = area(token, block);
        if value > maximum {
            maximum = value;
            best = Some(index);
        }
    }
    best
}

/// 【图片翻译】【译文恢复】按 tokens 和 blocks 返回完整段落译文，保留原段落数量，失配或空输出不伪造成功
pub fn texts(tokens: &[Token], blocks: &[SourceBlock]) -> Vec<String> {
    let mut result = vec![String::new(); blocks.len()];
    for token in tokens {
        if let Some(index) = matching(token, blocks) {
            result[index] = paragraphs::join_line(&result[index], &token.text);
        }
    }
    result
}

/// 【图片翻译】【响应矩形】按 value 读取四元矩形或多边形外接框，返回未规范化范围，非法坐标返回空值
fn rect(value: &Value) -> Option<BoxRect> {
    let values = value.as_array()?;
    if values.len() == 4 && values.iter().all(Value::is_number) {
        return Some(BoxRect::new(
            values[0].as_f64()?,
            values[1].as_f64()?,
            values[2].as_f64()?,
            values[3].as_f64()?,
        ));
    }
    if values.len() < 2 || !values[0].is_array() {
        return None;
    }
    let mut points = values.iter().filter_map(|value| {
        let point = value.as_array()?;
        Some((point.first()?.as_f64()?, point.get(1)?.as_f64()?))
    });
    let (x, y) = points.next()?;
    let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
    for (x, y) in points {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    Some(BoxRect::new(left, top, right - left, bottom - top))
}

/// 【图片翻译】【字段矩形】按 token 优先采用 box、bbox、points，其次读取坐标字段，返回原版矩形解析结果
fn token_rect(token: &Value) -> Option<BoxRect> {
    let object = token.as_object()?;
    for name in ["box", "bbox", "points"] {
        if let Some(value) = object.get(name) {
            return rect(value);
        }
    }
    for (x, y) in [("x", "y"), ("left", "top")] {
        if object.contains_key(x) && object.contains_key(y) {
            return Some(BoxRect::new(
                object[x].as_f64().unwrap_or(0.0),
                object[y].as_f64().unwrap_or(0.0),
                object.get("width").and_then(Value::as_f64).unwrap_or(0.0),
                object.get("height").and_then(Value::as_f64).unwrap_or(0.0),
            ));
        }
    }
    None
}

/// 【图片翻译】【整数身份】按 value 和 fallback 返回原版整数身份，非整数或超出范围时采用缺省值
fn integer(value: Option<&Value>, fallback: i32) -> i32 {
    value
        .and_then(Value::as_f64)
        .filter(|value| {
            value.fract() == 0.0 && *value >= f64::from(i32::MIN) && *value <= f64::from(i32::MAX)
        })
        .map_or(fallback, |value| value as i32)
}

/// 【图片翻译】【响应识别】按 root、bounds 返回规范化裁切和稳定排序后的原版 token，缺少矩形的文字不参与段落匹配
pub fn tokens(root: &Value, bounds: BoxRect) -> Vec<Token> {
    let mut fallback = 0;
    let mut result = root
        .as_array()
        .or_else(|| root.get("tokens").and_then(Value::as_array))
        .into_iter()
        .flatten()
        .filter_map(|token| {
            let text = token.get("text")?.as_str()?.trim();
            if text.is_empty() {
                return None;
            }
            let raw = token_rect(token)?;
            let normalized = BoxRect::new(
                raw.left().min(raw.right()),
                raw.top().min(raw.bottom()),
                raw.width().abs(),
                raw.height().abs(),
            );
            let rect = if bounds.valid() {
                normalized.intersect(bounds)
            } else {
                normalized
            };
            if bounds.valid() && !rect.valid() {
                return None;
            }
            let index = integer(token.get("index"), fallback);
            fallback += 1;
            Some(Token {
                text: text.to_owned(),
                rect,
                line: integer(token.get("line"), 0),
                index,
                confidence: token
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        let close = (left.rect.top() - right.rect.top()).abs() * 1e12
            <= left.rect.top().abs().min(right.rect.top().abs());
        left.line
            .cmp(&right.line)
            .then(left.index.cmp(&right.index))
            .then_with(|| {
                if close {
                    std::cmp::Ordering::Equal
                } else {
                    left.rect.top().total_cmp(&right.rect.top())
                }
            })
            .then_with(|| left.rect.left().total_cmp(&right.rect.left()))
    });
    result
}
