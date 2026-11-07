//! 【图片翻译】【原文分段】保留 OCR 行号首次顺序、列间断开与段落对齐规则，不把两栏合成一句

use super::{BoxRect, SourceBlock, Token};
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

struct TextLine {
    text: String,
    rect: BoxRect,
}

/// 【图片翻译】【中日韩判断】按 character 返回是否属于原版汉字、假名或韩文范围
fn cjk(character: char) -> bool {
    matches!(character as u32,0x3400..=0x4dbf|0x4e00..=0x9fff|0xf900..=0xfaff|0x20000..=0x2a6df|0x2a700..=0x2b73f|0x2b740..=0x2b81f|0x2b820..=0x2ceaf|0x3040..=0x30ff|0xac00..=0xd7af)
}

/// 【图片翻译】【宽字符判断】按 character 返回段落连接时是否不应插入空格，保留原版代理对与全角标点规则
fn wide(character: char) -> bool {
    cjk(character) || matches!(character as u32,0x3000..=0x303f|0xff00..=0xffef|0x10000..=0x10ffff)
}

/// 【图片翻译】【段落连接】按 head、tail 连接一行，恢复行末连字符断词，中日韩边界不加空格
pub fn join_line(head: &str, tail: &str) -> String {
    let (head, tail) = (head.trim(), tail.trim());
    let (Some(last), Some(first)) = (head.chars().last(), tail.chars().next()) else {
        return format!("{head}{tail}");
    };
    let mut characters = head.chars().rev();
    characters.next();
    if last == '-' && characters.next().is_some_and(char::is_alphabetic) && first.is_lowercase() {
        return format!("{}{tail}", &head[..head.len() - 1]);
    }
    if wide(last) || wide(first) {
        format!("{head}{tail}")
    } else {
        format!("{head} {tail}")
    }
}

/// 【图片翻译】【列表判断】按 text 识别原版项目符号和一至三位编号，编号小数后不误切段
fn list_item(text: &str) -> bool {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(r"^(?:[•·●○■□◆▪‣⁃*\-–—]\s|\d{1,3}[.)、]|[(（]\d{1,3}[)）])").unwrap()
    });
    let Some(found) = pattern.find(text) else {
        return false;
    };
    let decimal = DECIMAL.get_or_init(|| Regex::new(r"^\d").unwrap());
    !decimal.is_match(text) || !decimal.is_match(&text[found.end()..])
}

/// 【图片翻译】【词间空白】按 previous、current 返回原版 OCR token 拼接是否插入空格，标点和相邻中文优先直接连接
fn insert_space(previous: &Token, current: &Token) -> bool {
    let first = current.text.chars().next();
    if first.is_some_and(|ch| ",.!?:;)]}、。》】！），：；？".contains(ch)) {
        return false;
    }
    let previous_cjk = previous.text.chars().any(cjk);
    let current_cjk = current.text.chars().any(cjk);
    if previous_cjk && current_cjk
        || current_cjk
            && previous
                .text
                .chars()
                .last()
                .is_some_and(|ch| "、。》】！），：；？".contains(ch))
    {
        return false;
    }
    current.rect.left() - previous.rect.right()
        > 3.0_f64.max(previous.rect.height().min(current.rect.height()) * 0.28)
}

