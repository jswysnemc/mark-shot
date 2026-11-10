//! 【工具栏】【尺寸令牌】绘制、布局和点击区域读取同一份已保存外观

/// 【工具栏】【图标尺寸】无参数，返回经过范围约束的逻辑像素尺寸
pub fn icon() -> f32 {
    crate::config::preferences::current()
        .toolbar
        .icon_size
        .clamp(12, 48) as f32
}

/// 【工具栏】【文字尺寸】无参数，返回属性栏字号，不改变标注文字本身
pub fn font() -> f32 {
    crate::config::preferences::current()
        .toolbar
        .font_size
        .clamp(8, 24) as f32
}

/// 【工具栏】【按钮尺寸】按 secondary 返回能容纳图标的按钮边长，保留现有自定义最小尺寸
pub fn button(secondary: bool) -> f32 {
    let configured = crate::config::get().toolbar.button_size - if secondary { 2.0 } else { 0.0 };
    configured.max(icon() + if secondary { 6.0 } else { 8.0 })
}

/// 【工具栏】【属性比例】无参数，返回字体与图标共同要求的放大比例
pub fn properties_scale() -> f32 {
    (font() / 11.0).max(icon() / 22.0).max(1.0)
}

/// 【工具栏】【属性高度】无参数，返回能够容纳属性控件的逻辑高度
pub fn properties_height() -> f32 {
    crate::theme::size::panel_height() * properties_scale()
}
