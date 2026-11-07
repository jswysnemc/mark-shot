//! 【选区尺寸绘制对照】【真实缓存】独立调用生产缓存组件，输出原始像素供 Qt 与原生截图核对

use mark_shot::ui::{
    language::Language,
    selection_size::{cache, state::Panel},
};
use std::path::PathBuf;

/// 【选区尺寸绘制对照】【采集入口】从命令参数读取新输出目录，保存一倍和一点五倍生产缓存，返回文件操作结果
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("missing output directory")?);
    std::fs::create_dir(&root)?;
    let mut panel = Panel::new(Language::English);
    panel.sync((100, 80), (1200, 800), 0.0, Language::English);
    for scale in [1.0, 1.5] {
        panel.dirty = true;
        if !cache::panel(&mut panel, scale, false, false) {
            return Err("panel allocation failed".into());
        }
        panel
            .cache
            .as_ref()
            .unwrap()
            .save_png(root.join(format!("light-{scale}.png")))?;
    }
    panel.fields[0].begin();
    for scale in [1.0, 1.5] {
        panel.dirty = true;
        if !cache::panel(&mut panel, scale, false, true) {
            return Err("field allocation failed".into());
        }
        panel
            .cache
            .as_ref()
            .unwrap()
            .save_png(root.join(format!("width-selected-{scale}.png")))?;
    }
    Ok(())
}
