//! 【箭头】【配置兼容】新配置保存组合属性，旧字符串和数字仍可读取

use super::{ArrowHead, ArrowStyle, ArrowTail};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Deserialize)]
#[serde(untagged)]
enum Representation {
    Name(String),
    Number(u8),
    Attributes {
        #[serde(default)]
        head: ArrowHead,
        #[serde(default)]
        tail: ArrowTail,
        #[serde(default)]
        bidirectional: bool,
    },
}

impl<'de> Deserialize<'de> for ArrowStyle {
    /// 【箭头】【属性读取】按 deserializer 读取组合属性或旧值，返回完整属性，未知旧值返回格式错误
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Representation::deserialize(deserializer)? {
            Representation::Name(value) => {
                Self::parse(&value).ok_or_else(|| D::Error::custom("unknown arrow style"))
            }
            Representation::Number(value) => Self::parse(&value.to_string())
                .ok_or_else(|| D::Error::custom("unknown arrow style index")),
            Representation::Attributes {
                head,
                tail,
                bidirectional,
            } => Ok(Self {
                head,
                tail,
                bidirectional,
            }),
        }
    }
}

/// 【箭头测试】【旧值兼容】无参数，六个旧值保持头部和方向，新结构独立保存尾部与双向，无返回值
#[test]
fn legacy_values_and_independent_attributes_round_trip() {
    for (index, name) in [
        "fletched",
        "kde",
        "bidirectional_fletched",
        "bidirectional_kde",
        "direct",
        "bidirectional_direct",
    ]
    .into_iter()
    .enumerate()
    {
        let old: ArrowStyle = serde_json::from_str(&format!("\"{name}\"")).unwrap();
        assert_eq!(
            old,
            serde_json::from_str::<ArrowStyle>(&index.to_string()).unwrap()
        );
        let changed = ArrowStyle {
            tail: ArrowTail::Pointed,
            ..old
        };
        assert_eq!(
            changed,
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap()
        );
        assert_eq!(
            changed.with_head(ArrowStyle::Direct).bidirectional,
            old.bidirectional
        );
        assert_eq!(
            changed.with_head(ArrowStyle::Direct).tail,
            ArrowTail::Pointed
        );
    }
    assert_eq!(ArrowStyle::ALL.len(), 3);
}
