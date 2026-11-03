// Runs OCR off the UI thread to keep the app responsive.
//
// Model loading and text recognition are slow (100+ ms), so they run on a
// single background worker thread. Only one job runs at a time.
//
// Highlights:
// - Non-blocking: Requests sent before the engine finishes loading are queued.
// - Error handling: If loading fails, future requests are rejected instantly
//   without retrying on the UI thread.
// - on-demand: engine doesn't build up, before choosing ocr tool (prepare / start).
// - cancellation flag: polled by the engine between pipeline stages.

use log::error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use super::models::ModelFiles;
use super::settings::Mode;
use super::{OcrBackend, OcrError, OcrImage, OcrText, build_backend};

type JobDone = (Box<dyn OcrBackend>, Result<OcrText, String>);
type WarmDone = Result<Box<dyn OcrBackend>, String>;
type Build = Arc<dyn Fn() -> Result<Box<dyn OcrBackend>, OcrError> + Send + Sync>;

pub enum StartOutcome {
    /// Running, or parked until the engine is ready.
    Started,
    /// A recognition is already in flight.
    Busy,
    /// The engine could not be built (see the contained message).
    Unavailable(String),
}

struct Job {
    rx: Receiver<JobDone>,
    cancel: Arc<AtomicBool>,
}

pub struct OcrRuntime {
    mode: Mode,
    build: Option<Build>,
    backend: Option<Box<dyn OcrBackend>>,
    warm_rx: Option<Receiver<WarmDone>>,
    job: Option<Job>,
    /// Parked until the engine is ready.
    queued: Option<OcrImage>,
    /// Set if the engine fails to build; later requests fail fast.
    failed: Option<String>,
    /// we can't kill the thread, so we wait for it to finish, and only then
    /// return the engine with discarding the text
    discarded: bool,
    // Engine in the running task was built with the previous model; do not reuse it further
    stale: bool,
}

impl Default for OcrRuntime {
    fn default() -> Self {
        Self::new(Mode::OnDemand)
    }
}

