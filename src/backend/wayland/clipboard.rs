pub mod daemon;
pub mod ext_data_control;

use std::time::Duration;

/// overlay re-execs itself with these args. `cli` on the other side reads them
pub const DAEMON_ARG: &str = "--clipboard-daemon";
pub const TEXT_ARG: &str = "text";
pub const PRIMARY_ARG: &str = "primary";

/// how long the parent waits for the helper to claim the selection
pub const READY_TIMEOUT: Duration = Duration::from_secs(3);
