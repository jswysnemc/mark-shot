use super::*;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Binding {
    One(String),
    Many(Vec<String>),
}

impl Binding {
    pub fn chords(&self) -> Vec<&str> {
        let all: Vec<&str> = match self {
            Binding::One(chord) => vec![chord.as_str()],
            Binding::Many(chords) => chords.iter().map(String::as_str).collect(),
        };
        all.into_iter().filter(|c| !c.trim().is_empty()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Keys(pub BTreeMap<String, Binding>);

impl Default for Keys {
    fn default() -> Self {
        let binding = |chords: &[&str]| match chords {
            [one] => Binding::One((*one).into()),
            many => Binding::Many(many.iter().map(|c| (*c).into()).collect()),
        };
        Self(
            crate::keys::ACTIONS
                .iter()
                .map(|(name, _, chords)| ((*name).into(), binding(chords)))
                .collect(),
        )
    }
}
