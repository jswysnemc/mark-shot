use super::*;

#[test]
fn status_and_stop_commands() {
    let (dir, paths) = dir_and_paths("stop");
    let server = start(config(&dir, factory(Fake::new("d"))));
    let status = client::status(&paths).expect("a running daemon answers status");
    assert_eq!(status.pid, std::process::id());
    assert!(status.idle_left_secs.is_some_and(|secs| secs <= 60));

    let obeys = client::stop(&paths, Duration::from_secs(2), &|_| {
        panic!("an obedient daemon must not be killed")
    });
    assert_eq!(obeys, Ok(true));
    assert_eq!(finish(server), Outcome::Shutdown);
    assert!(client::status(&paths).is_none());
    assert_eq!(
        client::stop(&paths, Duration::from_millis(200), &|_| panic!(
            "nothing to kill"
        )),
        Ok(false)
    );
}

#[test]
fn client_starts_the_daemon_and_moves_to_it_when_warm() {
    let (dir, paths) = dir_and_paths("warm");
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(
        &dir,
        delayed(Fake::new("daemon"), Duration::from_millis(300)),
        &probe,
        &servers,
    );

    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    assert_eq!(probe.spawns(), 1);
    assert_eq!(
        read(&*backend),
        "local",
        "a cold daemon must not make the user wait"
    );
    assert!(wait_until(LONG, || read(&*backend) == "daemon"));
    assert_eq!(probe.spawns(), 1);
    assert_eq!(probe.locals(), 1);
    assert!(read_strikes(&paths).is_empty());

    shutdown(&paths);
    assert_eq!(finish_all(&servers), vec![Outcome::Shutdown]);
}

#[test]
fn daemon_dying_mid_scan_means_a_local_read_and_a_strike() {
    let (_dir, paths) = dir_and_paths("dies");
    let files = testing::files("m");
    let state = ready(&files);
    let daemon = fake_daemon(&paths, 2, move |stream| {
        answer_hello(&stream, state.clone());
        let mut reader = &stream;
        // Accept the image and hang up without sending a reply.
        let _ = protocol::read_request(&mut reader);
    });
    let probe = Probe::default();
    let backend = client::connect_with(
        client_config(&paths, refusing_spawner(&probe), &probe),
        &files,
    )
    .unwrap();

    assert_eq!(read(&*backend), "local");
    assert!(read_strikes(&paths).contains("failed"));
    daemon.join().unwrap();
    assert_eq!(read(&*backend), "local");
    assert_eq!(
        probe.spawns(),
        0,
        "a daemon that broke must not be retried in the same session"
    );
    assert_eq!(probe.locals(), 1);
}

#[test]
fn hung_daemon_is_killed_and_the_scan_read_locally() {
    let (_dir, paths) = dir_and_paths("hung");
    let files = testing::files("m");
    let state = ready(&files);
    let (release, released) = std::sync::mpsc::channel::<()>();
    let released = Mutex::new(released);
    let daemon = fake_daemon(&paths, 2, move |stream| {
        answer_hello(&stream, state.clone());
        let mut reader = &stream;
        if protocol::read_request(&mut reader).is_ok() {
            let _ = released.lock().unwrap().recv_timeout(LONG);
        }
    });
    let probe = Probe::default();
    let config = client::Config {
        timing: Timing {
            reply_base: Duration::from_millis(300),
            ..timing()
        },
        ..client_config(&paths, refusing_spawner(&probe), &probe)
    };
    let backend = client::connect_with(config, &files).unwrap();

    let started = Instant::now();
    assert_eq!(read(&*backend), "local");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(probe.kills(), vec![std::process::id() as i32]);
    assert!(read_strikes(&paths).contains("failed"));
    drop(release);
    daemon.join().unwrap();
}

#[test]
fn handshake_with_a_leaving_daemon_is_retried_without_a_strike() {
    let (dir, paths) = dir_and_paths("leaving");
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(&dir, factory(Fake::new("fresh")), &probe, &servers);
    let socket = paths.socket.clone();
    let leaving = fake_daemon(&paths, 1, move |stream| {
        let _ = fs::remove_file(&socket);
        drop(stream);
    });

    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    leaving.join().unwrap();
    assert!(
        read_strikes(&paths).is_empty(),
        "a daemon on its way out is not a failure"
    );
    assert_eq!(probe.spawns(), 1);
    assert!(wait_until(LONG, || read(&*backend) == "fresh"));

    shutdown(&paths);
    assert_eq!(finish_all(&servers), vec![Outcome::Shutdown]);
}

#[test]
fn a_successful_scan_clears_old_strikes() {
    let (dir, paths) = dir_and_paths("clear-strikes");
    record_strike(&paths, BUILD, Strike::Failed);
    record_strike(&paths, BUILD, Strike::Failed);
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(&dir, factory(Fake::new("daemon")), &probe, &servers);

    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    assert!(wait_until(LONG, || read(&*backend) == "daemon"));
    assert!(
        read_strikes(&paths).is_empty(),
        "a working daemon forgets old failures"
    );

    shutdown(&paths);
    assert_eq!(finish_all(&servers), vec![Outcome::Shutdown]);
}