/// 【图片翻译】【原文行】按 tokens 拼接 OCR 行，行内宽空白拆开列，返回按顶边稳定排序的行
fn lines(tokens: &[Token]) -> Vec<TextLine> {
    let mut rows: Vec<Vec<&Token>> = Vec::new();
    let mut indices = HashMap::new();
    for token in tokens
        .iter()
        .filter(|token| token.rect.valid() && !token.text.trim().is_empty())
    {
        let index = *indices.entry(token.line).or_insert_with(|| {
            rows.push(Vec::new());
            rows.len() - 1
        });
        rows[index].push(token);
    }
    let mut result = Vec::new();
    for mut row in rows {
        row.sort_by(|left, right| {
            left.index
                .cmp(&right.index)
                .then_with(|| left.rect.left().total_cmp(&right.rect.left()))
        });
        let mut current = TextLine {
            text: String::new(),
            rect: BoxRect::default(),
        };
        let mut previous: Option<&Token> = None;
        for token in row {
            if let Some(old) = previous {
                let height = old.rect.height().min(token.rect.height()).max(1.0);
                if token.rect.left() - old.rect.right() > height * 2.2 {
                    result.push(current);
                    current = TextLine {
                        text: String::new(),
                        rect: BoxRect::default(),
                    };
                    previous = None;
                }
            }
            if previous.is_some_and(|old| insert_space(old, token)) {
                current.text.push(' ');
            }
            current.text.push_str(token.text.trim());
            current.rect = current.rect.union(token.rect);
            previous = Some(token);
        }
        if !current.text.is_empty() {
            result.push(current);
        }
    }
    result.sort_by(|left, right| left.rect.top().total_cmp(&right.rect.top()));
    result
}

/// 【图片翻译】【续段判断】按 block、last_text、line 返回可连接间距，字号、间隙、对齐和句末不满足时返回空值
fn gap(block: &SourceBlock, last_text: &str, line: &TextLine) -> Option<f64> {
    let last = *block.line_boxes.last()?;
    let height = last.height().max(1.0);
    let next_height = line.rect.height().max(1.0);
    let unit = height.min(next_height);
    let gap = line.rect.top() - last.bottom();
    if height.max(next_height) / unit > 1.4 || gap > unit * 0.8 || gap < -unit * 0.35 {
        return None;
    }
    if line.rect.left() > block.rect.right() || line.rect.right() < block.rect.left() {
        return None;
    }
    let aligned = (line.rect.left() - block.rect.left()).abs() <= unit
        || block.line_boxes.len() == 1
            && line.rect.left() < last.left()
            && last.left() - line.rect.left() <= unit * 3.0
        || (line.rect.left() + line.rect.width() / 2.0 - last.left() - last.width() / 2.0).abs()
            <= unit;
    if !aligned {
        return None;
    }
    let right = block.rect.right().max(line.rect.right());
    let width = right - block.rect.left().min(line.rect.left());
    let full = last.width() >= unit * 8.0 && right - last.right() <= (unit * 4.0).max(width * 0.3);
    if !full
        || last_text
            .chars()
            .last()
            .is_some_and(|ch| ".!?:;。！？：；".contains(ch))
        || list_item(&line.text)
    {
        return None;
    }
    Some(gap)
}

/// 【图片翻译】【段落合并】按 tokens 返回原版阅读顺序段落，行高取上中位数、行距取首末顶边的平均差
pub fn group(tokens: &[Token]) -> Vec<SourceBlock> {
    let mut blocks: Vec<SourceBlock> = Vec::new();
    let mut last_texts: Vec<String> = Vec::new();
    for line in lines(tokens) {
        let target = blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                gap(block, &last_texts[index], &line).map(|gap| (index, gap.abs()))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|pair| pair.0);
        if let Some(index) = target {
            let block = &mut blocks[index];
            block.text = join_line(&block.text, &line.text);
            block.rect = block.rect.union(line.rect);
            block.line_boxes.push(line.rect);
            last_texts[index] = line.text;
        } else {
            blocks.push(SourceBlock {
                text: line.text.clone(),
                rect: line.rect,
                line_boxes: vec![line.rect],
                ..Default::default()
            });
            last_texts.push(line.text);
        }
    }
    for block in &mut blocks {
        let mut heights = block
            .line_boxes
            .iter()
            .map(|rect| rect.height())
            .collect::<Vec<_>>();
        heights.sort_by(f64::total_cmp);
        block.line_height = heights[heights.len() / 2];
        let count = block.line_boxes.len();
        block.line_pitch = if count > 1 {
            (block.line_boxes[count - 1].top() - block.line_boxes[0].top()) / (count - 1) as f64
        } else {
            0.0
        };
    }
    blocks
}
