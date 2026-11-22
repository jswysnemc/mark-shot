//! 【界面字库】【按需加载】独立窗口定向匹配系统家族，只补充实际显示内容的缺字

use cosmic_text::FontSystem;
use std::ops::{Deref, DerefMut};

pub struct UiFonts {
    system: FontSystem,
}

impl UiFonts {
    /// 【界面字库】【轻量创建】无参数，返回定向匹配系统家族的字库，不枚举全部字体，不创建线程
    pub fn new() -> Self {
        Self {
            system: crate::fonts::ui_system::new(),
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

    /// 【界面字库】【Unicode 补充】按 text 只加载实际缺失字形，当前窗口复用匹配字体，无返回值
    pub fn ensure_text(&mut self, text: &str) {
        crate::fonts::ui_fallback::ensure_text(&mut self.system, text);
    }
}

impl Default for UiFonts {
    /// 【界面字库】【默认创建】无参数，返回定向加载系统家族的轻量字库
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
    /// 【界面字库】【排版访问】无参数，返回可变字体系统引用，缺字仍按当前内容定向补充
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.system
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【界面字库测试】【重复请求】无参数，相同 Unicode 内容不重复加载字体，内置字形继续保留
    #[test]
    fn unicode_loading_is_shared_and_one_shot() {
        let mut fonts = UiFonts::new();
        fonts.ensure_text("plain");
        fonts.ensure_text("会议");
        let count = fonts.db().len();
        fonts.ensure_text("会议");
        assert_eq!(fonts.db().len(), count);
        assert!(count >= 5);
    }
}
