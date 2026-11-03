// cursor form set
// names match cursor-shape-v1 / CSS

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorIcon {
    #[default]
    Default,
    Crosshair,
    Pointer,
    Text,
    Grab,
    Grabbing,
    NsResize,
    EwResize,
    NeswResize,
    NwseResize,
}
