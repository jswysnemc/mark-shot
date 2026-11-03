// Entire daemon test suite: in-memory server running in a background thread using temporary directories, fake engines, and mock daemons,
// along with integration tests using real processes (where the test binary re-executes itself as a background daemon).

use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use super::client::{self, Spawner, Timing};
use super::protocol::{self, EngineState, Outgoing, PROTO, Reply, Request, Welcome};
use super::server::{self, BuildError, Engine, Factory, Outcome};
use super::{
    Paths, Strike, daemon_blocked, gpu_ruled_out, lock_free, model_name, read_strikes,
    record_strike, spawn_detached, unix_now,
};
use crate::ocr::models::ModelFiles;
use crate::ocr::testing::{self, Fake, label_of, wait_until};
use crate::ocr::{CANCELLED, OcrBackend};

const BUILD: &str = "test-build";
const LONG: Duration = Duration::from_secs(10);

type Server = JoinHandle<Result<Outcome, String>>;
type Servers = Arc<Mutex<Vec<Server>>>;

fn never() -> bool {
    false
}

fn factory(fake: Fake) -> Factory {
    Arc::new(move |_: &ModelFiles| {
        Ok(Engine {
            backend: Box::new(fake.clone()),
            device: "fake".into(),
        })
    })
}

fn delayed(fake: Fake, delay: Duration) -> Factory {
    Arc::new(move |_: &ModelFiles| {
        thread::sleep(delay);
        Ok(Engine {
            backend: Box::new(fake.clone()),
            device: "fake".into(),
        })
    })
}

fn config(dir: &Path, factory: Factory) -> server::Config {
    server::Config {
        io_timeout: Duration::from_millis(300),
        unusable_grace: Duration::from_millis(200),
        lock_wait: Duration::from_millis(100),
        ..server::Config::new(
            Paths::in_dir(dir.to_path_buf()),
            BUILD.into(),
            Some(Duration::from_secs(60)),
            factory,
        )
    }
}

fn start(config: server::Config) -> Server {
    let socket = config.paths.socket.clone();
    let server = thread::spawn(move || server::serve(config));
    assert!(
        wait_until(LONG, || UnixStream::connect(&socket).is_ok()),
        "the server did not come up"
    );
    server
}

fn finish(server: Server) -> Outcome {
    assert!(
        wait_until(LONG, || server.is_finished()),
        "the server did not leave"
    );
    server.join().unwrap().unwrap()
}

fn finish_all(servers: &Servers) -> Vec<Outcome> {
    let drained: Vec<Server> = servers.lock().unwrap().drain(..).collect();
    drained.into_iter().map(finish).collect()
}

struct Conn(UnixStream);

impl Conn {
    fn open(paths: &Paths) -> Self {
        let stream = UnixStream::connect(&paths.socket).unwrap();
        stream.set_read_timeout(Some(LONG)).unwrap();
        Self(stream)
    }

    fn greeted(paths: &Paths) -> (Self, Welcome) {
        let conn = Self::open(paths);
        match conn.ask(Outgoing::Hello(BUILD)) {
            Reply::Welcome(welcome) => (conn, welcome),
            other => panic!("expected welcome, got {other:?}"),
        }
    }

    fn send(&self, request: Outgoing) {
        protocol::write_request(&mut &self.0, request).unwrap();
    }

    fn reply(&self) -> Reply {
        protocol::read_reply(&mut &self.0).unwrap()
    }

    fn ask(&self, request: Outgoing) -> Reply {
        self.send(request);
        self.reply()
    }
}

/// Does not panic: the daemon might be in the middle of shutting down.
fn state(paths: &Paths) -> Option<EngineState> {
    let stream = UnixStream::connect(&paths.socket).ok()?;
    stream.set_read_timeout(Some(LONG)).ok()?;
    protocol::write_request(&mut &stream, Outgoing::Hello(BUILD)).ok()?;
    match protocol::read_reply(&mut &stream).ok()? {
        Reply::Welcome(welcome) => Some(welcome.state),
        _ => None,
    }
}

