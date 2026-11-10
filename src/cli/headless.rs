use std::io::Write;

use crate::backend::session::Session as Connection;
use serde::Serialize;

use super::arguments::CaptureOptions;
use super::capture_output::{self, Report};
use crate::AppResult;
use crate::backend::{ScreenOverlay, output_selection};
use crate::types::Output;

#[derive(Serialize)]
struct Display {
    name: String,
    description: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f32,
    active: bool,
}

/// 【截图】【显示器发现】建立未映射的 Wayland 客户端，只读取输出信息，不显示编辑器
fn discover() -> AppResult<(Connection, Box<dyn ScreenOverlay>)> {
    let connection = Connection::connect_to_env()?;
    let overlay = crate::backend::initialize_overlay(connection.clone())?;
    Ok((connection, overlay))
}

/// 【截图】【显示器列表】向标准输出写入显示器 JSON 数组；不捕获像素和加载 OCR
pub(super) fn list_displays() -> AppResult {
    let (_, mut overlay) = discover()?;
    let active = overlay.active_output();
    let displays: Vec<_> = overlay
        .discovered_outputs()
        .iter()
        .enumerate()
        .map(|(index, output)| {
            let (x, y) = output.info.logical_position.unwrap_or(output.info.location);
            let (width, height) = output.info.logical_size.unwrap_or_else(|| {
                output
                    .info
                    .modes
                    .iter()
                    .find(|mode| mode.current)
                    .map(|mode| mode.dimensions)
                    .unwrap_or((0, 0))
            });
            Display {
                name: output_selection::name(output, index),
                description: output.info.description.clone().unwrap_or_default(),
                x,
                y,
                width,
                height,
                scale: output.scale(),
                active: active == Some(index),
            }
        })
        .collect();
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &displays)?;
    writeln!(stdout)?;
    Ok(())
}

/// 【截图】【图像合成】捕获指定输出并按全局逻辑区域合成 PNG；返回图像和实际像素尺寸
async fn capture(
    method: &dyn crate::backend::CaptureMethod,
    outputs: &[Output],
    options: &CaptureOptions,
) -> AppResult<(Vec<u8>, u32, u32, f32)> {
    let visibility = crate::backend::capture_visibility::Guard::begin(
        crate::config::preferences::current()
            .capture
            .hide_own_windows,
    )?;
    let result = method.capture_frame(outputs).await?;
    drop(visibility);
    let kept: Vec<_> = result
        .frames
        .iter()
        .map(|frame| {
            outputs
                .get(frame.output)
                .cloned()
                .ok_or("capture returned an invalid display index")
        })
        .collect::<Result<_, _>>()?;
    let captures = crate::app::build_captures(result.frames)?;
    let placements = crate::app::build_placements(&kept, false);
    let bounds =
        crate::utils::get_full_workspace_rect(&placements).ok_or("no display was captured")?;
    let region = options
        .region
        .map(|region| region.rect().ok_or("invalid capture region"))
        .transpose()?
        .unwrap_or(bounds);
    if region.intersect(&bounds).is_none() {
        return Err("the requested region does not intersect the selected displays".into());
    }
    let (pixmap, _, scale) = crate::renderer::composite(&captures, &placements, region, None)
        .ok_or("cannot composite the requested region")?;
    Ok((
        crate::utils::encode_png(&pixmap),
        pixmap.width(),
        pixmap.height(),
        scale,
    ))
}

/// 【截图】【无界面捕获】依次捕获目标显示器、原子保存并打印原版 JSON 报告；任一失败返回错误
pub(super) async fn run(options: &CaptureOptions) -> AppResult {
    let mut profiler = crate::profiler::Profiler::new(options.speed);
    let (connection, mut overlay) = match discover() {
        Ok(found) => found,
        Err(error) => {
            report_error(options, &error.to_string())?;
            return Err(error);
        }
    };
    let all = overlay.discovered_outputs().to_vec();
    let names: Vec<_> = all
        .iter()
        .enumerate()
        .map(|(index, output)| output_selection::name(output, index))
        .collect();
    profiler.mark("outputs discovered");
    let active = if options.displays.is_empty() && !options.all_outputs {
        overlay.active_output()
    } else {
        None
    };
    let chosen =
        match output_selection::indices(&names, &options.displays, options.all_outputs, active) {
            Ok(chosen) => chosen,
            Err(error) => {
                report_error(options, &error)?;
                return Err(error.into());
            }
        };
    let method =
        crate::backend::initialize_capture_with_cursor(&connection, options.include_cursor);
    if let Some(deadline) = options.shot_at {
        if let Err(error) = method.prepare().await {
            report_error(options, &error.to_string())?;
            return Err(error);
        }
        tokio::time::sleep(deadline.saturating_duration_since(std::time::Instant::now())).await;
    }
    let multiple = options.displays.len() > 1;
    let batches: Vec<Vec<usize>> = if multiple {
        chosen.into_iter().map(|index| vec![index]).collect()
    } else {
        vec![chosen]
    };
    let mut reports = Vec::new();
    for indices in batches {
        let output = if indices.len() == 1 {
            names[indices[0]].clone()
        } else {
            "all".into()
        };
        let path = match capture_output::destination(options, &output, multiple) {
            Ok(path) => path,
            Err(error) => {
                reports.push(Report {
                    path: options.destination.clone(),
                    width: 0,
                    height: 0,
                    output,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        let selected: Vec<_> = indices.iter().map(|index| all[*index].clone()).collect();
        let result =
            capture(&*method, &selected, options)
                .await
                .and_then(|(png, width, height, scale)| {
                    capture_output::write(&path, &png)?;
                    crate::features::capture_history::remember(&png, scale, &output);
                    Ok((width, height))
                });
        let (width, height, error) = match result {
            Ok((width, height)) => (width, height, None),
            Err(error) => (0, 0, Some(error.to_string())),
        };
        reports.push(Report {
            path,
            width,
            height,
            output,
            error,
        });
    }
    capture_output::print(&reports, multiple)?;
    profiler.mark("capture + PNG + atomic save");
    profiler.dump();
    if reports.iter().any(|report| report.error.is_some()) {
        return Err("one or more captures failed; see the JSON report".into());
    }
    Ok(())
}

/// 【截图】【错误报告】在 Wayland 连接或显示器选择失败时仍输出机器可读结果
fn report_error(options: &CaptureOptions, error: &str) -> AppResult {
    let multiple = options.displays.len() > 1;
    let names = if options.displays.is_empty() {
        vec![String::new()]
    } else {
        options.displays.clone()
    };
    let reports: Vec<_> = names
        .into_iter()
        .map(|output| Report {
            path: options.destination.clone(),
            width: 0,
            height: 0,
            output,
            error: Some(error.into()),
        })
        .collect();
    capture_output::print(&reports, multiple)
}