#[test]
fn daemon_from_another_build_is_replaced_exactly_once() {
    let (dir, paths) = dir_and_paths("foreign");
    let old = start(server::Config {
        build: "old-build".into(),
        ..config(&dir, factory(Fake::new("old")))
    });
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(&dir, factory(Fake::new("new")), &probe, &servers);

    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    assert_eq!(finish(old), Outcome::Shutdown);
    assert_eq!(probe.spawns(), 1);
    assert_eq!(Conn::greeted(&paths).1.build, BUILD);
    assert!(wait_until(LONG, || read(&*backend) == "new"));
    assert!(
        probe.kills().is_empty(),
        "the old daemon obeyed, nobody should be killed"
    );

    shutdown(&paths);
    assert_eq!(finish_all(&servers), vec![Outcome::Shutdown]);
}

#[test]
fn concurrent_clients_start_a_single_daemon() {
    let (dir, paths) = dir_and_paths("race");
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(&dir, factory(Fake::new("daemon")), &probe, &servers);

    let clients: Vec<_> = (0..6)
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
    assert!(probe.spawns() >= 1);

    // Unsuccessful lock competitors wait for `lock_wait`: allow them to exit before issuing `Shutdown` for the winner.
    thread::sleep(Duration::from_millis(250));
    shutdown(&paths);
    let outcomes = finish_all(&servers);
    let serving = outcomes
        .iter()
        .filter(|o| **o != Outcome::AlreadyRunning)
        .count();
    assert_eq!(serving, 1, "{outcomes:?}");
}

#[test]
fn strike_rules() {
    let now = 10_000;
    let failed = |ago: u64| format!("{} failed {BUILD}\n", now - ago);
    assert!(!daemon_blocked("", BUILD, now));

    let two = failed(10) + &failed(20);
    assert!(!daemon_blocked(&two, BUILD, now));
    assert!(daemon_blocked(&(two.clone() + &failed(30)), BUILD, now));
    assert!(
        !daemon_blocked(&(two.clone() + &failed(601)), BUILD, now),
        "old failures expire"
    );
    assert!(!daemon_blocked(
        &(two + &format!("{now} failed other-build\n")),
        BUILD,
        now
    ));

    let no_gpu = format!("{now} no-gpu {BUILD}\n");
    assert!(
        daemon_blocked(&no_gpu, BUILD, now + 10_000_000),
        "no GPU does not expire for a build"
    );
    assert!(gpu_ruled_out(&no_gpu, BUILD));
    assert!(!gpu_ruled_out(&no_gpu, "next-build"));
    assert!(!daemon_blocked(
        "junk\n\n1 2 3\nnan failed test-build\n42 exploded test-build\n",
        BUILD,
        now
    ));
}

#[test]
fn repeated_failures_stop_spawning_for_a_while() {
    let (_dir, paths) = dir_and_paths("strikes");
    for _ in 0..20 {
        record_strike(&paths, BUILD, Strike::Failed);
    }
    assert_eq!(
        read_strikes(&paths).lines().count(),
        8,
        "the strike file must stay small"
    );
    assert!(daemon_blocked(&read_strikes(&paths), BUILD, unix_now()));

    let probe = Probe::default();
    let backend = client::connect_with(
        client_config(&paths, refusing_spawner(&probe), &probe),
        &testing::files("m"),
    )
    .unwrap();
    assert_eq!(read(&*backend), "local");
    assert_eq!(probe.spawns(), 0);
}

#[test]
fn daemon_without_gpu_is_remembered_and_not_started_again() {
    let (dir, paths) = dir_and_paths("no-gpu");
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(
        &dir,
        Arc::new(|_: &ModelFiles| Err(BuildError::NoGpu)),
        &probe,
        &servers,
    );

    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    assert!(wait_until(LONG, || {
        read(&*backend) == "local" && gpu_ruled_out(&read_strikes(&paths), BUILD)
    }));
    assert_eq!(probe.spawns(), 1);
    assert_eq!(finish_all(&servers), vec![Outcome::Unusable]);

    let again = client::connect_with(
        client_config(&paths, refusing_spawner(&probe), &probe),
        &testing::files("m"),
    )
    .unwrap();
    assert_eq!(read(&*again), "local");
    assert_eq!(probe.spawns(), 1);
}

#[test]
fn cancelling_while_the_daemon_scans_returns_at_once() {
    let (dir, paths) = dir_and_paths("cancel");
    let fake = Fake::new("daemon").slow(400, Duration::from_millis(5));
    let probe = Probe::default();
    let servers = Servers::default();
    let spawn = thread_spawner(&dir, factory(fake.clone()), &probe, &servers);
    let backend =
        client::connect_with(client_config(&paths, spawn, &probe), &testing::files("m")).unwrap();
    wait_ready(&paths);

    let flag = Arc::new(AtomicBool::new(false));
    let setter = {
        let flag = flag.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            flag.store(true, Ordering::SeqCst);
        })
    };
    let started = Instant::now();
    let result = backend.recognize(img(), &|| flag.load(Ordering::SeqCst));
    assert_eq!(result.unwrap_err().to_string(), CANCELLED);
    assert!(started.elapsed() < Duration::from_millis(600));
    assert!(
        wait_until(LONG, || fake.cancels() == 1),
        "the daemon did not notice the cancel"
    );
    assert!(read_strikes(&paths).is_empty(), "a cancel is not a failure");
    assert_eq!(probe.locals(), 0);

    setter.join().unwrap();
    shutdown(&paths);
    assert_eq!(finish_all(&servers), vec![Outcome::Shutdown]);
}
