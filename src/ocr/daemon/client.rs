// Daemon client implementation running as a standard `OcrBackend` on an `OcrRuntime` worker thread.
// asynchronously OFF the main thread to keep the UI responsive and independent
//
// Automatically falls back to the local engine on any connection issue, daemon crash,
// timeout, binary mismatch, or missing GPU support, ensuring text recognition always succeeds.
// Operates on a single-connection-per-request model.

use log::info;
use std::cell::{Cell, RefCell};
use std::io;
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::signal::{Signal, kill};
use nix::sys::socket::{getsockopt, sockopt};
use nix::unistd::Pid;

use super::protocol::{self, EngineState, Outgoing, PROTO, Reply, Status, Welcome};
use super::{
    Paths, Strike, build_id, clear_strikes, daemon_blocked, identity, lock_file_pid, lock_free,
    model_name, read_strikes, record_strike, spawn_daemon, unix_now, wait_lock_free,
};
use crate::ocr::models::ModelFiles;
use crate::ocr::settings::EngineSettings;
use crate::ocr::{CANCELLED, OcrBackend, OcrError, OcrImage, OcrText, default_backend};

pub type Spawner = Arc<dyn Fn() -> io::Result<()> + Send + Sync>;
pub type Killer = Arc<dyn Fn(i32) + Send + Sync>;
pub type Local = Arc<dyn Fn(&ModelFiles) -> Result<Box<dyn OcrBackend>, OcrError> + Send + Sync>;

#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub connect: Duration,
    pub retry: Duration,
    pub reply_base: Duration,
    pub reply_per_megapixel: Duration,
    pub slice: Duration,
    pub io: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(2),
            retry: Duration::from_millis(20),
            reply_base: Duration::from_secs(10),
            reply_per_megapixel: Duration::from_secs(3),
            slice: Duration::from_millis(50),
            io: Duration::from_secs(2),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub paths: Paths,
    pub build: String,
    pub spawn: Spawner,
    pub kill: Killer,
    pub local: Local,
    pub timing: Timing,
}

impl Config {
    pub fn system(paths: Paths) -> Self {
        Self {
            paths,
            build: identity(&EngineSettings::load()),
            spawn: Arc::new(spawn_daemon),
            kill: Arc::new(kill_hard),
            local: Arc::new(default_backend),
            timing: Timing::default(),
        }
    }
}

pub fn kill_hard(pid: i32) {
    // Received PID via socket or lockfile
    // ignore self-PID and non-existent process IDs.
    if pid <= 0 || u32::try_from(pid) == Ok(std::process::id()) {
        return;
    }
    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
}

pub fn connect(files: &ModelFiles) -> Result<Box<dyn OcrBackend>, OcrError> {
    match Paths::from_env() {
        Some(paths) => connect_with(Config::system(paths), files),
        None => default_backend(files),
    }
}

pub fn connect_with(config: Config, files: &ModelFiles) -> Result<Box<dyn OcrBackend>, OcrError> {
    if daemon_blocked(&read_strikes(&config.paths), &config.build, unix_now()) {
        return (config.local)(files);
    }
    let client = DaemonBackend {
        config,
        files: files.clone(),
        local: RefCell::new(None),
        broken: Cell::new(false),
        spawned: Cell::new(false),
    };
    match client.open() {
        Ok(session) => {
            info!(
                "ocr: connected to the OCR daemon (pid {}, {:?})",
                session.pid.map_or("?".to_owned(), |pid| pid.to_string()),
                session.welcome.state
            );
            client.ask_for_model(&session);
            Ok(Box::new(client))
        }
        Err(fail) => {
            client.strike(&fail);
            (client.config.local)(files)
        }
    }
}

pub struct DaemonBackend {
    config: Config,
    files: ModelFiles,
    local: RefCell<Option<Box<dyn OcrBackend>>>,
    broken: Cell<bool>,
    spawned: Cell<bool>,
}

struct Session {
    stream: UnixStream,
    welcome: Welcome,
    pid: Option<i32>,
}

enum Fail {
    Blocked,
    Unreachable(String),
    NoGpu,
    Failed(String),
    Protocol(String),
}

enum Remote {
    Done(OcrText),
    Cancelled,
    Local,
    Fail(Fail),
}

