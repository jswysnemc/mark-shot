use crate::keys::Chord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// 【图片编辑】【窗口调整】普通编辑窗口的合成器确认尺寸，原图与标注保持原坐标
    CanvasResized {
        size: (u32, u32),
    },
    /// 【主选择区】【接收完成】id 绑定本次粘贴，空正文表示来源不可用或传输失败
    PrimaryText {
        id: u64,
        text: Option<String>,
    },
    TextInputFocus(super::text_input::Focus),
    TextInputLeave {
        focus_id: u64,
    },
    TextInput(super::text_input::Update),
    PointerMove {
        monitor_idx: usize,
        x: f64,
        y: f64,
        /// 【截图输入】【合并轨迹】保存有界实际位置和同输出共线端点，未合并事件保持空列表
        history: Vec<super::pointer_motion::PointerMotion>,
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
        /// 【截图输入】【按钮时间】Wayland 原始毫秒时间，未使用协议输入的调用保持空值
        timestamp: Option<u32>,
    },
    Popup {
        id: u64,
        event: super::popup::PopupEvent,
    },
    /// 【截图弹层】【键盘序列】真实按键的原生序列号，先于对应按键动作分派，用于建立新的菜单抓取
    PopupSerial(u32),
    EscapePressed,
    /// 【界面主题】【外部通知】同配置的另一个进程已更新主题，存活控件需要刷新
    ThemeChanged,
    Tick,
    /// compositor showed the last frame of this monitor, it can take the next one
    FrameDone {
        monitor_idx: usize,
    },
    ScrollPanelConfigured {
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