impl OcrRuntime {
    /// starts empty; `load` picks the model, the mode decides when it is built.
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            build: None,
            backend: None,
            warm_rx: None,
            job: None,
            queued: None,
            failed: None,
            discarded: false,
            stale: false,
        }
    }

    pub fn load(&mut self, files: ModelFiles) {
        let mode = self.mode;
        self.set_build(Arc::new(move || build_backend(mode, &files)));
    }

    fn set_build(&mut self, build: Build) {
        self.build = Some(build);
        self.backend = None;
        self.warm_rx = None;
        self.failed = None;
        self.stale = self.job.is_some();
        if self.mode != Mode::OnDemand || self.queued.is_some() {
            self.warm();
        }
    }

    /// Triggers build initialization immediately if the engine is uninitialized, not currently building, and not pending task completion.
    pub fn prepare(&mut self) {
        let coming_back = self.job.is_some() && !self.stale;
        if self.backend.is_none() && self.warm_rx.is_none() && self.failed.is_none() && !coming_back
        {
            self.warm();
        }
    }

    fn warm(&mut self) {
        let Some(build) = self.build.clone() else {
            return;
        };
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(build().map_err(|e| e.to_string()));
        });
        self.warm_rx = Some(rx);
    }

    /// A scan somebody is still waiting for: running, or parked until the
    /// engine is ready. A cancelled one does not count.
    pub fn is_busy(&self) -> bool {
        (self.job.is_some() && !self.discarded) || self.queued.is_some()
    }

    pub fn needs_poll(&self) -> bool {
        self.job.is_some() || self.queued.is_some()
    }

    pub fn cancel(&mut self) {
        self.queued = None;
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
            self.discarded = true;
        }
    }

    /// Start recognizing `image`. Parks the request if the engine isn't ready.
    pub fn start(&mut self, image: OcrImage) -> StartOutcome {
        self.collect_warm();
        if let Some(err) = &self.failed {
            return StartOutcome::Unavailable(err.clone());
        }
        if self.is_busy() {
            return StartOutcome::Busy;
        }
        if self.build.is_none() && self.backend.is_none() && self.job.is_none() {
            return StartOutcome::Unavailable("no model is loaded".into());
        }
        // Hold off sending the new engine to the thread until the old task returns
        // `stale` tracks whether the old task is still running.
        if self.job.is_none()
            && let Some(backend) = self.backend.take()
        {
            self.spawn(backend, image);
        } else {
            self.queued = Some(image);
            self.prepare();
        }
        StartOutcome::Started
    }

    /// Once per event-loop iteration. Returns the text when a job finishes,
    /// or the reason it could not run.
    pub fn poll(&mut self) -> Option<Result<OcrText, String>> {
        self.collect_warm();

        // Engine arrived while a request was parked.
        if self.job.is_none()
            && let Some(image) = self.queued.take()
        {
            match self.backend.take() {
                Some(backend) => self.spawn(backend, image),
                None => {
                    self.queued = Some(image);
                    self.prepare();
                }
            }
        }

        // Engine failed while a request was parked.
        if self.queued.is_some()
            && let Some(err) = self.failed.clone()
        {
            self.queued = None;
            return Some(Err(err));
        }

        let job = self.job.as_ref()?;
        let (backend, result) = match job.rx.try_recv() {
            Ok((backend, result)) => (Some(backend), result),
            Err(TryRecvError::Empty) => return None,
            // Engine thread panicked or crashed: the next request will initialize a new instance using `prepare`
            Err(TryRecvError::Disconnected) => (None, Err("recognition thread vanished".into())),
        };
        if !self.stale
            && let Some(backend) = backend
        {
            self.backend = Some(backend);
        }
        self.job = None;
        self.stale = false;

        if std::mem::take(&mut self.discarded) {
            return None;
        }
        Some(result)
    }

    fn spawn(&mut self, backend: Box<dyn OcrBackend>, image: OcrImage) {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            let result = backend
                .recognize(image, &|| flag.load(Ordering::Relaxed))
                .map_err(|e| e.to_string());
            let _ = tx.send((backend, result));
        });
        self.job = Some(Job { rx, cancel });
    }

    /// Collect the backend if the build thread is done. Non-blocking.
    fn collect_warm(&mut self) {
        let Some(rx) = &self.warm_rx else { return };
        match rx.try_recv() {
            Ok(Ok(backend)) => {
                self.backend = Some(backend);
                self.warm_rx = None;
            }
            Ok(Err(e)) => {
                error!("ocr: failed to initialise engine: {e}");
                self.failed = Some(e);
                self.warm_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.failed = Some("engine build thread vanished".into());
                self.warm_rx = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::testing::{self, Fake, label_of, wait_until};
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    const LONG: Duration = Duration::from_secs(5);

    // Runtime manager tracking build counts and yielding engine instances via `make(build_id)`
    fn runtime(
        mode: Mode,
        make: impl Fn(usize) -> Result<Fake, String> + Send + Sync + 'static,
    ) -> (OcrRuntime, Arc<AtomicUsize>) {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut rt = OcrRuntime::new(mode);
        rt.set_build(counting(&builds, make));
        (rt, builds)
    }

    fn counting(
        builds: &Arc<AtomicUsize>,
        make: impl Fn(usize) -> Result<Fake, String> + Send + Sync + 'static,
    ) -> Build {
        let builds = builds.clone();
        Arc::new(move || {
            let n = builds.fetch_add(1, Ordering::SeqCst);
            make(n)
                .map(|fake| Box::new(fake) as Box<dyn OcrBackend>)
                .map_err(Into::into)
        })
    }

    fn result(rt: &mut OcrRuntime) -> Option<Result<OcrText, String>> {
        let deadline = Instant::now() + LONG;
        while Instant::now() < deadline {
            if let Some(result) = rt.poll() {
                return Some(result);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }

    fn drained(rt: &mut OcrRuntime) -> bool {
        wait_until(LONG, || {
            assert!(
                rt.poll().is_none(),
                "a cancelled scan must not deliver text"
            );
            !rt.needs_poll()
        })
    }

    fn label(result: Option<Result<OcrText, String>>) -> String {
        label_of(&result.expect("no result in time").expect("scan failed"))
    }

    fn img() -> OcrImage {
        testing::image(6, 4)
    }

    #[test]
    fn scan_before_the_engine_is_ready_is_parked() {
        let (mut rt, builds) = runtime(Mode::AtLaunch, |n| {
            std::thread::sleep(Duration::from_millis(60));
            Ok(Fake::new(&format!("e{n}")))
        });
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert!(rt.is_busy());
        assert_eq!(label(result(&mut rt)), "e0");
        assert!(!rt.is_busy() && !rt.needs_poll());
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_build_reports_once_then_fails_fast() {
        let (mut rt, _) = runtime(Mode::AtLaunch, |_| {
            std::thread::sleep(Duration::from_millis(40));
            Err("no such model".into())
        });
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert_eq!(result(&mut rt), Some(Err("no such model".into())));
        let started = Instant::now();
        assert!(matches!(rt.start(img()), StartOutcome::Unavailable(e) if e == "no such model"));
        assert!(started.elapsed() < Duration::from_millis(20));
    }

    #[test]
    fn new_model_clears_a_failure() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut rt = OcrRuntime::new(Mode::AtLaunch);
        rt.set_build(counting(&builds, |_| Err("broken".into())));
        assert!(wait_until(LONG, || {
            rt.collect_warm();
            rt.failed.is_some()
        }));
        rt.set_build(counting(&builds, |_| Ok(Fake::new("good"))));
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert_eq!(label(result(&mut rt)), "good");
    }

    #[test]
    fn cancel_interrupts_the_scan_and_keeps_the_engine() {
        let fake = Fake::new("e").slow(200, Duration::from_millis(5));
        let proto = fake.clone();
        let (mut rt, builds) = runtime(Mode::AtLaunch, move |_| Ok(proto.clone()));
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert!(wait_until(LONG, || {
            rt.poll();
            fake.calls() == 1
        }));
        std::thread::sleep(Duration::from_millis(30));

        let cancelled_at = Instant::now();
        rt.cancel();
        assert!(!rt.is_busy(), "a cancelled scan is not waited for");
        assert!(rt.needs_poll(), "but the engine still has to come back");
        assert!(drained(&mut rt));
        assert!(
            cancelled_at.elapsed() < Duration::from_millis(300),
            "the scan ran to its end"
        );
        assert_eq!(fake.cancels(), 1);
        assert!(rt.backend.is_some());
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn scan_started_while_a_cancelled_one_drains_runs_after_it() {
        let fake = Fake::new("e").slow(30, Duration::from_millis(5)).deaf();
        let proto = fake.clone();
        let (mut rt, builds) = runtime(Mode::AtLaunch, move |_| Ok(proto.clone()));
        rt.start(img());
        assert!(wait_until(LONG, || {
            rt.poll();
            fake.calls() == 1
        }));
        rt.cancel();
        assert!(matches!(
            rt.start(testing::image(9, 9)),
            StartOutcome::Started
        ));
        assert!(rt.is_busy());
        let text = result(&mut rt).unwrap().unwrap();
        assert_eq!(text.lines[0].text, "e 9x9");
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn switching_models_mid_scan_retires_the_old_engine() {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut rt = OcrRuntime::new(Mode::AtLaunch);
        let old = Fake::new("old").slow(20, Duration::from_millis(5)).deaf();
        let proto = old.clone();
        rt.set_build(counting(&builds, move |_| Ok(proto.clone())));
        rt.start(img());
        assert!(wait_until(LONG, || {
            rt.poll();
            old.calls() == 1
        }));

        rt.set_build(counting(&builds, |_| Ok(Fake::new("new"))));
        assert_eq!(label(result(&mut rt)), "old");
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert_eq!(label(result(&mut rt)), "new");
        assert_eq!(old.calls(), 1, "the old engine must not be reused");
    }

    #[test]
    fn panicking_scan_does_not_leave_an_endless_spinner() {
        let (mut rt, builds) = runtime(Mode::AtLaunch, |n| {
            Ok(if n == 0 {
                Fake::new("e0").panicking()
            } else {
                Fake::new(&format!("e{n}"))
            })
        });
        rt.start(img());
        assert!(matches!(result(&mut rt), Some(Err(e)) if e.contains("vanished")));
        assert!(!rt.is_busy());

        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert_eq!(label(result(&mut rt)), "e1");
        assert!(!rt.is_busy());
        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn on_demand_builds_nothing_until_asked() {
        let (mut rt, builds) = runtime(Mode::OnDemand, |_| Ok(Fake::new("e")));
        std::thread::sleep(Duration::from_millis(40));
        rt.poll();
        assert_eq!(builds.load(Ordering::SeqCst), 0);

        rt.prepare();
        assert!(wait_until(LONG, || builds.load(Ordering::SeqCst) == 1));
        for _ in 0..5 {
            rt.prepare();
            rt.poll();
        }
        assert!(wait_until(LONG, || {
            rt.collect_warm();
            rt.backend.is_some()
        }));
        rt.prepare();
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn on_demand_scan_builds_the_engine_itself() {
        let (mut rt, builds) = runtime(Mode::OnDemand, |_| Ok(Fake::new("e")));
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert_eq!(label(result(&mut rt)), "e");
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn prepare_does_not_rebuild_an_engine_that_is_scanning() {
        let fake = Fake::new("e").slow(20, Duration::from_millis(5));
        let proto = fake.clone();
        let (mut rt, builds) = runtime(Mode::OnDemand, move |_| Ok(proto.clone()));
        rt.start(img());
        assert!(wait_until(LONG, || {
            rt.poll();
            fake.calls() == 1
        }));
        rt.prepare();
        assert!(result(&mut rt).is_some());
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nothing_loaded_is_unavailable() {
        let mut rt = OcrRuntime::new(Mode::OnDemand);
        rt.prepare();
        assert!(matches!(rt.start(img()), StartOutcome::Unavailable(_)));
        assert!(!rt.is_busy() && !rt.needs_poll());
    }

    #[test]
    fn second_scan_while_busy_is_refused() {
        let (mut rt, _) = runtime(Mode::AtLaunch, |_| {
            Ok(Fake::new("e").slow(10, Duration::from_millis(5)))
        });
        assert!(matches!(rt.start(img()), StartOutcome::Started));
        assert!(matches!(rt.start(img()), StartOutcome::Busy));
        assert!(result(&mut rt).is_some());
    }
}
