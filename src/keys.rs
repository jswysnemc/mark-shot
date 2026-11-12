use std::collections::{BTreeMap, HashMap};
mod labels;
use std::sync::{Arc, OnceLock, RwLock};

use log::info;

use crate::config::Binding;
use crate::tools::Tool;
use crate::types::Finish;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,
    Delete,
    Insert,
    PrintScreen,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub mods: Mods,
    pub key: Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Accept,
    Finish(Finish),
    SaveAs,
    OpenWith,
    Extensions,
    Upload,
    Translate,
    ScanCode,
    Cancel,
    Undo,
    Redo,
    SelectAll,
    Delete,
    ToggleUi,
    ToggleMagnifier,
    ToggleTheme,
    ScrollCapture,
    History,
    Settings,
    ClearAnnotations,
    ToggleScope,
    ToggleToolbarLayout,
    RecordVideo,
    RecordGif,
    RecordPause,
    RecordStop,
    SizeUp,
    SizeDown,
    Tool(Tool),
    Move { dir: Dir, fast: bool },
    Resize { dir: Dir, fast: bool },
}

pub const ACTIONS: &[(&str, Action, &[&str])] = &[
    ("accept", Action::Accept, &["Return"]),
    ("copy", Action::Finish(Finish::Copy), &["Ctrl+C"]),
    ("save", Action::Finish(Finish::Save), &["Ctrl+S"]),
    ("save_as", Action::SaveAs, &["Ctrl+Alt+S"]),
    ("open_with", Action::OpenWith, &[]),
    ("extensions", Action::Extensions, &[]),
    ("upload", Action::Upload, &[]),
    ("translate", Action::Translate, &[]),
    ("scan_code", Action::ScanCode, &[]),
    ("pin", Action::Finish(Finish::Pin), &["Ctrl+P"]),
    ("cancel", Action::Cancel, &["Escape"]),
    ("undo", Action::Undo, &["Ctrl+Z"]),
    ("redo", Action::Redo, &["Ctrl+Shift+Z", "Ctrl+Y"]),
    ("select_all", Action::SelectAll, &["Ctrl+A"]),
    ("delete", Action::Delete, &["Delete", "Backspace"]),
    ("toggle_ui", Action::ToggleUi, &["Space"]),
    ("toggle_magnifier", Action::ToggleMagnifier, &["Shift+M"]),
    ("toggle_theme", Action::ToggleTheme, &["Ctrl+Shift+L"]),
    ("scroll_capture", Action::ScrollCapture, &["Ctrl+Shift+S"]),
    ("history", Action::History, &["Ctrl+H"]),
    ("settings", Action::Settings, &["Ctrl+,"]),
    ("clear_annotations", Action::ClearAnnotations, &[]),
    ("toggle_scope", Action::ToggleScope, &[]),
    ("toggle_toolbar_layout", Action::ToggleToolbarLayout, &[]),
    ("record_video", Action::RecordVideo, &["Ctrl+Shift+R"]),
    ("record_gif", Action::RecordGif, &["Ctrl+Shift+G"]),
    ("record_pause", Action::RecordPause, &["Ctrl+Shift+P"]),
    ("record_stop", Action::RecordStop, &["Ctrl+Shift+E"]),
    ("size_up", Action::SizeUp, &["]"]),
    ("size_down", Action::SizeDown, &["["]),
    ("tool_selection", Action::Tool(Tool::Selection), &["S"]),
    ("tool_pick", Action::Tool(Tool::Pick), &["V"]),
    ("tool_ocr", Action::Tool(Tool::Ocr), &["O"]),
    ("tool_eyedropper", Action::Tool(Tool::Eyedropper), &["I"]),
    ("tool_text", Action::Tool(Tool::Text), &["T"]),
    ("tool_pen", Action::Tool(Tool::Pen), &["P"]),
    ("tool_highlighter", Action::Tool(Tool::Highlighter), &["H"]),
    ("tool_laser", Action::Tool(Tool::Laser), &["G"]),
    ("tool_marker", Action::Tool(Tool::Marker), &["K"]),
    ("tool_ruler", Action::Tool(Tool::Ruler), &["U"]),
    ("tool_magnifier", Action::Tool(Tool::Magnifier), &["Z"]),
    ("tool_number", Action::Tool(Tool::Number), &["N"]),
    ("tool_line", Action::Tool(Tool::Line), &["D"]),
    ("tool_arrow", Action::Tool(Tool::Arrow), &["A"]),
    ("tool_rectangle", Action::Tool(Tool::Rectangle), &["R"]),
    ("tool_circle", Action::Tool(Tool::Circle), &["C"]),
    (
        "tool_numerated_arrow",
        Action::Tool(Tool::NumeratedArrow),
        &["Shift+N"],
    ),
    ("tool_redact", Action::Tool(Tool::Redact), &["M", "X"]),
    (
        "move_left",
        Action::Move {
            dir: Dir::Left,
            fast: false,
        },
        &["Left"],
    ),
    (
        "move_right",
        Action::Move {
            dir: Dir::Right,
            fast: false,
        },
        &["Right"],
    ),
    (
        "move_up",
        Action::Move {
            dir: Dir::Up,
            fast: false,
        },
        &["Up"],
    ),
    (
        "move_down",
        Action::Move {
            dir: Dir::Down,
            fast: false,
        },
        &["Down"],
    ),
    (
        "move_left_fast",
        Action::Move {
            dir: Dir::Left,
            fast: true,
        },
        &["Shift+Left"],
    ),
    (
        "move_right_fast",
        Action::Move {
            dir: Dir::Right,
            fast: true,
        },
        &["Shift+Right"],
    ),
    (
        "move_up_fast",
        Action::Move {
            dir: Dir::Up,
            fast: true,
        },
        &["Shift+Up"],
    ),
    (
        "move_down_fast",
        Action::Move {
            dir: Dir::Down,
            fast: true,
        },
        &["Shift+Down"],
    ),
    (
        "resize_left",
        Action::Resize {
            dir: Dir::Left,
            fast: false,
        },
        &["Alt+Left"],
    ),
    (
        "resize_right",
        Action::Resize {
            dir: Dir::Right,
            fast: false,
        },
        &["Alt+Right"],
    ),
    (
        "resize_up",
        Action::Resize {
            dir: Dir::Up,
            fast: false,
        },
        &["Alt+Up"],
    ),
    (
        "resize_down",
        Action::Resize {
            dir: Dir::Down,
            fast: false,
        },
        &["Alt+Down"],
    ),
    (
        "resize_left_fast",
        Action::Resize {
            dir: Dir::Left,
            fast: true,
        },
        &["Alt+Shift+Left"],
    ),
    (
        "resize_right_fast",
        Action::Resize {
            dir: Dir::Right,
            fast: true,
        },
        &["Alt+Shift+Right"],
    ),
    (
        "resize_up_fast",
        Action::Resize {
            dir: Dir::Up,
            fast: true,
        },
        &["Alt+Shift+Up"],
    ),
    (
        "resize_down_fast",
        Action::Resize {
            dir: Dir::Down,
            fast: true,
        },
        &["Alt+Shift+Down"],
    ),
];