impl OcrBackend for DaemonBackend {
    fn recognize(
        &self,
        image: OcrImage,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OcrText, OcrError> {
        if !self.broken.get() {
            match self.remote(&image, cancelled) {
                Remote::Done(text) => {
                    info!("ocr: read by the daemon");
                    clear_strikes(&self.config.paths);
                    // if there is daemon, no need in additional local engine, dropping it
                    self.local.borrow_mut().take();
                    return Ok(text);
                }
                Remote::Cancelled => return Err(CANCELLED.into()),
                Remote::Local => {
                    info!("ocr: the daemon is not ready yet, reading in this process")
                }
                Remote::Fail(fail) => {
                    self.strike(&fail);
                    self.broken.set(true);
                }
            }
        } else {
            info!("ocr: reading in this process (no daemon)");
        }
        let mut local = self.local.borrow_mut();
        if local.is_none() {
            *local = Some((self.config.local)(&self.files)?);
        }
        local
            .as_ref()
            .expect("built above")
            .recognize(image, cancelled)
    }
}

impl DaemonBackend {
    fn open(&self) -> Result<Session, Fail> {
        match self.open_once() {
            Err(Fail::Protocol(_)) if lock_free(&self.config.paths.lock) => self.open_once(),
            result => result,
        }
    }

    fn open_once(&self) -> Result<Session, Fail> {
        let mut session = self.handshake()?;
        if self.foreign(&session.welcome) {
            info!(
                "ocr: the running daemon is from another build or settings ({}), replacing it",
                session.welcome.build
            );
            self.retire(session);
            session = self.handshake()?;
            if self.foreign(&session.welcome) {
                return Err(Fail::Protocol(
                    "the daemon is still from another build or settings".into(),
                ));
            }
        }
        match &session.welcome.state {
            EngineState::NoGpu => Err(Fail::NoGpu),
            EngineState::Failed(e) => Err(Fail::Failed(e.clone())),
            _ => Ok(session),
        }
    }

    fn foreign(&self, welcome: &Welcome) -> bool {
        welcome.proto != PROTO || welcome.build != self.config.build
    }

    fn handshake(&self) -> Result<Session, Fail> {
        let stream = self.dial()?;
        let io = Some(self.config.timing.io);
        stream
            .set_read_timeout(io)
            .and_then(|()| stream.set_write_timeout(io))
            .map_err(|e| Fail::Unreachable(e.to_string()))?;
        let pid = getsockopt(&stream, sockopt::PeerCredentials)
            .ok()
            .map(|cred| cred.pid());
        protocol::write_request(&mut &stream, Outgoing::Hello(&self.config.build))
            .map_err(|e| Fail::Protocol(format!("hello: {e}")))?;
        match protocol::read_reply(&mut &stream) {
            Ok(Reply::Welcome(welcome)) => Ok(Session {
                stream,
                welcome,
                pid,
            }),
            Ok(_) => Err(Fail::Protocol("unexpected answer to hello".into())),
            Err(e) => Err(Fail::Protocol(format!("hello: {e}"))),
        }
    }

    fn dial(&self) -> Result<UnixStream, Fail> {
        let socket = &self.config.paths.socket;
        match UnixStream::connect(socket) {
            Ok(stream) => return Ok(stream),
            // socket remains, but nobody is listening: the daemon we spawned has crashed. when GPU driver failed during load
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused && self.spawned.get() => {
                return Err(Fail::Unreachable(
                    "the daemon started for this session crashed".into(),
                ));
            }
            Err(e)
                if !matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                return Err(Fail::Unreachable(e.to_string()));
            }
            Err(_) => {}
        }
        if daemon_blocked(
            &read_strikes(&self.config.paths),
            &self.config.build,
            unix_now(),
        ) {
            return Err(Fail::Blocked);
        }
        self.spawned.set(true);
        info!("ocr: no daemon at {}, starting one", socket.display());
        (self.config.spawn)()
            .map_err(|e| Fail::Unreachable(format!("cannot start the daemon: {e}")))?;
        let deadline = Instant::now() + self.config.timing.connect;
        loop {
            std::thread::sleep(self.config.timing.retry);
            match UnixStream::connect(socket) {
                Ok(stream) => return Ok(stream),
                Err(e) if Instant::now() >= deadline => {
                    return Err(Fail::Unreachable(format!(
                        "the daemon did not come up: {e}"
                    )));
                }
                Err(_) => {}
            }
        }
    }

    fn retire(&self, session: Session) {
        let _ = protocol::write_request(&mut &session.stream, Outgoing::Shutdown);
        let _ = protocol::read_reply(&mut &session.stream);
        let Session { stream, pid, .. } = session;
        drop(stream);
        let lock = &self.config.paths.lock;
        if !wait_lock_free(lock, self.config.timing.connect) {
            if let Some(pid) = pid {
                (self.config.kill)(pid);
            }
            wait_lock_free(lock, self.config.timing.connect);
        }
    }

    /// Returns `true` if the daemon is already hosting our target model; otherwise, requests it asynchronously without blocking.
    fn ask_for_model(&self, session: &Session) -> bool {
        let ours = model_name(&self.files);
        match &session.welcome.state {
            EngineState::Ready { model, .. } if *model == ours => true,
            EngineState::Loading { model } if *model == ours => false,
            _ => {
                if protocol::write_request(&mut &session.stream, Outgoing::Load(&self.files))
                    .is_ok()
                {
                    let _ = protocol::read_reply(&mut &session.stream);
                }
                false
            }
        }
    }

