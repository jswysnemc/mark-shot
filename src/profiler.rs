use std::time::{Duration, Instant};

/// Startup timings, printed to stderr with `--speed`.
pub struct Profiler {
    t0: Instant,
    last: Instant,
    enabled: bool,
    lines: Vec<String>,
}

impl Profiler {
    pub fn new(enabled: bool) -> Self {
        let now = Instant::now();
        Self {
            t0: now,
            last: now,
            enabled,
            lines: Vec::new(),
        }
    }

    pub fn mark(&mut self, label: &str) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        self.lines.push(format!(
            "[{:>6.2}ms] (+{:>5.2}ms) {label}",
            self.t0.elapsed().as_secs_f64() * 1000.0,
            now.duration_since(self.last).as_secs_f64() * 1000.0,
        ));
        self.last = now;
    }

    /// used to show taken delay
    pub fn idle(&mut self) {
        let now = Instant::now();
        let idle = now.duration_since(self.last);
        self.t0 += idle;
        self.last = now;
        if self.enabled {
            self.lines.push(format!(
                "           ({:>7.0}ms) was delayed (-d / --delay)",
                idle.as_secs_f64() * 1000.0
            ));
        }
    }

    pub fn mark_external(&mut self, label: &str, elapsed_since_t0: Duration) {
        if !self.enabled {
            return;
        }
        self.lines.push(format!(
            "[{:>6.2}ms] (thread)   {label}",
            elapsed_since_t0.as_secs_f64() * 1000.0,
        ));
    }

    pub fn dump(&self) {
        if !self.enabled || self.lines.is_empty() {
            return;
        }
        eprintln!("--- timing ---\n{}", self.lines.join("\n"));
    }
}
