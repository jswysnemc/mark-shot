// The daemon process itself.
// Main thread handles file locking (`flock`), the UNIX socket, `accept()`, and the idle timer,
// sleeping inside `poll` without tick loops.
//
// Each incoming connection spawns a lightweight thread configured with read/write timeouts to
// ensure idle clients do not hold resources.
//
// Recognition scans are executed sequentially (protected by an engine `Mutex`), while `Hello`
// and `Status` requests always respond immediately, even during active scanning.

use log::info;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

use super::protocol::{self, EngineState, PROTO, Reply, Request, Status, Welcome};
use super::{Paths, Strike, model_name, record_strike};
use crate::ocr::models::ModelFiles;
use crate::ocr::{OcrBackend, OcrImage};

pub struct Engine {
    pub backend: Box<dyn OcrBackend>,
    pub device: String,
}

pub enum BuildError {
    NoGpu,
    Failed(String),
}

pub type Factory = Arc<dyn Fn(&ModelFiles) -> Result<Engine, BuildError> + Send + Sync>;

pub struct Config {
    pub paths: Paths,
    pub build: String,
    pub idle: Option<Duration>,
    pub factory: Factory,
    pub io_timeout: Duration,
    pub unusable_grace: Duration,
    pub lock_wait: Duration,
}

impl Config {
    pub fn new(paths: Paths, build: String, idle: Option<Duration>, factory: Factory) -> Self {
        Self {
            paths,
            build,
            idle,
            factory,
            io_timeout: Duration::from_secs(5),
            unusable_grace: Duration::from_secs(2),
            // A terminating daemon releases its lock within milliseconds; wait briefly rather than losing the lock race immediately.
            lock_wait: Duration::from_millis(500),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    AlreadyRunning,
    Idle,
    Shutdown,
    Replaced,
    Unusable,
}

struct Shared {
    paths: Paths,
    build: String,
    factory: Factory,
    idle: Option<Duration>,
    grace: Duration,
    io_timeout: Duration,
    started: Instant,
    state: Mutex<EngineState>,
    wanted: Mutex<Option<ModelFiles>>,
    generation: AtomicU64,
    engine: Mutex<Option<Box<dyn OcrBackend>>>,
    activity: Mutex<Instant>,
    busy: AtomicUsize,
    connections: AtomicUsize,
    quit: AtomicBool,
    served: AtomicU64,
    last_ms: AtomicU32,
    wake: UnixStream,
}

enum Deadline {
    Never,
    In(Duration),
}

pub fn serve(config: Config) -> Result<Outcome, String> {
    let Config {
        paths,
        build,
        idle,
        factory,
        io_timeout,
        unusable_grace,
        lock_wait,
    } = config;

    create_private_dir(&paths.dir)
        .map_err(|e| format!("cannot create {}: {e}", paths.dir.display()))?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&paths.lock)
        .map_err(|e| format!("cannot open {}: {e}", paths.lock.display()))?;
    let Some(lock) = take_lock(file, lock_wait) else {
        return Ok(Outcome::AlreadyRunning);
    };
    let _ = lock
        .set_len(0)
        .and_then(|()| writeln!(&*lock, "{}", std::process::id()));

    // Holding the lock guarantees the socket is unowned
    // any existing file is a leftover from a killed daemon instance.
    match fs::remove_file(&paths.socket) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("cannot remove the old socket: {e}")),
    }
    let listener = UnixListener::bind(&paths.socket)
        .map_err(|e| format!("cannot listen on {}: {e}", paths.socket.display()))?;
    let _ = fs::set_permissions(&paths.socket, fs::Permissions::from_mode(0o600));
    let (wake_rx, wake) = UnixStream::pair().map_err(|e| e.to_string())?;
    for nonblocking in [
        listener.set_nonblocking(true),
        wake_rx.set_nonblocking(true),
        wake.set_nonblocking(true),
    ] {
        nonblocking.map_err(|e| e.to_string())?;
    }

    let shared = Arc::new(Shared {
        paths: paths.clone(),
        build,
        factory,
        idle,
        grace: unusable_grace,
        io_timeout,
        started: Instant::now(),
        state: Mutex::new(EngineState::Idle),
        wanted: Mutex::new(None),
        generation: AtomicU64::new(0),
        engine: Mutex::new(None),
        activity: Mutex::new(Instant::now()),
        busy: AtomicUsize::new(0),
        connections: AtomicUsize::new(0),
        quit: AtomicBool::new(false),
        served: AtomicU64::new(0),
        last_ms: AtomicU32::new(0),
        wake,
    });
    info!(
        "ocr-daemon: pid {} listening on {}",
        std::process::id(),
        paths.socket.display()
    );

    let outcome = loop {
        if shared.quit.load(Ordering::SeqCst) {
            break Outcome::Shutdown;
        }
        if binary_replaced() {
            break Outcome::Replaced;
        }
        let timeout = match shared.deadline() {
            Deadline::Never => PollTimeout::NONE,
            Deadline::In(left) if left.is_zero() => {
                if shared.connections.load(Ordering::SeqCst) == 0 {
                    break if shared.unusable() {
                        Outcome::Unusable
                    } else {
                        Outcome::Idle
                    };
                }
                PollTimeout::try_from(Duration::from_millis(50)).unwrap_or(PollTimeout::MAX)
            }
            // Add 1 ms: `poll` rounds timeouts down and would otherwise spin redundantly on the final millisecond.
            Deadline::In(left) => {
                PollTimeout::try_from(left + Duration::from_millis(1)).unwrap_or(PollTimeout::MAX)
            }
        };
        let mut fds = [
            PollFd::new(listener.as_fd(), PollFlags::POLLIN),
            PollFd::new(wake_rx.as_fd(), PollFlags::POLLIN),
        ];
        if let Err(e) = poll(&mut fds, timeout)
            && e != Errno::EINTR
        {
            info!("ocr-daemon: poll failed: {e}");
            break Outcome::Shutdown;
        }
        let mut sink = [0u8; 64];
        while matches!((&wake_rx).read(&mut sink), Ok(n) if n > 0) {}
        accept_all(&listener, &shared);
    };

    let _ = fs::remove_file(&paths.socket);
    drop(lock);
    info!("ocr-daemon: leaving ({outcome:?})");
    Ok(outcome)
}

