use super::*;

#[test]
#[ignore = "entry point of the daemon processes started by the tests below"]
fn child_daemon() {
    let Some(dir) = std::env::var_os(CHILD_DIR).map(PathBuf::from) else {
        return;
    };
    let _ = fs::write(dir.join(format!("pid-{}", std::process::id())), "");
    if let Some(fd) = std::env::var(CHILD_FD)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
    {
        // SAFETY: Only queries flags for the provided descriptor number without opening or closing file handles.
        let open = unsafe { nix::libc::fcntl(fd, nix::libc::F_GETFD) } != -1;
        let _ = fs::write(dir.join("fd-inherited"), if open { "yes" } else { "no" });
    }
    let scan_ms: usize = std::env::var(CHILD_SCAN_MS)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let fake = Fake::new("child").slow(scan_ms / 5, Duration::from_millis(5));
    let factory = match std::env::var_os(CHILD_CRASH) {
        Some(_) => {
            Arc::new(|_: &ModelFiles| -> Result<Engine, BuildError> { std::process::abort() })
        }
        None => factory(fake),
    };
    let _ = server::serve(server::Config {
        idle: Some(Duration::from_secs(30)),
        ..config(&dir, factory)
    });
    std::process::exit(0);
}

#[test]
fn real_daemon_processes_do_not_stack() {
    let (dir, paths) = dir_and_paths("proc-stack");
    let probe = Probe::default();
    let spawn = child_spawner(&dir, 0, &probe);

    let clients: Vec<_> = (0..5)
        .map(|_| {
            let config = client_config(&paths, spawn.clone(), &probe);
            thread::spawn(move || {
                client::connect_with(config, &testing::files("m"))
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
        })
        .collect();
    for client in clients {
        client.join().unwrap().unwrap();
    }
    assert!(
        read_strikes(&paths).is_empty(),
        "a client gave up: {}",
        read_strikes(&paths)
    );
    assert!(
        wait_until(LONG, || children(&dir).len() == 1),
        "live daemons: {:?}",
        children(&dir)
    );
    let pid = ready_pid(&paths);
    assert_eq!(children(&dir), vec![pid]);

    assert_eq!(
        client::stop(&paths, Duration::from_secs(2), &client::kill_hard),
        Ok(true)
    );
    assert!(wait_until(LONG, || children(&dir).is_empty()));
}

#[test]
fn killed_daemon_is_survived_and_replaced() {
    let (dir, paths) = dir_and_paths("proc-kill");
    let probe = Probe::default();
    let spawn = child_spawner(&dir, 1500, &probe);
    let files = testing::files("m");

    let backend =
        client::connect_with(client_config(&paths, spawn.clone(), &probe), &files).unwrap();
    let pid = ready_pid(&paths);
    let killer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        kill(Pid::from_raw(pid), Signal::SIGKILL)
    });
    assert_eq!(read(&*backend), "local");
    killer.join().unwrap().unwrap();
    assert!(wait_until(LONG, || !alive(pid)));
    assert!(read_strikes(&paths).contains("failed"));

    let fresh = client::connect_with(client_config(&paths, spawn, &probe), &files).unwrap();
    let new_pid = ready_pid(&paths);
    assert_ne!(new_pid, pid);
    assert_eq!(read(&*fresh), "child");
    assert_eq!(probe.spawns(), 2);

    assert_eq!(
        client::stop(&paths, Duration::from_secs(2), &client::kill_hard),
        Ok(true)
    );
    assert!(wait_until(LONG, || !alive(new_pid)));
}

#[test]
fn hung_real_daemon_is_killed_by_the_watchdog() {
    let (dir, paths) = dir_and_paths("proc-hung");
    let probe = Probe::default();
    let config = client::Config {
        kill: Arc::new(client::kill_hard),
        timing: Timing {
            reply_base: Duration::from_millis(400),
            ..timing()
        },
        ..client_config(&paths, child_spawner(&dir, 20_000, &probe), &probe)
    };
    let backend = client::connect_with(config, &testing::files("m")).unwrap();
    let pid = ready_pid(&paths);

    let started = Instant::now();
    assert_eq!(read(&*backend), "local");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        wait_until(LONG, || !alive(pid)),
        "the hung daemon is still alive"
    );
    assert!(wait_until(LONG, || lock_free(&paths.lock)));
}

#[test]
fn daemon_that_dies_while_loading_is_not_respawned_forever() {
    let (dir, paths) = dir_and_paths("proc-crash");
    let probe = Probe::default();
    let spawns = probe.spawns.clone();
    let root = dir.to_path_buf();
    let spawn: Spawner = Arc::new(move || {
        spawns.fetch_add(1, Ordering::SeqCst);
        let mut command = child_command(&root, 0);
        command.env(CHILD_CRASH, "1");
        spawn_detached(command, child_log(&root))
    });
    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    for _ in 0..6 {
        assert!(wait_until(LONG, || children(&dir).is_empty()));
        assert_eq!(read(&*backend), "local");
    }
    assert_eq!(probe.spawns(), 1, "every scan started a daemon that died");
    assert!(read_strikes(&paths).contains("failed"));
    assert!(wait_until(LONG, || children(&dir).is_empty()));
}

#[test]
fn a_daemon_that_died_on_the_gpu_is_not_followed_onto_it() {
    if !crate::ocr::settings::GPU_BUILD {
        return;
    }
    let (_dir, paths) = dir_and_paths("gpu-marker");
    fs::write(paths.dir.join("ocr.gpu-loading"), "1").unwrap();
    let factory = super::super::engine_factory(crate::ocr::settings::Device::Gpu, paths.clone());
    assert!(matches!(
        factory(&testing::files("m")),
        Err(BuildError::NoGpu)
    ));
    assert!(!paths.dir.join("ocr.gpu-loading").exists());
}

#[test]
fn stop_kills_a_frozen_daemon() {
    let (dir, paths) = dir_and_paths("proc-frozen");
    let probe = Probe::default();
    (child_spawner(&dir, 0, &probe))().unwrap();
    assert!(wait_until(LONG, || client::status(&paths).is_some()));
    let pid = client::status(&paths).unwrap().pid as i32;

    kill(Pid::from_raw(pid), Signal::SIGSTOP).unwrap();
    assert!(
        client::status(&paths).is_none(),
        "a frozen daemon cannot answer"
    );
    assert_eq!(
        client::stop(&paths, Duration::from_millis(500), &client::kill_hard),
        Ok(true)
    );
    assert!(wait_until(LONG, || !alive(pid)));
}

#[test]
fn daemon_does_not_inherit_descriptors() {
    let (dir, paths) = dir_and_paths("proc-fd");
    let file = fs::File::open("/proc/self/stat").unwrap();
    // dup without CLOEXEC — simulates an fd left open by a third-party library
    let leaked = nix::unistd::dup(&file).unwrap();
    let mut command = child_command(&dir, 0);
    command.env(CHILD_FD, leaked.as_raw_fd().to_string());
    spawn_detached(command, child_log(&dir)).unwrap();

    let marker = dir.join("fd-inherited");
    assert!(wait_until(LONG, || fs::read_to_string(&marker)
        .is_ok_and(|s| !s.is_empty())));
    assert_eq!(fs::read_to_string(&marker).unwrap(), "no");
    assert!(wait_until(LONG, || client::status(&paths).is_some()));
    assert_eq!(
        client::stop(&paths, Duration::from_secs(2), &client::kill_hard),
        Ok(true)
    );
}
