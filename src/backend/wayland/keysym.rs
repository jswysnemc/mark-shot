use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers};

use crate::keys::{Chord, Key, Mods};

const PHYSICAL: &[(u32, char)] = &[
    (2, '1'),
    (3, '2'),
    (4, '3'),
    (5, '4'),
    (6, '5'),
    (7, '6'),
    (8, '7'),
    (9, '8'),
    (10, '9'),
    (11, '0'),
    (12, '-'),
    (13, '='),
    (16, 'q'),
    (17, 'w'),
    (18, 'e'),
    (19, 'r'),
    (20, 't'),
    (21, 'y'),
    (22, 'u'),
    (23, 'i'),
    (24, 'o'),
    (25, 'p'),
    (26, '['),
    (27, ']'),
    (30, 'a'),
    (31, 's'),
    (32, 'd'),
    (33, 'f'),
    (34, 'g'),
    (35, 'h'),
    (36, 'j'),
    (37, 'k'),
    (38, 'l'),
    (39, ';'),
    (40, '\''),
    (41, '`'),
    (43, '\\'),
    (44, 'z'),
    (45, 'x'),
    (46, 'c'),
    (47, 'v'),
    (48, 'b'),
    (49, 'n'),
    (50, 'm'),
    (51, ','),
    (52, '.'),
    (53, '/'),
];

pub fn mods(modifiers: &Modifiers) -> Mods {
    Mods {
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        alt: modifiers.alt,
        logo: modifiers.logo,
    }
}

fn named(keysym: Keysym) -> Option<Key> {
    let raw = keysym.raw();
    if (Keysym::F1.raw()..=Keysym::F24.raw()).contains(&raw) {
        return Some(Key::F((raw - Keysym::F1.raw() + 1) as u8));
    }
    Some(match keysym {
        Keysym::Escape => Key::Escape,
        Keysym::Return | Keysym::KP_Enter => Key::Enter,
        Keysym::space => Key::Space,
        Keysym::Tab | Keysym::ISO_Left_Tab => Key::Tab,
        Keysym::BackSpace => Key::Backspace,
        Keysym::Delete | Keysym::KP_Delete => Key::Delete,
        Keysym::Insert => Key::Insert,
        Keysym::Home => Key::Home,
        Keysym::End => Key::End,
        Keysym::Page_Up => Key::PageUp,
        Keysym::Page_Down => Key::PageDown,
        Keysym::Left => Key::Left,
        Keysym::Right => Key::Right,
        Keysym::Up => Key::Up,
        Keysym::Down => Key::Down,
        _ => return None,
    })
}

pub fn chord(keysym: Keysym, raw_code: u32, mods: Mods) -> Option<Chord> {
    // for latin keyboards different from QWERTY
    let key = named(keysym).or_else(|| {
        let latin = char::from_u32(keysym.raw()).filter(char::is_ascii_alphabetic);
        let physical = PHYSICAL
            .iter()
            .find(|(code, _)| *code == raw_code)
            .map(|(_, c)| *c);
        latin
            .map(|c| c.to_ascii_lowercase())
            .or(physical)
            .map(Key::Char)
    })?;
    Some(Chord { mods, key })
}
