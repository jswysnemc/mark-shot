// --delay: wait until the shot time, letting the capture backend prepare meanwhile,
// with a countdown when run from a terminal
use crate::backend::CaptureMethod;
use crate::profiler::Profiler;

use std::io::IsTerminal;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(10);

pub(super) async fn wait(
    shot_at: Option<Instant>,
    capture: &dyn CaptureMethod,
    prof: &mut Profiler,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(at) = shot_at else {
        return Ok(());
    };
    capture.prepare().await?;
    prof.mark("capture prepared");
    countdown(at).await;
    prof.idle();
    Ok(())
}

/// only for a terminal
async fn countdown(at: Instant) {
    if !std::io::stderr().is_terminal() {
        return tokio::time::sleep_until(at.into()).await;
    }
    loop {
        let now = Instant::now();
        if now >= at {
            break;
        }
        let left = at - now;
        eprint!(
            "\rscreenshot will be taken in {:.2}s\x1b[K",
            left.as_secs_f64()
        );
        let to_tick = left.as_nanos() % TICK.as_nanos();
        tokio::time::sleep_until((now + Duration::from_nanos(to_tick as u64)).into()).await;
    }
    eprint!("\r\x1b[K");
}
