// Scrolling capture: frame cropping, preview thumbnails, panel and session state.

pub mod frame;
pub mod panel;
pub(crate) mod png;
pub mod preview;
pub mod session;
pub(crate) mod source;

#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod session_tests;

pub use session::ScrollSession;

#[cfg(all(test, target_os = "linux"))]
mod overlay_tests;
