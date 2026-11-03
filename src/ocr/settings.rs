/// see when and why each mode is chosen in [crate::ocr] top comments
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// build up when ocr tool is selected, dies with the overlay
    OnDemand,
    /// build up with every programm launch, dies with the overlay
    AtLaunch,
    /// ready engine in resident background process `--ocr-daemon`
    Daemon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Device {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineSettings {
    pub mode: Mode,
    pub device: Device,
    /// `None` disables the idle auto-exit behavior for the daemon.
    pub daemon_idle: Option<Duration>,
}

// (feature = "ocr-gpu")
pub const GPU_BUILD: bool = cfg!(feature = "ocr-gpu");

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            mode: Mode::OnDemand,
            device: Device::Auto,
            daemon_idle: Some(Duration::from_secs(900)),
        }
    }
}

impl EngineSettings {
    pub fn load() -> Self {
        let ocr = &crate::config::get().ocr;
        Self {
            mode: ocr.mode,
            device: ocr.device,
            daemon_idle: (ocr.daemon_idle_secs > 0)
                .then(|| Duration::from_secs(ocr.daemon_idle_secs)),
        }
    }

    /// Resolves the actual execution mode.
    ///
    /// daemon is only beneficial with GPU acceleration. Therefore, if GPU access
    /// is unavailable, `daemon` mode silently falls back to `on-demand`. Explicitly setting
    /// `device = cpu` bypasses this fallback to allow daemon debugging.
    pub fn resolve(&self, daemon_possible: bool, gpu_possible: bool) -> Mode {
        match self.mode {
            Mode::Daemon if !daemon_possible => Mode::OnDemand,
            Mode::Daemon if self.device == Device::Cpu || gpu_possible => Mode::Daemon,
            Mode::Daemon => Mode::OnDemand,
            mode => mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_needs_a_gpu_unless_cpu_is_forced() {
        let daemon = EngineSettings {
            mode: Mode::Daemon,
            ..EngineSettings::default()
        };
        assert_eq!(daemon.resolve(true, true), Mode::Daemon);
        assert_eq!(daemon.resolve(true, false), Mode::OnDemand);
        assert_eq!(daemon.resolve(false, true), Mode::OnDemand);

        let gpu = EngineSettings {
            device: Device::Gpu,
            ..daemon
        };
        assert_eq!(gpu.resolve(true, false), Mode::OnDemand);

        let cpu = EngineSettings {
            device: Device::Cpu,
            ..daemon
        };
        assert_eq!(cpu.resolve(true, false), Mode::Daemon);
        assert_eq!(cpu.resolve(false, false), Mode::OnDemand);
    }

    #[test]
    fn local_modes_are_kept() {
        for mode in [Mode::OnDemand, Mode::AtLaunch] {
            let settings = EngineSettings {
                mode,
                ..EngineSettings::default()
            };
            assert_eq!(settings.resolve(false, false), mode);
            assert_eq!(settings.resolve(true, true), mode);
        }
    }

    #[test]
    fn nothing_runs_in_the_background_by_default() {
        let settings = EngineSettings::default();
        assert_eq!(settings.mode, Mode::OnDemand);
        assert_eq!(settings.resolve(true, true), Mode::OnDemand);
    }
}