const PUNCTUATION: &[(char, &str)] = &[
    ('[', "bracketleft"),
    (']', "bracketright"),
    ('-', "minus"),
    ('=', "equal"),
    (';', "semicolon"),
    ('\'', "apostrophe"),
    (',', "comma"),
    ('.', "period"),
    ('/', "slash"),
    ('\\', "backslash"),
    ('`', "grave"),
];

fn parse_key(name: &str) -> Result<Key, String> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphanumeric() {
            return Ok(Key::Char(c.to_ascii_lowercase()));
        }
        if PUNCTUATION.iter().any(|(p, _)| *p == c) {
            return Ok(Key::Char(c));
        }
        return Err(format!(
            "{name:?} is not a key, for shifted symbols write Shift and the base key"
        ));
    }

    let lower = name.to_ascii_lowercase();
    if let Some((c, _)) = PUNCTUATION.iter().find(|(_, n)| *n == lower) {
        return Ok(Key::Char(*c));
    }
    if let Some(n) = lower.strip_prefix('f')
        && let Ok(n) = n.parse::<u8>()
        && (1..=24).contains(&n)
    {
        return Ok(Key::F(n));
    }
    Ok(match lower.as_str() {
        "escape" | "esc" => Key::Escape,
        "return" | "enter" => Key::Enter,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "insert" | "ins" => Key::Insert,
        "print" | "printscreen" | "print_screen" => Key::PrintScreen,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "page_up" => Key::PageUp,
        "pagedown" | "page_down" => Key::PageDown,
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        _ => return Err(format!("unknown key {name:?}")),
    })
}

