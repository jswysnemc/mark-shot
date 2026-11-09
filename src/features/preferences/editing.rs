//! 【设置窗口】【编辑事务】切换焦点前提交输入，失败保留原字段与错误信息
use super::{catalog::ENTRIES, window::Preferences};
use serde_json::Value;

impl Preferences {
    /// 【设置窗口】【资源摘要】无参数，读取本地模型与运行库状态供 OCR 页面展示，不联网
    pub(super) fn refresh_resources(&mut self) {
        if self.values["ocr"]["engine"] == "system" {
            self.resource_note = if self.chinese() {
                "使用 Windows 已安装的识别语言"
            } else {
                "Uses installed Windows recognition languages"
            }
            .into();
            return;
        }
        let models = crate::ocr::models::OcrModels::load();
        let runtime = models.library.status == crate::ocr::models::ModelStatus::Installed;
        let language = models
            .active()
            .map(|index| crate::ocr::models::MODELS[index].label(self.chinese()));
        self.resource_note = if self.chinese() {
            format!(
                "推理库{} · {}",
                if runtime { "已就绪" } else { "待安装" },
                language.unwrap_or("尚未安装识别语言")
            )
        } else {
            format!(
                "Runtime {} · {}",
                if runtime { "ready" } else { "missing" },
                language.unwrap_or("No language installed")
            )
        };
    }

    /// 【设置窗口】【数值微调】按 index 和 direction 调整一个合法步长并保存，边界处保持原值
    pub(super) fn step(&mut self, index: usize, direction: i8) {
        let entry = &ENTRIES[index];
        let Some(step) = entry.step() else {
            return;
        };
        let Ok(current) = self.value(index).parse::<f64>() else {
            return;
        };
        let (minimum, maximum) = entry.range().unwrap_or((0.0, i32::MAX as f64));
        let next = ((current + step * f64::from(direction)) * 100.0).round() / 100.0;
        let next = next.clamp(minimum, maximum);
        if next == current {
            return;
        }
        let value = if entry.default_value().is_i64() || entry.default_value().is_u64() {
            Value::from(next as i64)
        } else {
            Value::from(next)
        };
        self.focus = Some(index);
        self.save(index, value);
    }

    /// 【设置窗口】【运行发布】无参数，正式窗口发布配置；单元测试仅验证自己的文件，返回结果
    pub(super) fn publish(&self) -> std::io::Result<()> {
        if cfg!(test) {
            Ok(())
        } else {
            crate::config::preferences::reload(&self.path)
        }
    }

    /// 【设置窗口】【输入确认】无参数，将当前输入按字段类型提交，成功结束编辑，失败保留草稿
    pub(super) fn apply(&mut self) -> bool {
        let Some((index, field)) = &self.editing else {
            return true;
        };
        let index = *index;
        if field.text() == ENTRIES[index].display(&self.values) {
            self.editing = None;
            self.shortcut_pause = None;
            return true;
        }
        if matches!(ENTRIES[index].kind, super::catalog::Kind::Shortcut) {
            self.editing = None;
            self.shortcut_pause = None;
            return true;
        }
        match ENTRIES[index].parse(&field.text()) {
            Ok(value) => {
                if self.save(index, value) {
                    self.editing = None;
                    self.shortcut_pause = None;
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                self.status = error.to_string();
                false
            }
        }
    }

    /// 【设置窗口】【统一失焦】无参数，提交通用字段和翻译草稿，返回是否允许导航或关闭
    pub(super) fn commit_pending(&mut self) -> bool {
        self.apply() && super::translation::input::apply(self)
    }

    /// 【设置窗口】【多重绑定】按 index、input 追加快捷键，已有绑定保留，返回空值
    pub(super) fn record_shortcut(
        &mut self,
        index: usize,
        input: &crate::backend::window::KeyInput,
    ) {
        let Some(chord) = super::shortcuts::chord(input) else {
            return;
        };
        if ENTRIES[index].section == "desktop" {
            if self.save(index, chord.into()) {
                self.editing = None;
                self.shortcut_pause = None;
            }
            return;
        }
        let mut bindings: Vec<_> = super::shortcuts::text(&self.values, ENTRIES[index].key)
            .split(" / ")
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
            .collect();
        if !bindings.contains(&chord) {
            bindings.push(chord);
        }
        let value = serde_json::json!(bindings);
        if self.save(index, value) {
            self.editing = None;
            self.shortcut_pause = None;
        }
    }
}
