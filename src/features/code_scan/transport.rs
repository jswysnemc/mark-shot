//! 【扫码】【图像交接】选区和贴图通过有界标准输入交给独立结果进程

use std::{io::Read, path::Path};

pub const ARG: &str = "--code-scan-snapshot";
const ABOVE_PIN: &str = "--above-pin";

/// 【扫码】【有界读取】按 reader 读取至多 64 MiB 图像，返回字节或明确的超限错误
pub fn read(reader: impl Read) -> crate::AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(super::decoder::INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > super::decoder::INPUT_LIMIT {
        return Err("扫码图片为空或超过 64 MiB / Invalid scan input size".into());
    }
    Ok(bytes)
}

/// 【扫码】【独立窗口】按 png 启动结果进程，窗口准备完成后返回，调用者可释放截图资源
pub fn spawn(png: &[u8]) -> crate::AppResult {
    spawn_from_pin(png, false)
}

/// 【扫码】【贴图交接】按 png 启动独立结果窗口，above_pin 保证置顶贴图不会遮挡内容；返回首帧就绪或启动错误
pub fn spawn_from_pin(png: &[u8], above_pin: bool) -> crate::AppResult {
    if png.is_empty() || png.len() > super::decoder::INPUT_LIMIT {
        return Err("扫码图片为空或超过 64 MiB / Invalid scan input size".into());
    }
    let args = if above_pin {
        &[ARG, ABOVE_PIN][..]
    } else {
        &[ARG][..]
    };
    crate::utils::spawn_self_ready(args, png, crate::backend::window::READY_TIMEOUT)
}

/// 【扫码】【内部入口】按 args 校验参数并读取标准输入，返回结果窗口生命周期错误
pub fn cli(args: &[String]) -> crate::AppResult {
    let above_pin = match args {
        [] => false,
        [flag] if flag == ABOVE_PIN => true,
        _ => return Err("code scan snapshot only accepts --above-pin".into()),
    };
    super::window::run(read(std::io::stdin().lock())?, true, above_pin)
}

/// 【扫码】【文件入口】按 path 读取图片，json 控制输出机器可读结果或原生窗口，返回读取及识别错误
pub fn file(path: &Path, json: bool) -> crate::AppResult {
    let bytes = read(std::fs::File::open(path)?)?;
    if json {
        let codes = super::decoder::decode(&bytes)?;
        println!("{}", serde_json::to_string(&codes)?);
        Ok(())
    } else {
        super::window::run(bytes, false, false)
    }
}