    fn remote(&self, image: &OcrImage, cancelled: &dyn Fn() -> bool) -> Remote {
        let session = match self.open() {
            Ok(session) => session,
            Err(fail) => return Remote::Fail(fail),
        };
        if !self.ask_for_model(&session) {
            // demon is not ready for our model, using local then
            return Remote::Local;
        }
        if let Err(e) = protocol::write_request(&mut &session.stream, Outgoing::Recognize(image)) {
            return Remote::Fail(Fail::Protocol(format!("sending the image: {e}")));
        }

        let timing = self.config.timing;
        let megapixels = f64::from(image.width) * f64::from(image.height) / 1e6;
        let budget = timing.reply_base + timing.reply_per_megapixel.mul_f64(megapixels);
        let deadline = Instant::now() + budget;
        loop {
            if cancelled() {
                return Remote::Cancelled;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                if let Some(pid) = session.pid {
                    (self.config.kill)(pid);
                }
                return Remote::Fail(Fail::Protocol(format!(
                    "no answer in {budget:?}, the daemon was killed"
                )));
            }
            let slice = PollTimeout::try_from(left.min(timing.slice)).unwrap_or(PollTimeout::MAX);
            let mut fds = [PollFd::new(session.stream.as_fd(), PollFlags::POLLIN)];
            match poll(&mut fds, slice) {
                Ok(0) | Err(Errno::EINTR) => {}
                Ok(_) => break,
                Err(e) => return Remote::Fail(Fail::Protocol(e.to_string())),
            }
        }

        match protocol::read_reply(&mut &session.stream) {
            Ok(Reply::Lines(text)) => Remote::Done(text),
            Ok(Reply::Error(e)) => {
                info!("ocr: the daemon could not read the image ({e}), reading it here");
                Remote::Local
            }
            Ok(_) => Remote::Fail(Fail::Protocol("unexpected answer to recognize".into())),
            Err(e) => Remote::Fail(Fail::Protocol(format!("the daemon connection broke: {e}"))),
        }
    }

    fn strike(&self, fail: &Fail) {
        let (strike, reason) = match fail {
            Fail::Blocked => {
                info!("ocr: the daemon failed here recently, OCR runs in this process");
                return;
            }
            Fail::NoGpu => (Strike::NoGpu, "no GPU".to_owned()),
            Fail::Unreachable(e) | Fail::Failed(e) | Fail::Protocol(e) => {
                (Strike::Failed, e.clone())
            }
        };
        info!("ocr: daemon unusable ({reason}), OCR runs in this process");
        record_strike(&self.config.paths, &self.config.build, strike);
    }
}

fn plain_session(paths: &Paths, io: Duration) -> Option<(UnixStream, Option<i32>)> {
    let stream = UnixStream::connect(&paths.socket).ok()?;
    stream.set_read_timeout(Some(io)).ok()?;
    stream.set_write_timeout(Some(io)).ok()?;
    let pid = getsockopt(&stream, sockopt::PeerCredentials)
        .ok()
        .map(|cred| cred.pid());
    protocol::write_request(&mut &stream, Outgoing::Hello(build_id())).ok()?;
    matches!(protocol::read_reply(&mut &stream), Ok(Reply::Welcome(_))).then_some((stream, pid))
}

pub fn status(paths: &Paths) -> Option<Status> {
    let (stream, _) = plain_session(paths, Duration::from_secs(2))?;
    protocol::write_request(&mut &stream, Outgoing::Status).ok()?;
    match protocol::read_reply(&mut &stream) {
        Ok(Reply::Status(status)) => Some(status),
        _ => None,
    }
}

/// Returns `Ok(true)` if the daemon was running and successfully terminated (lock is now free)
/// returns `Ok(false)` if no active daemon was found.
pub fn stop(paths: &Paths, wait: Duration, kill: &dyn Fn(i32)) -> Result<bool, String> {
    let Some((stream, pid)) = plain_session(paths, wait) else {
        if lock_free(&paths.lock) {
            return Ok(false);
        }
        let pid = lock_file_pid(paths)
            .ok_or("the daemon holds its lock, does not answer and left no pid")?;
        kill(pid);
        return if wait_lock_free(&paths.lock, wait) {
            Ok(true)
        } else {
            Err(format!("pid {pid} did not exit"))
        };
    };
    let _ = protocol::write_request(&mut &stream, Outgoing::Shutdown);
    let _ = protocol::read_reply(&mut &stream);
    drop(stream);
    if wait_lock_free(&paths.lock, wait) {
        return Ok(true);
    }
    if let Some(pid) = pid {
        kill(pid);
    }
    if wait_lock_free(&paths.lock, wait) {
        Ok(true)
    } else {
        Err("the daemon did not exit".into())
    }
}