fn load(paths: &Paths, files: &ModelFiles) {
    assert_eq!(Conn::greeted(paths).0.ask(Outgoing::Load(files)), Reply::Ok);
}

fn wait_ready(paths: &Paths) {
    assert!(
        wait_until(LONG, || matches!(
            state(paths),
            Some(EngineState::Ready { .. })
        )),
        "the engine never became ready: {:?}",
        state(paths)
    );
}

fn shutdown(paths: &Paths) {
    if let Ok(stream) = UnixStream::connect(&paths.socket) {
        let _ = stream.set_read_timeout(Some(LONG));
        let _ = protocol::write_request(&mut &stream, Outgoing::Shutdown);
        let _ = protocol::read_reply(&mut &stream);
    }
}

fn dir_and_paths(tag: &str) -> (testing::TempDir, Paths) {
    let dir = testing::temp_dir(tag);
    let paths = Paths::in_dir(dir.to_path_buf());
    (dir, paths)
}

fn img() -> crate::ocr::OcrImage {
    testing::image(8, 6)
}

#[derive(Default, Clone)]
struct Probe {
    spawns: Arc<AtomicUsize>,
    locals: Arc<AtomicUsize>,
    kills: Arc<Mutex<Vec<i32>>>,
}

impl Probe {
    fn spawns(&self) -> usize {
        self.spawns.load(Ordering::SeqCst)
    }

    fn locals(&self) -> usize {
        self.locals.load(Ordering::SeqCst)
    }

    fn kills(&self) -> Vec<i32> {
        self.kills.lock().unwrap().clone()
    }
}

fn timing() -> Timing {
    Timing {
        connect: Duration::from_secs(3),
        retry: Duration::from_millis(5),
        reply_base: Duration::from_secs(5),
        reply_per_megapixel: Duration::ZERO,
        slice: Duration::from_millis(10),
        io: Duration::from_secs(3),
    }
}

fn client_config(paths: &Paths, spawn: Spawner, probe: &Probe) -> client::Config {
    let (locals, kills) = (probe.locals.clone(), probe.kills.clone());
    client::Config {
        paths: paths.clone(),
        build: BUILD.into(),
        spawn,
        kill: Arc::new(move |pid| kills.lock().unwrap().push(pid)),
        local: Arc::new(move |_: &ModelFiles| {
            locals.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(Fake::new("local")) as Box<dyn OcrBackend>)
        }),
        timing: timing(),
    }
}

fn thread_spawner(dir: &Path, factory: Factory, probe: &Probe, servers: &Servers) -> Spawner {
    let (dir, spawns, servers) = (dir.to_path_buf(), probe.spawns.clone(), servers.clone());
    Arc::new(move || {
        spawns.fetch_add(1, Ordering::SeqCst);
        let config = config(&dir, factory.clone());
        servers
            .lock()
            .unwrap()
            .push(thread::spawn(move || server::serve(config)));
        Ok(())
    })
}

fn refusing_spawner(probe: &Probe) -> Spawner {
    let spawns = probe.spawns.clone();
    Arc::new(move || {
        spawns.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::other("spawning is not expected here"))
    })
}

fn read(backend: &dyn OcrBackend) -> String {
    label_of(&backend.recognize(img(), &never).unwrap())
}

/// Mock "daemon": accepts `n` incoming connections and executes prescribed scenario steps for each.
fn fake_daemon(
    paths: &Paths,
    connections: usize,
    script: impl Fn(UnixStream) + Send + 'static,
) -> JoinHandle<()> {
    fs::create_dir_all(&paths.dir).unwrap();
    let listener = UnixListener::bind(&paths.socket).unwrap();
    thread::spawn(move || {
        for stream in listener.incoming().take(connections) {
            script(stream.unwrap());
        }
    })
}

fn answer_hello(mut stream: &UnixStream, state: EngineState) {
    assert!(matches!(
        protocol::read_request(&mut stream),
        Ok(Request::Hello { .. })
    ));
    let welcome = Welcome {
        proto: PROTO,
        build: BUILD.into(),
        state,
    };
    protocol::write_reply(&mut stream, &Reply::Welcome(welcome)).unwrap();
}

