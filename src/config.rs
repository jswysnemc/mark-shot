// ~/.config/mark-shot/config.toml, loaded once at startup

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use log::info;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::backend::OverlayShell;
use crate::ocr::settings::{Device, Mode};
use crate::theme::Rgba;
use crate::types::Outputs;

/// config section: every field sits next to its default
macro_rules! section {
    ($name:ident { $( $(#[$meta:meta])* $field:ident : $ty:ty = $default:expr),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(default)]
        pub struct $name {
            $( $(#[$meta])* pub $field: $ty,)*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }
    };
}

mod component_theme;
mod legacy_theme;
mod values;
pub use values::*;
pub mod capture;
mod sections;
pub use sections::*;
mod bindings;
pub mod desktop;
pub use bindings::*;
mod loading;
pub use loading::*;
mod document;
mod legacy;
mod paths;
pub mod preferences;
pub mod recovery;
pub(crate) mod runtime_overrides;
pub mod settings;
pub mod settings_store;
pub(crate) mod startup;
pub mod toolbar_layout;
pub use startup::Startup;
pub mod history;
mod persistence;
pub use history::History;
pub mod translation;
pub use translation::Translation;
pub mod upload;
pub use upload::Upload;
pub mod ocr_result_window;
pub use ocr_result_window::OcrResultWindow;
pub mod interface_language;
pub mod interface_theme;
pub use interface_language::Ui;
pub mod export;
pub use export::Export;
mod region_detection;
pub use region_detection::Detection;
#[cfg(test)]
#[path = "config/tests/capture_behavior.rs"]
mod capture_behavior_tests;
#[cfg(test)]
#[path = "config/tests/legacy_config.rs"]
mod legacy_config_tests;
#[cfg(test)]
mod tests;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba(r, g, b, 255)
}

// 【界面】【默认主题】紧凑石墨色面板，薄荷绿色用于交互强调
const TEXT: Rgba = rgb(229, 237, 239);
const OVERLAY2: Rgba = rgb(147, 153, 178);
const CRUST: Rgba = rgb(16, 22, 23);
const BLUE: Rgba = rgb(137, 180, 250);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub capture: capture::Capture,
    pub desktop: desktop::Desktop,
    pub ui: Ui,
    pub startup: Startup,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub general: General,
    #[serde(alias = "regionDetection")]
    pub region_detection: Detection,
    #[serde(alias = "windowDetection")]
    pub window_detection: Detection,
    pub save: Save,
    pub export: Export,
    pub pinned: Pinned,
    pub history: History,
    pub notifications: Notifications,
    pub animation: Animation,
    pub theme: Theme,
    #[serde(skip)]
    pub(crate) theme_custom: u16,
    pub toolbar: Toolbar,
    pub toolbar_layout: toolbar_layout::ToolbarLayout,
    pub settings_panel: SettingsPanel,
    pub color_picker: ColorPicker,
    pub model_picker: ModelPicker,
    pub choice_picker: ChoicePicker,
    pub magnifier: Magnifier,
    pub selection: Selection,
    pub annotations: Annotations,
    pub tools: Tools,
    pub toasts: Toasts,
    pub input: Input,
    pub ocr: Ocr,
    #[serde(alias = "ocrResultWindow")]
    pub ocr_result_window: OcrResultWindow,
    pub translation: Translation,
    #[serde(alias = "imageUpload", alias = "uploader", alias = "imageHost")]
    pub upload: Upload,
    pub log: Log,
    pub keys: Keys,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            capture: capture::Capture::default(),
            desktop: desktop::Desktop::default(),
            ui: Ui::default(),
            startup: Startup::default(),
            language: None,
            general: General::default(),
            region_detection: Detection::default(),
            window_detection: Detection::default(),
            save: Save::default(),
            export: Export::default(),
            pinned: Pinned::default(),
            history: History::default(),
            notifications: Notifications::default(),
            animation: Animation::default(),
            theme: Theme::default(),
            theme_custom: 0,
            toolbar: Toolbar::default(),
            toolbar_layout: toolbar_layout::ToolbarLayout::default(),
            settings_panel: SettingsPanel::default(),
            color_picker: ColorPicker::default(),
            model_picker: ModelPicker::default(),
            choice_picker: ChoicePicker::default(),
            magnifier: Magnifier::default(),
            selection: Selection::default(),
            annotations: Annotations::default(),
            tools: Tools::default(),
            toasts: Toasts::default(),
            input: Input::default(),
            ocr: Ocr::default(),
            ocr_result_window: OcrResultWindow::default(),
            translation: Translation::default(),
            upload: Upload::default(),
            log: Log::default(),
            keys: Keys::default(),
        };
        config.resolve();
        config
    }
}

impl Config {
    /// Fills everything unset with its respective role
    fn resolve(&mut self) {
        let t = &self.theme;
        let toolbar = &mut self.toolbar;
        toolbar
            .background
            .or(t.background, crate::theme::Role::Background);
        toolbar
            .icon
            .or(t.foreground, crate::theme::Role::Foreground);
        toolbar
            .icon_hovered
            .or(t.on_accent, crate::theme::Role::OnAccent);
        toolbar
            .icon_selected
            .or(t.on_accent, crate::theme::Role::OnAccent);
        toolbar.icon_to.or(t.accent, crate::theme::Role::Accent);
        toolbar
            .button_hovered
            .or(t.hover, crate::theme::Role::Hover);
        toolbar
            .button_selected
            .or(t.accent, crate::theme::Role::Accent);
        toolbar
            .separator
            .or(t.foreground, crate::theme::Role::Foreground);

        let panel = &mut self.settings_panel;
        panel
            .background
            .or(t.background, crate::theme::Role::Background);
        panel.text.or(t.foreground, crate::theme::Role::Foreground);
        panel.icon.or(t.foreground, crate::theme::Role::Foreground);
        panel.icon_hovered.or(t.hover, crate::theme::Role::Hover);
        panel.icon_active.or(t.accent, crate::theme::Role::Accent);
        panel
            .separator
            .or(t.foreground, crate::theme::Role::Foreground);

        let picker = &mut self.color_picker;
        picker
            .background
            .or(t.background, crate::theme::Role::Background);
        picker.text.or(t.foreground, crate::theme::Role::Foreground);
        picker.label.or(t.muted, crate::theme::Role::Muted);
        picker
            .marker
            .or(t.foreground, crate::theme::Role::Foreground);

        self.model_picker
            .background
            .or(t.background, crate::theme::Role::Background);

        let magnifier = &mut self.magnifier;
        magnifier
            .outline
            .or(t.foreground, crate::theme::Role::Foreground);
        magnifier
            .label_background
            .or(t.background, crate::theme::Role::Background);
        magnifier
            .label_text
            .or(t.foreground, crate::theme::Role::Foreground);

        self.selection
            .border
            .or(t.foreground, crate::theme::Role::Foreground);
        self.annotations
            .handles
            .color
            .or(t.foreground, crate::theme::Role::Foreground);
        self.annotations
            .text
            .selected_text
            .or(t.foreground, crate::theme::Role::Foreground);

        self.toasts
            .background
            .or(t.background, crate::theme::Role::Background);
        self.toasts
            .text
            .or(t.foreground, crate::theme::Role::Foreground);

        let look = &mut self.ocr.look;
        look.badge_background
            .or(t.background, crate::theme::Role::Background);
        look.badge_icon
            .or(t.foreground, crate::theme::Role::Foreground);
        look.scan.or(t.accent, crate::theme::Role::Accent);
    }
}

// ==========================================
pub const TEMPLATE: &str = include_str!("config/default.toml");
pub const INITIAL: &str = "# 仅保存用户修改的偏好，未设置的字段继承当前版本默认值\n# 使用 mark-shot --default-config 查看完整配置参考\n";

static CONFIG: OnceLock<Config> = OnceLock::new();
static OVERRIDE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub const CONFIG_ARG: &str = "--config";

// complaints are filled before logger starts to work
// so we need to remember em
static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
