use super::*;

#[test]
#[ignore]
fn the_local_engine_never_touches_the_gpu() {
    let traces = read_probe(false);
    assert!(
        traces.is_empty(),
        "on-demand OCR opened the GPU: {traces:?}"
    );
}

#[test]
#[ignore]
fn the_gpu_engine_is_seen_by_the_check_above() {
    let traces = read_probe(true);
    assert!(
        !traces.is_empty(),
        "the GPU engine left no trace, so the check proves nothing"
    );
}

#[test]
#[ignore]
fn local_engine_cost() {
    let models = crate::ocr::models::OcrModels::load();
    let files = models
        .active()
        .and_then(|idx| models.files(idx))
        .expect("install an OCR model first");
    let started = Instant::now();
    let backend = crate::ocr::build_backend(crate::ocr::settings::Mode::OnDemand, &files).unwrap();
    let build_ms = started.elapsed().as_millis();
    let mut scans = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        backend
            .recognize(super::super::calibrate::probe_image(), &never)
            .unwrap();
        scans.push(started.elapsed().as_millis());
    }
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let field = |name: &str| {
        status
            .lines()
            .find(|l| l.starts_with(name))
            .unwrap_or_default()
            .to_owned()
    };
    println!(
        "COST build {build_ms} ms, scans {scans:?} ms, {}, {}",
        field("VmHWM"),
        field("Threads")
    );
}
