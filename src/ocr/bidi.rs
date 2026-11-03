// Reverses right-to-left text for copy/paste operations.
//
// OCR engine reads images left-to-right, returning Arabic characters in visual order.
// Character positions (`char_x`) stay visual, but the text string is converted to logical order
// when copied: Latin text and numbers keep their original direction, while Arabic text is flipped.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Ltr,
    Rtl,
    Neutral,
    Mark,
}

fn class(c: char) -> Class {
    match c as u32 {
        0x0591..=0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED => {
            Class::Mark
        }
        0x0660..=0x0669 | 0x06F0..=0x06F9 => Class::Ltr,
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF => Class::Rtl,
        _ if c.is_alphanumeric() => Class::Ltr,
        _ => Class::Neutral,
    }
}

pub fn is_rtl(text: &str) -> bool {
    text.chars().any(|c| class(c) == Class::Rtl)
}

pub fn to_logical(visual: &str) -> String {
    if !is_rtl(visual) {
        return visual.to_string();
    }
    let chars: Vec<char> = visual.chars().collect();

    let mut units: Vec<(usize, usize, Class)> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        match (class(c), units.last_mut()) {
            (Class::Mark, Some(unit)) => unit.1 = i + 1,
            (Class::Mark, None) => units.push((i, i + 1, Class::Neutral)),
            (cls, _) => units.push((i, i + 1, cls)),
        }
    }

    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let (start, mut end, cls) = units[i];
        i += 1;
        if cls != Class::Ltr {
            runs.push((start, end, false));
            continue;
        }
        loop {
            let mut k = i;
            while k < units.len() && units[k].2 == Class::Neutral {
                k += 1;
            }
            if k < units.len() && units[k].2 == Class::Ltr {
                end = units[k].1;
                i = k + 1;
            } else {
                break;
            }
        }
        runs.push((start, end, true));
    }

    let mut out = String::with_capacity(visual.len());
    for &(start, end, ltr) in runs.iter().rev() {
        for &c in &chars[start..end] {
            out.push(if ltr { c } else { mirror(c) });
        }
    }
    out
}

fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        _ => c,
    }
}
