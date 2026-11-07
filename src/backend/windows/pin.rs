//! 【Windows贴图】【命令入口】共享贴图参数与进程交接，原生运行时独立实现

#[path = "pin/runtime.rs"]
mod runtime;
use crate::AppResult;
pub use runtime::run;
pub const PIN_ARG: &str = "--pin";
pub const SCALE_ARG: &str = "--scale";
pub const POSITION_ARG: &str = "--position";

/// 【贴图】【内部参数】从 args 读取文件或有界标准输入；返回窗口运行结果，非法比例直接拒绝
pub fn cli(mut args: impl Iterator<Item = String>) -> AppResult {
    use std::io::Read;
    let mut scale = 1.0;
    let mut path = None;
    let mut position = None;
    let mut translation_seed = false;
    while let Some(argument) = args.next() {
        if argument == crate::features::image_translation::seed::ARG {
            translation_seed = true;
        } else if argument == SCALE_ARG {
            scale = args
                .next()
                .ok_or("--scale requires a number")?
                .parse::<f32>()?;
            if !scale.is_finite() || scale <= 0.0 {
                return Err("--scale requires a finite positive number".into());
            }
        } else if argument == POSITION_ARG {
            let value = args.next().ok_or("--position requires x,y")?;
            let (x, y) = value.split_once(',').ok_or("--position requires x,y")?;
            position = Some((x.parse::<i32>()?, y.parse::<i32>()?));
        } else if argument.starts_with('-') {
            return Err("unknown pin argument".into());
        } else if path.replace(argument).is_some() {
            return Err("pin accepts one file".into());
        }
    }
    let bytes = if let Some(path) = path {
        std::fs::read(path)?
    } else {
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(crate::features::image_translation::seed::LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > crate::features::image_translation::seed::LIMIT {
            return Err("pin image exceeds the byte limit".into());
        }
        bytes
    };
    if translation_seed {
        let (seed, png) = crate::features::image_translation::seed::decode(&bytes)?;
        runtime::run_at_seed(png, scale, position, Some(seed))
    } else {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("pin image exceeds the byte limit".into());
        }
        runtime::run_at(&bytes, scale, position)
    }
}

/// 【图片翻译】【贴图启动】把 png、scale、position 和 seed 交给独立贴图进程，返回启动结果，不烘焙译文到原图
pub fn spawn_translated(
    png: &[u8],
    scale: f32,
    position: Option<(i32, i32)>,
    seed: &crate::features::image_translation::seed::Seed,
) -> AppResult {
    let payload = crate::features::image_translation::seed::encode(png, seed)?;
    let scale = scale.to_string();
    let mut arguments = vec![
        PIN_ARG,
        SCALE_ARG,
        scale.as_str(),
        crate::features::image_translation::seed::ARG,
    ];
    let position = position.map(|(x, y)| format!("{x},{y}"));
    if let Some(position) = position.as_deref() {
        arguments.extend([POSITION_ARG, position]);
    }
    crate::utils::spawn_self(&arguments, &payload)
}

/// 【贴图】【独立启动】把 png、scale 交给独立贴图进程；返回启动结果，不保留长期等待线程
pub fn spawn(png: &[u8], scale: f32) -> AppResult {
    crate::utils::spawn_self(&[PIN_ARG, SCALE_ARG, &scale.to_string()], png)
}

/// 【贴图】【选区位置交接】把 png、scale 与全局逻辑 position 交给贴图进程，目标输出由位置确定
pub fn spawn_at(png: &[u8], scale: f32, position: Option<(i32, i32)>) -> AppResult {
    let Some((x, y)) = position else {
        return spawn(png, scale);
    };
    crate::utils::spawn_self(
        &[
            PIN_ARG,
            SCALE_ARG,
            &scale.to_string(),
            POSITION_ARG,
            &format!("{x},{y}"),
        ],
        png,
    )
}
