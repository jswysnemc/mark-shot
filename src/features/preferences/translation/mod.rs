//! 【翻译设置】【页面组合】组合配置字段、目录查询和原生交互

pub(super) mod draw;
pub(super) mod fields;
pub(super) mod input;
pub(super) mod layout;
mod preview;
mod state;
pub(super) use state::State;

#[cfg(test)]
mod tests;
