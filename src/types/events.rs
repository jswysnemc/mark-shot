use crate::keys::Chord;

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PointerState {
    pub monitor_idx: usize,
    pub local: (f64, f64),
    pub global: (f64, f64),
}

impl PointerState {
    pub fn new(monitor_idx: usize, local: (f64, f64), global: (f64, f64)) -> Self {
        Self {
            monitor_idx,
            local,
            global,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    TextInputFocus(super::text_input::Focus),
    TextInputLeave {
        focus_id: u64,
    },
    TextInput(super::text_input::Update),
    PointerMove {
        monitor_idx: usize,
        x: f64,
        y: f64,
    },
    Focus {
        monitor_idx: usize,
    },
    FocusLost {
        monitor_idx: usize,
    },
    PointerButton {
        button: MouseButton,
        pressed: bool,
        serial: Option<u32>,
    },
    Popup {
        id: u64,
        event: super::popup::PopupEvent,
    },
    EscapePressed,
    Tick,
    /// compositor showed the last frame of this monitor, it can take the next one
    FrameDone {
        monitor_idx: usize,
    },
    Key {
        chord: Option<Chord>,
        text: Option<String>,
    },
    ModifiersChanged {
        ctrl: bool,
        shift: bool,
    },
    Scroll(super::scroll::ScrollEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Editor,
    Full,
    Window,
    Region,
    Ocr,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finish {
    Pin,
    Copy,
    Save,
}

/// What to do with the finished shot: any mix of copy, pin and save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outputs {
    pub copy: bool,
    pub pin: bool,
    pub save: bool,
    pub stdout: bool,
    pub primary: bool,
}

impl Outputs {
    /// Letters c, p, s in any order; repeats are fine, an empty string means nothing.
    pub fn parse(letters: &str) -> Result<Self, String> {
        let mut outputs = Self::default();
        for letter in letters.trim().chars() {
            match letter.to_ascii_lowercase() {
                'c' => outputs.copy = true,
                'p' => outputs.pin = true,
                's' => outputs.save = true,
                'o' => outputs.stdout = true,
                'r' => outputs.primary = true,
                other => {
                    return Err(format!(
                        "unknown letter {other:?} in {letters:?}, expected c (copy), p (pin), s (save), o (stdout) or r (primary selection)"
                    ));
                }
            }
        }
        Ok(outputs)
    }

    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    pub fn has(self, finish: Finish) -> bool {
        match finish {
            Finish::Copy => self.copy,
            Finish::Pin => self.pin,
            Finish::Save => self.save,
        }
    }
}

impl std::ops::BitOr for Outputs {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self {
            copy: self.copy || other.copy,
            pin: self.pin || other.pin,
            save: self.save || other.save,
            stdout: self.stdout || other.stdout,
            primary: self.primary || other.primary,
        }
    }
}

impl From<Finish> for Outputs {
    fn from(finish: Finish) -> Self {
        Self {
            copy: finish == Finish::Copy,
            pin: finish == Finish::Pin,
            save: finish == Finish::Save,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_ignore_order_and_repeats() {
        let all = Outputs {
            copy: true,
            pin: true,
            save: true,
            stdout: true,
            primary: true,
        };
        assert_eq!(Outputs::parse("cpsor"), Ok(all));
        assert_eq!(Outputs::parse("rospc"), Ok(all));
        assert_eq!(
            Outputs::parse("PsSp cOR".replace(' ', "").as_str()),
            Ok(all)
        );
        assert_eq!(Outputs::parse("p"), Ok(Outputs::from(Finish::Pin)));
    }

    #[test]
    fn empty_outputs_do_nothing() {
        assert!(Outputs::parse("").unwrap().is_empty());
    }

    #[test]
    fn unknown_letter_is_rejected() {
        assert!(Outputs::parse("cx").is_err());
    }
}

#[derive(Debug, Clone)]
pub enum SpecialKey {
    Backspace,
    Enter,
    Left,
    Right,
    Home,
    End,
    Delete,
    Up,
    Down,
    KeyA,
    KeyC,
    KeyX,
    KeyV,
}
