//! 【翻译服务】【语言规范】区分界面选项与各服务语言代码，未知语言不做近似替换

use super::{Error, ErrorKind, Provider, Result};

pub const OPTIONS: [&str; 9] = [
    "Simplified Chinese",
    "Traditional Chinese",
    "English",
    "Japanese",
    "Korean",
    "French",
    "German",
    "Spanish",
    "Russian",
];

pub use crate::ui::language::Language as UiLanguage;

const CHINESE_LABELS: [&str; 9] = [
    "简体中文",
    "繁体中文",
    "英语",
    "日语",
    "韩语",
    "法语",
    "德语",
    "西班牙语",
    "俄语",
];

/// 【翻译服务】【配置语言】按 text 返回英文界面的稳定选项名称，未知输入只去除首尾空白，不猜测配置来源的界面语言
pub fn value(text: &str) -> String {
    value_for(text, UiLanguage::English)
}

/// 【翻译服务】【选项标签】按 language 返回九个原版展示标签，不修改传给服务的稳定名称
pub fn labels(language: UiLanguage) -> &'static [&'static str; 9] {
    match language {
        UiLanguage::English => &OPTIONS,
        UiLanguage::Chinese => &CHINESE_LABELS,
    }
}

/// 【翻译服务】【目标展示】按 text、language 返回预设目标的原版界面标签，任意自由目标保持原样，不改变服务参数
pub fn display_for(text: &str, language: UiLanguage) -> &str {
    OPTIONS
        .iter()
        .position(|name| text.eq_ignore_ascii_case(name))
        .map_or(text, |index| labels(language)[index])
}

/// 【翻译服务】【界面语言】按 text 和 language 返回原版选项的稳定名称，仅识别当前界面标签，未知语言保留自由输入
pub fn value_for(text: &str, language: UiLanguage) -> String {
    let trimmed = text.trim();
    let labels = labels(language);
    OPTIONS
        .iter()
        .enumerate()
        .find_map(|(index, name)| {
            (trimmed.eq_ignore_ascii_case(name) || trimmed == labels[index])
                .then(|| (*name).to_owned())
        })
        .unwrap_or_else(|| trimmed.to_owned())
}

/// 【翻译服务】【规范语言】按 text 返回公共规范键，空输入默认简体中文，未知语言返回空值
pub(crate) fn canonical(text: &str) -> Option<&'static str> {
    let key: String = text
        .chars()
        .filter(|ch| !ch.is_whitespace() && !matches!(ch, '_' | '-'))
        .flat_map(char::to_lowercase)
        .collect();
    Some(match key.as_str() {
        "" | "zhhans" | "zhcn" | "zhsg" | "zh" | "chs" | "simplifiedchinese"
        | "chinesesimplified" | "chinese" | "简体中文" | "中文" | "汉语" => "zh-Hans",
        "zhhant" | "zhtw" | "zhhk" | "cht" | "traditionalchinese" | "chinesetraditional"
        | "繁体中文" | "繁體中文" => "zh-Hant",
        "yue" | "cantonese" | "粤语" => "yue",
        "en" | "enus" | "engb" | "eng" | "english" | "英语" | "英文" => "en",
        "ja" | "jp" | "jpn" | "japanese" | "日语" | "日文" | "日本語" => "ja",
        "ko" | "kor" | "korean" | "韩语" | "韩文" => "ko",
        "fr" | "fra" | "french" | "法语" => "fr",
        "de" | "deu" | "ger" | "german" | "德语" => "de",
        "ru" | "rus" | "russian" | "俄语" => "ru",
        "es" | "spa" | "spanish" | "西班牙语" => "es",
        "pt" | "por" | "portuguese" | "葡萄牙语" => "pt",
        "it" | "ita" | "italian" | "意大利语" => "it",
        "ar" | "ara" | "arabic" | "阿拉伯语" => "ar",
        "hi" | "hin" | "hindi" | "印地语" => "hi",
        "th" | "tha" | "thai" | "泰语" => "th",
        "vi" | "vie" | "vietnamese" | "越南语" => "vi",
        "id" | "ind" | "indonesian" | "印尼语" => "id",
        "ms" | "may" | "malay" | "马来语" => "ms",
        "tr" | "tur" | "turkish" | "土耳其语" => "tr",
        "nl" | "dut" | "dutch" | "荷兰语" => "nl",
        "pl" | "pol" | "polish" | "波兰语" => "pl",
        _ => return None,
    })
}

/// 【翻译服务】【厂商代码】按 provider 和 text 返回精确语言代码，未知或不支持时返回错误，不自动切换服务
pub(crate) fn code(provider: Provider, text: &str) -> Result<&'static str> {
    let key = canonical(text).ok_or_else(|| {
        Error::new(
            ErrorKind::Configuration,
            format!("unrecognized target language: {}", text.trim()),
        )
    })?;
    let code = match (provider, key) {
        (Provider::Tencent, "zh-Hans") | (Provider::Baidu, "zh-Hans") => "zh",
        (Provider::Tencent, "zh-Hant") => "zh-TW",
        (Provider::Baidu, "zh-Hant") => "cht",
        (Provider::Baidu, "ja") => "jp",
        (Provider::Baidu, "ko") => "kor",
        (Provider::Baidu, "fr") => "fra",
        (Provider::Baidu, "es") => "spa",
        (Provider::Baidu, "ar") => "ara",
        (Provider::Baidu, "vi") => "vie",
        (Provider::Baidu, "ms") => "may",
        (Provider::Youdao, "zh-Hans") => "zh-CHS",
        (Provider::Youdao, "zh-Hant") => "zh-CHT",
        (Provider::Tencent, "yue" | "nl" | "pl") => "",
        (_, key) => key,
    };
    if code.is_empty() {
        return Err(Error::new(
            ErrorKind::Configuration,
            format!("{} does not support target language: {text}", provider.id()),
        ));
    }
    Ok(code)
}
