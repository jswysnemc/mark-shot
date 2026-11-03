//! 【文字属性栏】【按下状态】记录控件、显示器、属性来源和滑条抓取点，不引用应用业务模块

use super::layout::Control;
use crate::features::text::size_input::SizeInput;
use crate::ui::settings_panel::SettingsSource;
use std::time::Instant;

#[derive(Default)]
pub struct WheelInputs {
    pub size: SizeInput,
    pub opacity: SizeInput,
    pub spin: SizeInput,
    pub list: SizeInput,
    pub source: Option<SettingsSource>,
}

impl WheelInputs {
    /// 【文字属性栏】【滚轮目标】按 source 更新目标，目标改变时清除控件输入余量；无返回值，画布余量独立维护
    pub fn prepare(&mut self, source: Option<SettingsSource>) {
        if self.source != source {
            self.size.reset();
            self.opacity.reset();
            self.spin.reset();
            self.source = source;
        }
    }
}

pub struct Press {
    pub control: Control,
    pub monitor: usize,
    pub source: Option<SettingsSource>,
    pub drag_offset: Option<f32>,
    pub page: i32,
    pub next: Instant,
}
