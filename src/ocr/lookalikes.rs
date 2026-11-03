// reponsible to deal with mix letters from different alhpabets that look the same
// Visually it looks correct, not always tho, and simply not so pleasent to work with.
//
// force each word to use single consistent alphabet. If a word could belong
// to multiple alphabets, we use the alphabet of the nearest clear word.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
}

const SCRIPTS: [Script; 3] = [Script::Latin, Script::Cyrillic, Script::Greek];

const CYRILLIC: &[(char, char)] = &[
    ('А', 'A'),
    ('В', 'B'),
    ('Е', 'E'),
    ('К', 'K'),
    ('М', 'M'),
    ('Н', 'H'),
    ('О', 'O'),
    ('Р', 'P'),
    ('С', 'C'),
    ('Т', 'T'),
    ('Х', 'X'),
    ('а', 'a'),
    ('е', 'e'),
    ('о', 'o'),
    ('р', 'p'),
    ('с', 'c'),
    ('у', 'y'),
    ('х', 'x'),
];

// turns into latin, never vice versas
const CYRILLIC_TO_LATIN: &[(char, char)] = &[
    ('І', 'I'),
    ('Ї', 'Ï'),
    ('Ј', 'J'),
    ('Ѕ', 'S'),
    ('Ү', 'Y'),
    ('Ѵ', 'V'),
    ('Ԛ', 'Q'),
    ('Ԝ', 'W'),
    ('Ӏ', 'l'),
    ('ԁ', 'd'),
    ('һ', 'h'),
    ('і', 'i'),
    ('ї', 'ï'),
    ('ј', 'j'),
    ('ӏ', 'l'),
    ('ԛ', 'q'),
    ('ѕ', 's'),
    ('ү', 'y'),
    ('ѵ', 'v'),
    ('ԝ', 'w'),
];

// no language of the model uses these, so its definetely latin
const FOREIGN: &[char] = &['Ӏ', 'ӏ', 'ԁ', 'Ԛ', 'ԛ', 'Ԝ', 'ԝ', 'Ѵ', 'ѵ'];

const GREEK: &[(char, char)] = &[
    ('Α', 'A'),
    ('Β', 'B'),
    ('Ε', 'E'),
    ('Ζ', 'Z'),
    ('Η', 'H'),
    ('Ι', 'I'),
    ('Κ', 'K'),
    ('Μ', 'M'),
    ('Ν', 'N'),
    ('Ο', 'O'),
    ('Ρ', 'P'),
    ('Τ', 'T'),
    ('Υ', 'Y'),
    ('Χ', 'X'),
    ('ν', 'v'),
    ('ο', 'o'),
    ('ρ', 'p'),
];

fn script(c: char) -> Option<Script> {
    match c {
        'A'..='Z' | 'a'..='z' | '\u{C0}'..='\u{2AF}' | '\u{1E00}'..='\u{1EFF}' => {
            Some(Script::Latin)
        }
        '\u{400}'..='\u{52F}' | '\u{1C80}'..='\u{1C8F}' | '\u{A640}'..='\u{A69F}' => {
            Some(Script::Cyrillic)
        }
        '\u{370}'..='\u{3FF}' | '\u{1F00}'..='\u{1FFF}' => Some(Script::Greek),
        _ => None,
    }
}

fn find(table: &[(char, char)], c: char) -> Option<char> {
    table.iter().find(|&&(own, _)| own == c).map(|&(_, l)| l)
}

fn find_back(table: &[(char, char)], latin: char) -> Option<char> {
    table
        .iter()
        .find(|&&(_, l)| l == latin)
        .map(|&(own, _)| own)
}

fn latin(c: char) -> Option<char> {
    match script(c)? {
        Script::Latin => Some(c),
        Script::Cyrillic => find(CYRILLIC, c).or_else(|| find(CYRILLIC_TO_LATIN, c)),
        Script::Greek => find(GREEK, c),
    }
}

