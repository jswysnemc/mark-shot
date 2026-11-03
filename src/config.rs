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
    ($name:ident { $($field:ident : $ty:ty = $default:expr),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(default)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }
    };
}

mod values;
pub use values::*;
mod sections;
pub use sections::*;
mod bindings;
pub use bindings::*;
mod loading;
pub use loading::*;
pub mod history;
pub use history::History;
#[cfg(test)]
mod tests;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba(r, g, b, 255)
}

// 【界面】【默认主题】紧凑石墨色面板，薄荷绿色用于交互强调
const TEXT: Rgba = rgb(229, 237, 239);
const OVERLAY2: Rgba = rgb(147, 153, 178);
const SURFACE0: Rgba = rgb(39, 45, 47);
const SURFACE1: Rgba = rgb(59, 70, 71);
const CRUST: Rgba = rgb(16, 22, 23);
const BLUE: Rgba = rgb(137, 180, 250);
const ROSEWATER: Rgba = rgb(245, 224, 220);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub save: Save,
    pub pinned: Pinned,
    pub history: History,
    pub notifications: Notifications,
    pub animation: Animation,
    pub theme: Theme,
    pub toolbar: Toolbar,
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
    pub log: Log,
    pub keys: Keys,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            general: General::default(),
            save: Save::default(),
            pinned: Pinned::default(),
            history: History::default(),
            notifications: Notifications::default(),
            animation: Animation::default(),
            theme: Theme::default(),
            toolbar: Toolbar::default(),
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
        toolbar.background.or(t.background);
        toolbar.icon.or(t.foreground);
        toolbar.icon_hovered.or(t.on_accent);
        toolbar.icon_selected.or(t.on_accent);
        toolbar.icon_to.or(t.accent);
        toolbar.button_hovered.or(t.hover);
        toolbar.button_selected.or(t.accent);
        toolbar.separator.or(t.foreground);

        let panel = &mut self.settings_panel;
        panel.background.or(t.background);
        panel.text.or(t.foreground);
        panel.icon.or(t.foreground);
        panel.icon_hovered.or(t.hover);
        panel.icon_active.or(t.accent);
        panel.separator.or(t.foreground);

        let picker = &mut self.color_picker;
        picker.background.or(t.background);
        picker.text.or(t.foreground);
        picker.label.or(t.muted);
        picker.marker.or(t.foreground);

        self.model_picker.background.or(t.background);

        let magnifier = &mut self.magnifier;
        magnifier.outline.or(t.foreground);
        magnifier.label_background.or(t.background);
        magnifier.label_text.or(t.foreground);

        self.selection.border.or(t.foreground);
        self.annotations.handles.color.or(t.foreground);
        self.annotations.text.selected_text.or(t.foreground);

        self.toasts.background.or(t.background);
        self.toasts.text.or(t.foreground);

        let look = &mut self.ocr.look;
        look.badge_background.or(t.background);
        look.badge_icon.or(t.foreground);
        look.scan.or(t.accent);
    }
}

// ==========================================
pub const TEMPLATE: &str = include_str!("config/default.toml");

static CONFIG: OnceLock<Config> = OnceLock::new();
static OVERRIDE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub const CONFIG_ARG: &str = "--config";

// complaints are filled before logger starts to work
// so we need to remember em
static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