pub fn parse(text: &str) -> Result<Chord, String> {
    let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let key = parts
        .pop()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| format!("{text:?} has no key"))?;

    let mut mods = Mods::default();
    for part in parts {
        let flag = match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => &mut mods.ctrl,
            "shift" => &mut mods.shift,
            "alt" => &mut mods.alt,
            "super" | "meta" | "logo" => &mut mods.logo,
            _ => return Err(format!("unknown modifier {part:?} in {text:?}")),
        };
        *flag = true;
    }
    Ok(Chord {
        mods,
        key: parse_key(key)?,
    })
}

pub struct Keymap {
    binds: HashMap<Chord, Action>,
}

impl Keymap {
    // wrong bindings fall back to defaults per-action
    pub fn build(overrides: &BTreeMap<String, Binding>) -> Self {
        for name in overrides.keys() {
            if !ACTIONS.iter().any(|(n, ..)| n == name) {
                info!("config: keys.{name} is not an action, ignoring it");
            }
        }

        let mut binds = HashMap::new();
        let mut owners: HashMap<Chord, &str> = HashMap::new();
        for (name, action, defaults) in ACTIONS {
            let chords = match overrides.get(*name) {
                Some(binding) => match binding.chords().iter().map(|c| parse(c)).collect() {
                    Ok(chords) => chords,
                    Err(e) => {
                        info!("config: keys.{name}: {e}, using the default");
                        parse_all(defaults)
                    }
                },
                None => parse_all(defaults),
            };
            for chord in chords {
                if let Some(owner) = owners.get(&chord) {
                    info!(
                        "config: keys.{name} reuses a chord of keys.{owner}, keeping keys.{owner}"
                    );
                    continue;
                }
                owners.insert(chord, name);
                binds.insert(chord, *action);
            }
        }
        Self { binds }
    }

    pub fn lookup(&self, chord: Chord) -> Option<Action> {
        self.binds.get(&chord).copied()
    }
}

fn parse_all(chords: &[&str]) -> Vec<Chord> {
    chords
        .iter()
        .map(|c| parse(c).expect("default chords parse"))
        .collect()
}

static MAP: OnceLock<RwLock<Arc<Keymap>>> = OnceLock::new();

