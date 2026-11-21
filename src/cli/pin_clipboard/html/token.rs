//! 【贴图】【HTML词法】解析开始标签、跳过不可见容器并解码字符实体，供富文本解析复用

/// 【贴图】【开始标签】记录标签名、属性与是否自闭合
pub(super) struct Tag {
    pub(super) name: String,
    pub(super) attrs: Vec<(String, String)>,
    pub(super) self_closing: bool,
}

impl Tag {
    /// 【贴图】【属性读取】按 name 返回属性值，不存在返回空值
    pub(super) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// 【贴图】【标签名】按 raw 读取小写标签名，忽略空白与自闭合斜杠
pub(super) fn tag_name(raw: &str) -> String {
    raw.trim()
        .split(|ch: char| ch.is_whitespace() || ch == '/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// 【贴图】【开始标签解析】按 tail 解析标签名与属性，返回标签及消耗的字节数
pub(super) fn parse_start_tag(tail: &str) -> Option<(Tag, usize)> {
    let bytes = tail.as_bytes();
    let mut index = 1;
    let start = index;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'-' || bytes[index] == b':')
    {
        index += 1;
    }
    if index == start {
        return None;
    }
    let name = tail[start..index].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        match bytes[index] {
            b'>' => {
                index += 1;
                break;
            }
            b'/' => {
                if bytes.get(index + 1) == Some(&b'>') {
                    self_closing = true;
                    index += 2;
                    break;
                }
                index += 1;
            }
            _ => {
                let attr_start = index;
                while index < bytes.len()
                    && !matches!(
                        bytes[index],
                        b'=' | b'>' | b'/' | b' ' | b'\t' | b'\r' | b'\n' | b'\x0c'
                    )
                {
                    index += 1;
                }
                let attr_name = tail[attr_start..index].to_ascii_lowercase();
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                let mut value = String::new();
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                        index += 1;
                    }
                    if matches!(bytes.get(index), Some(b'"' | b'\'')) {
                        let quote = bytes[index];
                        index += 1;
                        let value_start = index;
                        while index < bytes.len() && bytes[index] != quote {
                            index += 1;
                        }
                        value = decode_entities(&tail[value_start..index]);
                        if index < bytes.len() {
                            index += 1;
                        }
                    } else {
                        let value_start = index;
                        while index < bytes.len()
                            && !bytes[index].is_ascii_whitespace()
                            && bytes[index] != b'>'
                        {
                            index += 1;
                        }
                        value = decode_entities(&tail[value_start..index]);
                    }
                }
                if !attr_name.is_empty() {
                    attrs.push((attr_name, value));
                }
            }
        }
    }
    Some((
        Tag {
            name,
            attrs,
            self_closing,
        },
        index,
    ))
}

/// 【贴图】【原样内容】按 name 判断标签内容是否应完全跳过，返回是否为不可见容器
pub(super) fn is_raw(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "head" | "title" | "template" | "noscript"
    )
}

/// 【贴图】【跳过原样内容】从 pos 跳过 name 标签直到闭合标签，返回新位置
pub(super) fn skip_raw(html: &str, pos: usize, name: &str) -> usize {
    let lower = html[pos..].to_ascii_lowercase();
    let needle = format!("</{name}");
    let Some(offset) = lower.find(&needle) else {
        return html.len();
    };
    let after = pos + offset;
    html[after..]
        .find('>')
        .map_or(html.len(), |end| after + end + 1)
}

/// 【贴图】【实体解码】按 text 解码命名与数值字符实体，未知实体保留原文
pub(super) fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        output.push_str(&rest[..index]);
        let after = &rest[index + 1..];
        if let Some(semi) = after.find(';')
            && let Some(character) = entity_char(&after[..semi])
        {
            output.push(character);
            rest = &after[semi + 1..];
            continue;
        }
        output.push('&');
        rest = after;
    }
    output.push_str(rest);
    output
}

/// 【贴图】【实体字符】按 entity 解析单个命名或数值实体，返回对应字符
fn entity_char(entity: &str) -> Option<char> {
    let named = match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        "copy" => Some('©'),
        "reg" => Some('®'),
        "trade" => Some('™'),
        "hellip" => Some('…'),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        "bull" => Some('•'),
        "times" => Some('×'),
        "divide" => Some('÷'),
        "middot" => Some('·'),
        "deg" => Some('°'),
        "euro" => Some('€'),
        "pound" => Some('£'),
        "yen" => Some('¥'),
        "cent" => Some('¢'),
        "sect" => Some('§'),
        "para" => Some('¶'),
        "plusmn" => Some('±'),
        "laquo" => Some('«'),
        "raquo" => Some('»'),
        _ => None,
    };
    named.or_else(|| numeric_entity(entity))
}

/// 【贴图】【数值实体】按 entity 解析 #x 或 # 开头的数值实体，返回 Unicode 字符
fn numeric_entity(entity: &str) -> Option<char> {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
    }
    entity
        .strip_prefix('#')
        .and_then(|decimal| decimal.parse().ok())
        .and_then(char::from_u32)
}
