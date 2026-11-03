//! 【界面字库】【按需加载】独立窗口共用一次性系统字库加载状态

use cosmic_text::FontSystem;
use std::ops::{Deref, DerefMut};

pub struct UiFonts {
    system: FontSystem,
    loaded: bool,
}

impl UiFonts {
    /// 【界面字库】【轻量创建】创建只含内置字体的字库；不扫描系统目录，不创建线程
    pub fn new() -> Self {
        Self {
            system: crate::fonts::bundled(),
            loaded: false,
        }
    }

    /// 【界面字库】【定向家族】创建 family 对应的轻量字库，只匹配常规和半粗体文件，返回字体状态，不扫描系统目录
    pub fn with_system_family(family: &str) -> Self {
        let mut value = Self::new();
        let resolved = crate::fonts::text_faces::family(&mut value.system, family, false, false);
        crate::fonts::text_faces::family(&mut value.system, family, true, false);
        value.system.db_mut().set_sans_serif_family(resolved);
        value
    }

    /// 【界面字库】【Unicode 补充】text 非 ASCII 时最多加载一次系统字库；同一窗口的全部字段共享结果
    pub fn ensure_text(&mut self, text: &str) {
        if !self.loaded && !text.is_ascii() {
            crate::fonts::add_system(&mut self.system, crate::fonts::system());
            self.loaded = true;
        }
    }
}

impl Default for UiFonts {
    /// 【界面字库】【默认创建】返回轻量字库状态，不执行系统字体发现
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for UiFonts {
    type Target = FontSystem;
    /// 【界面字库】【只读访问】返回当前字体系统引用，供共享排版组件使用
    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl DerefMut for UiFonts {
    /// 【界面字库】【排版访问】返回可变字体系统引用，加载标志仍由当前状态维护
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.system
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【界面字库测试】【重复请求】ASCII 不加载外部字体，Unicode 字库只补充一次且保留内置字形
    #[test]
    fn unicode_loading_is_shared_and_one_shot() {
        let mut fonts = UiFonts::new();
        fonts.ensure_text("plain");
        assert!(!fonts.loaded);
        fonts.ensure_text("会议");
        let count = fonts.db().len();
        fonts.ensure_text("日本語");
        assert!(fonts.loaded);
        assert_eq!(fonts.db().len(), count);
        assert!(count >= 5);
    }
}