impl Shared {
    fn deadline(&self) -> Deadline {
        if self.busy.load(Ordering::SeqCst) > 0 {
            return Deadline::Never;
        }
        let limit = if self.unusable() {
            Some(self.grace)
        } else {
            self.idle
        };
        match limit {
            None => Deadline::Never,
            Some(limit) => Deadline::In(limit.saturating_sub(lock(&self.activity).elapsed())),
        }
    }

    fn unusable(&self) -> bool {
        matches!(
            *lock(&self.state),
            EngineState::Failed(_) | EngineState::NoGpu
        )
    }

    fn touch(&self) {
        *lock(&self.activity) = Instant::now();
    }

    fn wake(&self) {
        let _ = (&self.wake).write(&[1]);
    }

    fn status(&self) -> Status {
        let busy = self.busy.load(Ordering::SeqCst) > 0;
        let idle_left_secs = match self.deadline() {
            Deadline::In(left) => Some(left.as_secs()),
            Deadline::Never => self.idle.filter(|_| busy).map(|limit| limit.as_secs()),
        };
        Status {
            pid: std::process::id(),
            uptime_secs: self.started.elapsed().as_secs(),
            state: lock(&self.state).clone(),
            served: self.served.load(Ordering::SeqCst),
            last_ms: self.last_ms.load(Ordering::SeqCst),
            rss_kb: rss_kb(),
            idle_left_secs,
        }
    }
}

fn accept_all(listener: &UnixListener, shared: &Arc<Shared>) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                shared.connections.fetch_add(1, Ordering::SeqCst);
                let owned = shared.clone();
                let spawned = std::thread::Builder::new()
                    .name("ocr-conn".into())
                    .spawn(move || handle(owned, stream));
                if let Err(e) = spawned {
                    shared.connections.fetch_sub(1, Ordering::SeqCst);
                    info!("ocr-daemon: cannot serve a connection: {e}");
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => {
                info!("ocr-daemon: accept failed: {e}");
                // prevent spinning inside `poll` if file descriptors are exhausted, for instance.
                std::thread::sleep(Duration::from_millis(100));
                return;
            }
        }
    }
}

fn handle(shared: Arc<Shared>, stream: UnixStream) {
    let _open = Open(&shared);
    let timeout = Some(shared.io_timeout);
    let configured = stream
        .set_nonblocking(false)
        .and_then(|()| stream.set_read_timeout(timeout))
        .and_then(|()| stream.set_write_timeout(timeout));
    if configured.is_err() {
        return;
    }
    let mut reader = BufReader::new(&stream);
    let mut greeted = false;
    loop {
        let Ok(request) = protocol::read_request(&mut reader) else {
            return;
        };
        let reply = match request {
            Request::Hello { .. } => {
                greeted = true;
                Reply::Welcome(Welcome {
                    proto: PROTO,
                    build: shared.build.clone(),
                    state: lock(&shared.state).clone(),
                })
            }
            Request::Shutdown => {
                shared.quit.store(true, Ordering::SeqCst);
                shared.wake();
                Reply::Ok
            }
            // Require `Hello` before performing any operation other than `Shutdown`
            // rejects loading or recognition attempts on uninitialized connections.
            _ if !greeted => return,
            Request::Status => Reply::Status(shared.status()),
            Request::Load(files) => {
                load(&shared, files);
                Reply::Ok
            }
            Request::Recognize(image) => recognize(&shared, stream.as_fd(), image),
        };
        if protocol::write_reply(&mut &stream, &reply).is_err() {
            return;
        }
    }
}

