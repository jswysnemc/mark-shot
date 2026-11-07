//! 【持续捕获回归】【生产接口】通过真实Portal后端读取私有PipeWire原始视频并检查复用与回收

use mark_shot::{
    backend::{CaptureMethod, wayland::capture::portal::PortalMethod},
    types::MonitorFrame,
};
use std::time::{Duration, Instant};

/// 【持续捕获回归】【帧检查】核对frame的尺寸、紧密行距和纯红原始像素，返回完整像素指纹
fn verify(frame: &MonitorFrame) -> mark_shot::AppResult<String> {
    if (frame.pw_width, frame.pw_height, frame.pw_stride) != (64, 32, 256) {
        return Err("unexpected raw stream dimensions".into());
    }
    if frame.pixels.len() != 64 * 32 * 4
        || !frame
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel == &[255, 0, 0, 255])
    {
        return Err("PipeWire did not preserve the exact red source pixels".into());
    }
    Ok(format!(
        "{}x{}:{}",
        frame.pw_width,
        frame.pw_height,
        frame
            .pixels
            .iter()
            .map(|value| u64::from(*value))
            .sum::<u64>()
    ))
}

/// 【持续捕获回归】【线程数量】读取本进程task目录，返回真实原生线程数量，无参数
fn threads() -> usize {
    std::fs::read_dir("/proc/self/task").unwrap().count()
}

/// 【持续捕获回归】【验证入口】以args指定结果文件，运行生产Portal会话并检查重复读帧、暂停和关闭，返回验证结果
pub(super) async fn run(args: &[String]) -> mark_shot::AppResult {
    mark_shot::config::init(None, |_| {});
    let conn = wayland_client::Connection::connect_to_env()?;
    let overlay = mark_shot::backend::initialize_overlay(conn)?;
    let output = overlay
        .discovered_outputs()
        .first()
        .ok_or("private output required")?
        .clone();
    let method = PortalMethod {
        include_cursor: false,
    };
    let opened = method.open_session(&[output]).await;
    if args.iter().any(|arg| arg == "reject") {
        if opened.is_ok() {
            return Err("source rejection unexpectedly opened a capture session".into());
        }
        std::fs::write(
            args.first().ok_or("result required")?,
            "{\"rejected\":true}\n",
        )?;
        return Ok(());
    }
    let mut session = opened.map_err(|error| error as Box<dyn std::error::Error>)?;
    let cancelled = args.iter().any(|arg| arg == "cancel");
    if cancelled {
        // 1. 【持续捕获回归】【等待取消】端口尚未连接时丢弃请求，恢复仍必须复用同一授权和同一流
        if tokio::time::timeout(Duration::from_millis(150), session.next_frame())
            .await
            .is_ok()
        {
            return Err("unlinked stream unexpectedly produced a frame before cancellation".into());
        }
        println!("cancelled");
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    let mut samples = Vec::new();
    for _ in 0..5 {
        let now = Instant::now();
        let result = session
            .next_frame()
            .await
            .map_err(|error| error as Box<dyn std::error::Error>)?;
        let frame = result.frames.first().ok_or("missing frame")?;
        // 1. 【持续捕获回归】【原生逻辑尺寸】Portal报告的视频尺寸与输出逻辑尺寸不同，生产结果必须保留输出逻辑尺寸
        if frame.info.size != overlay.discovered_outputs()[0].info.logical_size {
            return Err("portal physical size replaced logical output dimensions".into());
        }
        samples.push(serde_json::json!({ "pixels": verify(frame)?, "latency_ms": now.elapsed().as_secs_f64() * 1000.0, "threads": threads() }));
        tokio::time::sleep(Duration::from_millis(70)).await;
    }
    println!("idle");
    tokio::time::sleep(Duration::from_millis(900)).await;
    let after_pause = session
        .next_frame()
        .await
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    verify(&after_pause.frames[0])?;
    let before_close = threads();
    let now = Instant::now();
    session
        .close()
        .await
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    session
        .close()
        .await
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    let close_ms = now.elapsed().as_secs_f64() * 1000.0;
    if session.next_frame().await.is_ok() {
        return Err("closed capture accepted another frame".into());
    }
    let after_close = threads();
    if after_close >= before_close {
        return Err("closing the session did not remove its PipeWire thread".into());
    }
    let report = serde_json::json!({ "samples": samples, "resumed": true, "cancelled_request_resumed": cancelled, "before_close_threads": before_close, "after_close_threads": after_close, "close_ms": close_ms, "closed_read_rejected": true });
    std::fs::write(
        args.first().ok_or("result required")?,
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}
