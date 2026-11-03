// Scrolling capture: frame cropping, preview thumbnails, panel and session state.

pub mod frame;
pub mod panel;
pub mod preview;
pub mod session;

#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod session_tests;

pub use session::ScrollSession;

#[cfg(test)]
mod overlay_tests;
