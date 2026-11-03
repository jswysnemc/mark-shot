use super::*;

#[test]
fn second_daemon_leaves_the_first_one_serving() {
    let (dir, paths) = dir_and_paths("second");
    let first = start(config(&dir, factory(Fake::new("d"))));
    let second = thread::spawn({
        let config = config(&dir, factory(Fake::new("d")));
        move || server::serve(config)
    });
    assert_eq!(finish(second), Outcome::AlreadyRunning);
    assert_eq!(Conn::greeted(&paths).1.build, BUILD);

    shutdown(&paths);
    assert_eq!(finish(first), Outcome::Shutdown);
    assert!(!paths.socket.exists());
    assert!(lock_free(&paths.lock));
}

#[test]
fn socket_left_by_a_dead_daemon_is_replaced() {
    let (dir, paths) = dir_and_paths("stale");
    drop(UnixListener::bind(&paths.socket).unwrap());
    assert!(paths.socket.exists());
    assert!(UnixStream::connect(&paths.socket).is_err());

    let server = start(config(&dir, factory(Fake::new("d"))));
    assert_eq!(state(&paths), Some(EngineState::Idle));
    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn silent_client_is_dropped_without_blocking_others() {
    let (dir, paths) = dir_and_paths("silent");
    let server = start(config(&dir, factory(Fake::new("d"))));
    let silent = UnixStream::connect(&paths.socket).unwrap();
    silent.set_read_timeout(Some(LONG)).unwrap();

    let started = Instant::now();
    Conn::greeted(&paths);
    assert!(started.elapsed() < Duration::from_millis(250));
    let mut byte = [0u8; 1];
    assert_eq!(
        (&silent).read(&mut byte).unwrap(),
        0,
        "the silent connection should be closed"
    );
    assert!(started.elapsed() < Duration::from_secs(3));

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn garbage_and_requests_before_hello_close_only_that_connection() {
    let (dir, paths) = dir_and_paths("garbage");
    let server = start(config(&dir, factory(Fake::new("d"))));

    let junk = UnixStream::connect(&paths.socket).unwrap();
    junk.set_read_timeout(Some(LONG)).unwrap();
    (&junk)
        .write_all(b"GET / HTTP/1.1\r\nHost: lumine\r\n\r\n")
        .unwrap();
    let mut buf = [0u8; 16];
    assert!(matches!((&junk).read(&mut buf), Ok(0) | Err(_)));

    let rude = Conn::open(&paths);
    rude.send(Outgoing::Load(&testing::files("m")));
    assert!(protocol::read_reply(&mut &rude.0).is_err());
    assert_eq!(
        state(&paths),
        Some(EngineState::Idle),
        "a load without hello must be ignored"
    );

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn client_that_leaves_cancels_its_scan() {
    let (dir, paths) = dir_and_paths("leave");
    let fake = Fake::new("d").slow(60, Duration::from_millis(5));
    let server = start(config(&dir, factory(fake.clone())));
    load(&paths, &testing::files("m"));
    wait_ready(&paths);

    let (scan, _) = Conn::greeted(&paths);
    scan.send(Outgoing::Recognize(&img()));
    thread::sleep(Duration::from_millis(60));
    drop(scan);
    assert!(
        wait_until(LONG, || fake.cancels() == 1),
        "the daemon kept scanning for nobody"
    );

    let (next, _) = Conn::greeted(&paths);
    match next.ask(Outgoing::Recognize(&img())) {
        Reply::Lines(text) => assert_eq!(label_of(&text), "d"),
        other => panic!("expected lines, got {other:?}"),
    }
    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn hello_and_status_answer_while_loading_and_scanning() {
    let (dir, paths) = dir_and_paths("status");
    let fake = Fake::new("d").slow(80, Duration::from_millis(5));
    let server = start(config(&dir, delayed(fake, Duration::from_millis(400))));
    let (conn, welcome) = Conn::greeted(&paths);
    assert_eq!(welcome.state, EngineState::Idle);
    assert_eq!(conn.ask(Outgoing::Load(&testing::files("m"))), Reply::Ok);

    let quick_status = |what: &str| {
        let started = Instant::now();
        let reply = Conn::greeted(&paths).0.ask(Outgoing::Status);
        assert!(
            started.elapsed() < Duration::from_millis(200),
            "{what} took {:?}",
            started.elapsed()
        );
        match reply {
            Reply::Status(status) => status,
            other => panic!("expected status, got {other:?}"),
        }
    };
    let loading = quick_status("status while loading");
    assert!(matches!(loading.state, EngineState::Loading { .. }));
    assert_eq!(loading.pid, std::process::id());

    wait_ready(&paths);
    let (scan, _) = Conn::greeted(&paths);
    scan.send(Outgoing::Recognize(&img()));
    thread::sleep(Duration::from_millis(50));
    let scanning = quick_status("status while scanning");
    assert_eq!(scanning.served, 0);
    assert_eq!(
        scanning.idle_left_secs,
        Some(60),
        "a scan keeps the idle timer full"
    );
    assert!(matches!(scan.reply(), Reply::Lines(_)));
    assert_eq!(quick_status("status after").served, 1);

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn idle_daemon_leaves_and_cleans_up() {
    let (dir, paths) = dir_and_paths("idle");
    let server = start(server::Config {
        idle: Some(Duration::from_millis(200)),
        ..config(&dir, factory(Fake::new("d")))
    });
    load(&paths, &testing::files("m"));
    let started = Instant::now();
    assert_eq!(finish(server), Outcome::Idle);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!paths.socket.exists());
    assert!(lock_free(&paths.lock));
}

#[test]
fn idle_timer_does_not_fire_during_a_scan() {
    let (dir, paths) = dir_and_paths("idle-scan");
    let fake = Fake::new("d").slow(160, Duration::from_millis(5));
    let server = start(server::Config {
        idle: Some(Duration::from_millis(300)),
        ..config(&dir, factory(fake))
    });
    load(&paths, &testing::files("m"));
    wait_ready(&paths);

    let (scan, _) = Conn::greeted(&paths);
    scan.send(Outgoing::Recognize(&img()));
    thread::sleep(Duration::from_millis(600));
    assert!(!server.is_finished(), "left in the middle of a scan");
    assert!(matches!(scan.reply(), Reply::Lines(_)));
    assert_eq!(finish(server), Outcome::Idle);
}

#[test]
fn idle_exit_waits_for_an_open_connection() {
    let (dir, paths) = dir_and_paths("idle-conn");
    let server = start(server::Config {
        idle: Some(Duration::from_millis(200)),
        io_timeout: Duration::from_millis(900),
        ..config(&dir, factory(Fake::new("d")))
    });
    let held = Conn::open(&paths);
    assert!(matches!(
        held.ask(Outgoing::Hello(BUILD)),
        Reply::Welcome(_)
    ));
    thread::sleep(Duration::from_millis(500));
    assert!(
        !server.is_finished(),
        "left while a client was mid-conversation"
    );

    drop(held);
    assert_eq!(finish(server), Outcome::Idle);
}

#[test]
fn shutdown_does_not_wait_for_a_long_scan() {
    let (dir, paths) = dir_and_paths("shutdown");
    let fake = Fake::new("d").slow(400, Duration::from_millis(5)).deaf();
    let server = start(config(&dir, factory(fake)));
    load(&paths, &testing::files("m"));
    wait_ready(&paths);

    let (scan, _) = Conn::greeted(&paths);
    scan.send(Outgoing::Recognize(&img()));
    thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn engine_that_cannot_run_makes_the_daemon_leave() {
    let cases: [(Factory, EngineState); 2] = [
        (
            Arc::new(|_: &ModelFiles| Err(BuildError::NoGpu)),
            EngineState::NoGpu,
        ),
        (
            Arc::new(|_: &ModelFiles| Err(BuildError::Failed("boom".into()))),
            EngineState::Failed("boom".into()),
        ),
    ];
    for (make, expected) in cases {
        let (dir, paths) = dir_and_paths("unusable");
        let server = start(config(&dir, make));
        load(&paths, &testing::files("m"));
        assert!(wait_until(LONG, || state(&paths).as_ref() == Some(&expected)));
        assert_eq!(finish(server), Outcome::Unusable);
        assert!(lock_free(&paths.lock));
    }
}

#[test]
fn panicking_loader_is_reported_as_failed() {
    let (dir, paths) = dir_and_paths("loader-panic");
    let make: Factory = Arc::new(|_: &ModelFiles| panic!("loader panicked on purpose"));
    let server = start(config(&dir, make));
    load(&paths, &testing::files("m"));
    assert!(wait_until(LONG, || matches!(
        state(&paths),
        Some(EngineState::Failed(_))
    )));
    assert_eq!(finish(server), Outcome::Unusable);
}

#[test]
fn crashing_engine_is_dropped_and_the_daemon_leaves() {
    let (dir, paths) = dir_and_paths("crash");
    let server = start(config(&dir, factory(Fake::new("d").panicking())));
    load(&paths, &testing::files("m"));
    wait_ready(&paths);
    let (conn, _) = Conn::greeted(&paths);
    assert!(matches!(
        conn.ask(Outgoing::Recognize(&img())),
        Reply::Error(_)
    ));
    assert_eq!(finish(server), Outcome::Unusable);
}

#[test]
fn too_long_socket_path_is_an_error_not_a_panic() {
    let root = testing::temp_dir("long");
    let dir = root.join("x".repeat(110));
    let result = server::serve(config(&dir, factory(Fake::new("d"))));
    assert!(result.is_err(), "{result:?}");
    assert!(lock_free(&Paths::in_dir(dir).lock));
}

#[test]
fn only_the_latest_requested_model_is_kept() {
    let (dir, paths) = dir_and_paths("switch");
    let make: Factory = Arc::new(|files: &ModelFiles| {
        if model_name(files).contains("slow") {
            thread::sleep(Duration::from_millis(300));
        }
        Ok(Engine {
            backend: Box::new(Fake::new("d")),
            device: "fake".into(),
        })
    });
    let server = start(config(&dir, make));
    let fast = testing::files("fast");
    load(&paths, &testing::files("slow"));
    load(&paths, &fast);
    let wanted = ready(&fast);
    assert!(wait_until(LONG, || state(&paths).as_ref() == Some(&wanted)));
    thread::sleep(Duration::from_millis(450));
    assert_eq!(
        state(&paths),
        Some(wanted),
        "the slow, older build overwrote the newer model"
    );

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn switching_models_during_a_scan_keeps_the_answer() {
    let (dir, paths) = dir_and_paths("switch-scan");
    let make: Factory = Arc::new(|files: &ModelFiles| {
        let label = if model_name(files).contains("second") {
            "second"
        } else {
            "first"
        };
        Ok(Engine {
            backend: Box::new(Fake::new(label).slow(60, Duration::from_millis(5))),
            device: "fake".into(),
        })
    });
    let server = start(config(&dir, make));
    let (first, second) = (testing::files("first"), testing::files("second"));
    load(&paths, &first);
    wait_ready(&paths);

    let (scan, _) = Conn::greeted(&paths);
    scan.send(Outgoing::Recognize(&img()));
    thread::sleep(Duration::from_millis(50));
    load(&paths, &second);
    match scan.reply() {
        Reply::Lines(text) => {
            assert_eq!(label_of(&text), "first", "the running scan kept its engine")
        }
        other => panic!("expected lines, got {other:?}"),
    }
    let wanted = ready(&second);
    assert!(wait_until(LONG, || state(&paths).as_ref() == Some(&wanted)));

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}

#[test]
fn scan_before_the_engine_is_ready_is_refused_quickly() {
    let (dir, paths) = dir_and_paths("not-ready");
    let server = start(config(
        &dir,
        delayed(Fake::new("d"), Duration::from_millis(500)),
    ));
    let (conn, _) = Conn::greeted(&paths);
    assert!(
        matches!(conn.ask(Outgoing::Recognize(&img())), Reply::Error(_)),
        "idle"
    );
    assert_eq!(conn.ask(Outgoing::Load(&testing::files("m"))), Reply::Ok);
    let started = Instant::now();
    assert!(
        matches!(conn.ask(Outgoing::Recognize(&img())), Reply::Error(_)),
        "loading"
    );
    assert!(started.elapsed() < Duration::from_millis(200));

    shutdown(&paths);
    assert_eq!(finish(server), Outcome::Shutdown);
}
