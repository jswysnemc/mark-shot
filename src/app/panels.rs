// Panel behaviour: what each panel shows, where it sits and where its edits land.
// The panels themselves (state + drawing) live in ui/.

pub mod color;
pub mod filter;
pub mod line;
pub mod marker;
pub mod model;
pub mod number;
pub mod radial_palette;
pub mod ruler;
pub mod settings;
pub mod text;
pub mod toolbar;

pub(crate) mod toolbar_docking;
