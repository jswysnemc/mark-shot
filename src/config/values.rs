use super::*;

// Value types
// ==========================================

/// color that defaults to a theme role unless explicitly set in config (`""` also keeps the default).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ThemeColor(Option<Rgba>);

impl ThemeColor {
    pub fn get(self) -> Rgba {
        // resolve() is called for every Config, unset color is a bug
        self.0.unwrap_or(Rgba(255, 0, 255, 255))
    }

    pub(super) fn or(&mut self, role: Rgba) {
        self.0.get_or_insert(role);
    }
}

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s.trim().is_empty() {
            return Ok(Self(None));
        }
        parse_rgba(&s)
            .map(|c| Self(Some(c)))
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for ThemeColor {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Some(color) => color.serialize(s),
            None => s.serialize_str(""),
        }
    }
}

pub(super) fn parse_rgba(s: &str) -> Result<Rgba, String> {
    Rgba::parse_hex(s).ok_or_else(|| format!("invalid color {s:?}, expected #RRGGBB or #RRGGBBAA"))
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        parse_rgba(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Rgba(r, g, b, a) = *self;
        let hex = if a == 255 {
            format!("#{r:02X}{g:02X}{b:02X}")
        } else {
            format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
        };
        s.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Outputs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Outputs::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Outputs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let letters: String = [
            (self.copy, 'c'),
            (self.pin, 'p'),
            (self.save, 's'),
            (self.stdout, 'o'),
            (self.primary, 'r'),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, letter)| letter)
        .collect();
        s.serialize_str(&letters)
    }
}

// ==========================================
// Sections
// ==========================================