fn load(shared: &Arc<Shared>, files: ModelFiles) {
    let generation = {
        let mut wanted = lock(&shared.wanted);
        let failed = matches!(*lock(&shared.state), EngineState::Failed(_));
        if wanted.as_ref() == Some(&files) && !failed {
            return;
        }
        *wanted = Some(files.clone());
        *lock(&shared.state) = EngineState::Loading {
            model: model_name(&files),
        };
        shared.generation.fetch_add(1, Ordering::SeqCst) + 1
    };
    shared.touch();

    let shared = shared.clone();
    std::thread::spawn(move || {
        // Drop the old engine before constructing the new one to prevent peak memory duplication.
        let old = lock(&shared.engine).take();
        drop(old);
        let started = Instant::now();
        let built =
            catch_unwind(AssertUnwindSafe(|| (shared.factory)(&files))).unwrap_or_else(|_| {
                Err(BuildError::Failed(
                    "the engine panicked while loading".into(),
                ))
            });

        let wanted = lock(&shared.wanted);
        // Discard build results if a different model was requested while construction was in progress.
        if shared.generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let state = match built {
            Ok(engine) => {
                *lock(&shared.engine) = Some(engine.backend);
                EngineState::Ready {
                    device: engine.device,
                    model: model_name(&files),
                }
            }
            Err(BuildError::NoGpu) => {
                // client might not catch this state before the daemon exits
                // so we remember it here to prevent repeated attempts to load a GPU engine
                record_strike(&shared.paths, &shared.build, Strike::NoGpu);
                EngineState::NoGpu
            }
            Err(BuildError::Failed(e)) => EngineState::Failed(e),
        };
        info!(
            "ocr-daemon: engine {state:?} after {} ms",
            started.elapsed().as_millis()
        );
        *lock(&shared.state) = state;
        drop(wanted);
        shared.touch();
        shared.wake();
    });
}

fn recognize(shared: &Shared, peer: BorrowedFd, image: OcrImage) -> Reply {
    let _busy = Busy::enter(shared);
    let mut engine = lock(&shared.engine);
    let ready = matches!(*lock(&shared.state), EngineState::Ready { .. });
    let Some(backend) = engine.as_ref().filter(|_| ready) else {
        return Reply::Error("the engine is not ready".into());
    };
    let started = Instant::now();
    let reply = match catch_unwind(AssertUnwindSafe(|| {
        backend.recognize(image, &|| peer_gone(peer))
    })) {
        Ok(Ok(text)) => {
            shared.served.fetch_add(1, Ordering::SeqCst);
            let ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
            shared.last_ms.store(ms, Ordering::SeqCst);
            Reply::Lines(text)
        }
        Ok(Err(e)) => Reply::Error(e.to_string()),
        Err(_) => {
            // Drop engine instance following a panic. Mark as failed so the daemon exits during its grace period.
            *engine = None;
            *lock(&shared.state) = EngineState::Failed("the engine crashed".into());
            Reply::Error("the engine crashed".into())
        }
    };
    // Reclaim memory freed from thread arenas by calling glibc memory trim on long-running daemons.
    unsafe { nix::libc::malloc_trim(0) };
    reply
}

struct Open<'a>(&'a Shared);

impl Drop for Open<'_> {
    fn drop(&mut self) {
        self.0.connections.fetch_sub(1, Ordering::SeqCst);
        self.0.wake();
    }
}

struct Busy<'a>(&'a Shared);

impl<'a> Busy<'a> {
    fn enter(shared: &'a Shared) -> Self {
        shared.busy.fetch_add(1, Ordering::SeqCst);
        Self(shared)
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.busy.fetch_sub(1, Ordering::SeqCst);
        self.0.touch();
        self.0.wake();
    }
}

/// Returns `true` if the client disconnected and closed the socket, rendering further scanning unnecessary.
fn peer_gone(fd: BorrowedFd) -> bool {
    use nix::libc::{POLLERR, POLLHUP, POLLRDHUP, poll as raw_poll, pollfd};
    let mut pfd = pollfd {
        fd: fd.as_raw_fd(),
        events: POLLRDHUP,
        revents: 0,
    };
    let ready = unsafe { raw_poll(&mut pfd, 1, 0) };
    ready > 0 && pfd.revents & (POLLRDHUP | POLLHUP | POLLERR) != 0
}

fn take_lock(mut file: File, wait: Duration) -> Option<Flock<File>> {
    let deadline = Instant::now() + wait;
    loop {
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => return Some(lock),
            Err((back, _)) if Instant::now() < deadline => {
                file = back;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

/// Checks if the underlying binary was recompiled or modified on disk to prevent obsolete processes from handling new client requests.
fn binary_replaced() -> bool {
    fs::read_link("/proc/self/exe")
        .is_ok_and(|exe| exe.as_os_str().as_bytes().ends_with(b" (deleted)"))
}

fn rss_kb() -> u64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("VmRSS:"))
                .and_then(|value| value.trim().trim_end_matches("kB").trim().parse().ok())
        })
        .unwrap_or(0)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
