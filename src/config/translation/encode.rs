//! 【翻译配置】【序列化边界】持久化与服务解析使用相同对象，缺省超时不会成为厂商覆盖

use super::Translation;
use serde::{Serialize, Serializer};

impl Serialize for Translation {
    /// 【翻译配置】【完整编码】按 serializer 输出稳定界面字段和原始厂商配置，返回编码结果，保留显式全局超时的存在性
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.root().serialize(serializer)
    }
}