/// 【快捷键】【有效映射】无参数，返回当前绑定快照；同一输入事件不读取配置文件
pub fn map() -> Arc<Keymap> {
    MAP.get_or_init(|| RwLock::new(Arc::new(Keymap::build(&crate::config::get().keys.0))))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// 【快捷键】【设置生效】按 overrides 替换完整映射，移除旧绑定，保留构建时冲突检查
pub fn reload(overrides: &BTreeMap<String, Binding>) {
    let value = Arc::new(Keymap::build(overrides));
    let map = MAP.get_or_init(|| RwLock::new(value.clone()));
    *map.write().unwrap_or_else(|error| error.into_inner()) = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(mods: Mods, key: Key) -> Chord {
        Chord { mods, key }
    }

    const CTRL: Mods = Mods {
        ctrl: true,
        shift: false,
        alt: false,
        logo: false,
    };

    #[test]
    fn modifiers_parse_in_any_order_and_case() {
        let expected = chord(
            Mods {
                ctrl: true,
                shift: true,
                ..Mods::default()
            },
            Key::Char('z'),
        );
        assert_eq!(parse("Ctrl+Shift+Z").unwrap(), expected);
        assert_eq!(parse("shift + control + z").unwrap(), expected);
    }

    #[test]
    fn named_and_punctuation_keys_parse() {
        assert_eq!(parse("Return").unwrap().key, Key::Enter);
        assert_eq!(parse("esc").unwrap().key, Key::Escape);
        assert_eq!(parse("F12").unwrap().key, Key::F(12));
        assert_eq!(parse("[").unwrap().key, Key::Char('['));
        assert_eq!(parse("bracketright").unwrap().key, Key::Char(']'));
        assert_eq!(
            parse("Super+1").unwrap(),
            chord(
                Mods {
                    logo: true,
                    ..Mods::default()
                },
                Key::Char('1')
            )
        );
    }

    #[test]
    fn bad_chords_are_rejected() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+").is_err());
        assert!(parse("Hyper+A").is_err());
        assert!(parse("!").is_err());
        assert!(parse("F25").is_err());
    }

    #[test]
    fn defaults_parse_and_never_collide() {
        let mut seen = HashMap::new();
        for (name, _, defaults) in ACTIONS {
            for text in *defaults {
                let chord = parse(text).unwrap();
                assert!(seen.insert(chord, *name).is_none(), "{name} reuses {text}");
            }
        }
    }

    /// 【快捷键测试】【滤镜入口】原版 M 与旧 X 进入滤镜，光标放大镜使用 Shift 加 M，不遮盖工具入口
    #[test]
    fn filter_shortcuts_do_not_collide_with_cursor_magnifier_visibility() {
        let map = Keymap::build(&BTreeMap::new());
        for value in ["M", "X"] {
            assert_eq!(
                map.lookup(parse(value).unwrap()),
                Some(Action::Tool(Tool::Redact))
            );
        }
        assert_eq!(
            map.lookup(parse("Shift+M").unwrap()),
            Some(Action::ToggleMagnifier)
        );
    }

    /// 【快捷键测试】【激光入口】验证激光使用 G，取色器使用 I，不互相覆盖
    #[test]
    fn laser_and_eyedropper_have_distinct_shortcuts() {
        let map = Keymap::build(&BTreeMap::new());
        assert_eq!(
            map.lookup(parse("G").unwrap()),
            Some(Action::Tool(Tool::Laser))
        );
        assert_eq!(
            map.lookup(parse("I").unwrap()),
            Some(Action::Tool(Tool::Eyedropper))
        );
    }

    #[test]
    fn override_replaces_the_defaults() {
        let overrides = BTreeMap::from([("copy".into(), Binding::One("Ctrl+Shift+C".into()))]);
        let map = Keymap::build(&overrides);
        assert_eq!(map.lookup(chord(CTRL, Key::Char('c'))), None);
        assert_eq!(
            map.lookup(parse("Ctrl+Shift+C").unwrap()),
            Some(Action::Finish(Finish::Copy))
        );
    }

    #[test]
    fn empty_binding_disables_the_action() {
        let overrides = BTreeMap::from([("tool_ocr".into(), Binding::One(String::new()))]);
        let map = Keymap::build(&overrides);
        assert_eq!(map.lookup(parse("O").unwrap()), None);
    }

    #[test]
    fn a_bad_binding_keeps_only_its_own_default() {
        let overrides = BTreeMap::from([
            ("save".into(), Binding::One("Ctrl+Nope".into())),
            ("pin".into(), Binding::One("Ctrl+Shift+P".into())),
        ]);
        let map = Keymap::build(&overrides);
        assert_eq!(
            map.lookup(chord(CTRL, Key::Char('s'))),
            Some(Action::Finish(Finish::Save))
        );
        assert_eq!(
            map.lookup(parse("Ctrl+Shift+P").unwrap()),
            Some(Action::Finish(Finish::Pin))
        );
    }

    #[test]
    fn a_duplicate_chord_keeps_the_first_action() {
        let overrides = BTreeMap::from([("save".into(), Binding::One("Ctrl+C".into()))]);
        let map = Keymap::build(&overrides);
        assert_eq!(
            map.lookup(chord(CTRL, Key::Char('c'))),
            Some(Action::Finish(Finish::Copy))
        );
    }
}