/// returns c's twin from `to`, if there is one
fn write_in(c: char, to: Script) -> Option<char> {
    if c.is_numeric() {
        return Some(c);
    }
    if script(c) == Some(to) {
        return (!FOREIGN.contains(&c)).then_some(c);
    }
    let l = latin(c)?;
    match to {
        Script::Latin => Some(l),
        Script::Cyrillic => find_back(CYRILLIC, l),
        Script::Greek => find_back(GREEK, l),
    }
}

fn candidates(word: &[char]) -> Vec<Script> {
    SCRIPTS
        .into_iter()
        .filter(|&s| word.iter().all(|&c| write_in(c, s).is_some()))
        .collect()
}

fn decided(word: &[char]) -> Option<Script> {
    match candidates(word)[..] {
        [only] => Some(only),
        _ => None,
    }
}

pub fn fix(text: &str) -> String {
    let mut out: Vec<char> = text.chars().collect();
    let mut words: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, c) in out.iter().enumerate() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                words.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        words.push((s, out.len()));
    }

    let verdicts: Vec<Option<Script>> = words.iter().map(|&(s, e)| decided(&out[s..e])).collect();
    let neighbour = |i: usize| {
        (1..words.len()).find_map(|d| {
            let left = i.checked_sub(d).and_then(|j| verdicts[j]);
            let right = verdicts.get(i + d).copied().flatten();
            left.or(right)
        })
    };

    for (i, &(s, e)) in words.iter().enumerate() {
        let word = &out[s..e];
        let cands = candidates(word);
        let target = match cands[..] {
            [] => None,
            [only] => Some(only),
            // when its letters + numbers, and its unclear to which alphabet the letter
            // belongs to, it uses latin
            _ if word.iter().any(|c| c.is_numeric()) => Some(Script::Latin),
            // single letters are left as it is
            [..] if e - s == 1 => None,
            _ => neighbour(i).filter(|s| cands.contains(s)),
        };
        if let Some(to) = target {
            for c in &mut out[s..e] {
                *c = write_in(*c, to).unwrap_or(*c);
            }
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::fix;

    #[test]
    fn latin_words_restored() {
        assert_eq!(fix("Aрреaranсе"), "Appearance");
        assert_eq!(fix("onnх"), "onnx");
        assert_eq!(fix("HelӀо"), "Hello");
        assert_eq!(fix("аррӀу"), "apply");
    }

    #[test]
    fn following_neighbors() {
        assert_eq!(fix("Нажмите Ha кнопку"), "Нажмите На кнопку");
        assert_eq!(fix("Ok, сору"), "Ok, copy");
    }

    #[test]
    fn single_letters_dont_get_converted() {
        for text in ["Fedora і Ubuntu", "release с GitHub", "Press a key"] {
            assert_eq!(fix(text), text);
        }
    }

    #[test]
    fn latin_words_stay_latin() {
        for text in [
            "нажмите Allow и",
            "только Shell.Screenshot",
            "в Vivaldi нет",
            "движок ONNX Runtime",
        ] {
            assert_eq!(fix(text), text);
        }
        assert_eq!(fix("а Х11 пока"), "а X11 пока");
    }

    #[test]
    fn real_cyrillic_is_left_alone() {
        for text in [
            "Україна і світ",
            "Работа с файлами",
            "сор",
            "ЈЕДАН",
            "Ѕвезда ѕ",
        ] {
            assert_eq!(fix(text), text);
        }
    }

    #[test]
    fn mixed_words_are_left_alone() {
        assert_eq!(fix("Windowsы"), "Windowsы");
    }

    #[test]
    fn greek_takes_the_same_path() {
        assert_eq!(fix("ΕΛΛAΣ"), "ΕΛΛΑΣ");
        assert_eq!(fix("ΟΚ ready"), "OK ready");
    }

    #[test]
    fn character_count_is_kept() {
        let text = "Нажмите аррӀу, x86_64 ΟΚ";
        assert_eq!(fix(text).chars().count(), text.chars().count());
    }
}