fn ready(files: &ModelFiles) -> EngineState {
    EngineState::Ready {
        device: "fake".into(),
        model: model_name(files),
    }
}

// ---------------------------------------------------------------- server

// ---------------------------------------------------------------- client

// ---------------------------------------------------------------- real processes

const CHILD_DIR: &str = "MARK_SHOT_TEST_DAEMON_DIR";
const CHILD_SCAN_MS: &str = "MARK_SHOT_TEST_DAEMON_SCAN_MS";
const CHILD_FD: &str = "MARK_SHOT_TEST_DAEMON_FD";
const CHILD_CRASH: &str = "MARK_SHOT_TEST_DAEMON_CRASH";

fn child_command(dir: &Path, scan_ms: u64) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "ocr::daemon::tests::process_tests::child_daemon",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_DIR, dir)
        .env(CHILD_SCAN_MS, scan_ms.to_string());
    command
}

fn child_log(dir: &Path) -> Option<fs::File> {
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("child.log"))
        .ok()
}

fn child_spawner(dir: &Path, scan_ms: u64, probe: &Probe) -> Spawner {
    let (dir, spawns) = (dir.to_path_buf(), probe.spawns.clone());
    Arc::new(move || {
        spawns.fetch_add(1, Ordering::SeqCst);
        spawn_detached(child_command(&dir, scan_ms), child_log(&dir))
    })
}

fn alive(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(')')
            .is_some_and(|(_, rest)| !rest.trim_start().starts_with('Z'))
    })
}

fn children(dir: &Path) -> Vec<i32> {
    let mut pids: Vec<i32> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_prefix("pid-")?
                .parse()
                .ok()
        })
        .filter(|&pid| alive(pid))
        .collect();
    pids.sort_unstable();
    pids
}

fn ready_pid(paths: &Paths) -> i32 {
    assert!(wait_until(LONG, || {
        client::status(paths)
            .is_some_and(|status| matches!(status.state, EngineState::Ready { .. }))
    }));
    client::status(paths).unwrap().pid as i32
}

use super::calibrate::{self, Record, Verdict};

fn measured(verdict: Verdict, cpu_ms: Option<u32>, gpu_ms: Option<u32>) -> Record {
    Record {
        verdict,
        cpu_ms,
        gpu_ms,
        at: 1_700_000_000,
    }
}

fn gpu_traces() -> Vec<String> {
    let maps = fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let mut traces: Vec<String> = maps
        .lines()
        .filter_map(|line| line.split_whitespace().nth(5))
        .filter(|lib| {
            [
                "libwebgpu_dawn",
                "libvulkan",
                "libnvidia",
                "libGLX",
                "libEGL",
                "libdrm",
                "_dri",
            ]
            .iter()
            .any(|name| lib.contains(name))
        })
        .map(str::to_owned)
        .collect();
    for fd in fs::read_dir("/proc/self/fd")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        if let Ok(target) = fs::read_link(fd.path())
            && (target.starts_with("/dev/dri")
                || target.to_string_lossy().starts_with("/dev/nvidia"))
        {
            traces.push(target.display().to_string());
        }
    }
    traces.sort();
    traces.dedup();
    traces
}

fn read_probe(gpu: bool) -> Vec<String> {
    let models = crate::ocr::models::OcrModels::load();
    let files = models
        .active()
        .and_then(|idx| models.files(idx))
        .expect("install an OCR model first");
    let backend: Box<dyn OcrBackend> = if gpu {
        Box::new(crate::ocr::paddle_backend::PaddleBackend::on_gpu(&files).expect("gpu engine"))
    } else {
        crate::ocr::build_backend(crate::ocr::settings::Mode::OnDemand, &files)
            .expect("local engine")
    };
    backend
        .recognize(super::calibrate::probe_image(), &never)
        .expect("scan");
    gpu_traces()
}

mod calibration_tests;
mod client_tests;
mod process_tests;
mod resources_tests;
mod server_tests;
