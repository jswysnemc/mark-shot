//! 【贴图】【显示比例】优先使用 PNG 内嵌 DPR，未知来源按配置回退系统缩放或逻辑像素

use crate::config::ClipboardImageScale;

/// 【贴图】【像素来源】区分文本合成、剪贴板原始图片和文件图片，供比例策略决定是否回退系统缩放
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Text,
    ClipboardImage,
    FileImage,
}

impl Source {
    /// 【贴图】【图片来源】无参数，返回像素是否来自图片而不是文本或 HTML 合成
    fn is_image(self) -> bool {
        matches!(self, Self::ClipboardImage | Self::FileImage)
    }
}

/// 【贴图】【内嵌比例】按 png 返回 mark-shot 写入的设备像素比；没有 DPR 文本块或格式无效时返回空值
pub fn embedded(png: &[u8]) -> Option<f32> {
    crate::features::capture_history::png_metadata::inspect(&mut std::io::Cursor::new(png))
        .ok()
        .and_then(|info| info.dpr)
}

/// 【贴图】【比例策略】按 policy、dpr、source 和 system 返回显示密度；文本合成始终使用逻辑像素
pub fn resolve_with(
    policy: ClipboardImageScale,
    dpr: Option<f32>,
    source: Source,
    system: f32,
) -> f32 {
    let system = if system.is_finite() && system > 0.0 {
        system
    } else {
        1.0
    };
    match policy {
        ClipboardImageScale::One => 1.0,
        ClipboardImageScale::System => {
            if source.is_image() {
                system
            } else {
                1.0
            }
        }
        ClipboardImageScale::Auto => dpr.unwrap_or(if source.is_image() { system } else { 1.0 }),
    }
}

/// 【贴图】【显示密度】按 dpr、source 与当前配置返回贴图密度；需要系统比例时读取失败回退 1.0
pub fn resolve(dpr: Option<f32>, source: Source) -> f32 {
    let policy = crate::config::get().pinned.clipboard_image_scale;
    let system = match policy {
        ClipboardImageScale::System if source.is_image() => {
            crate::backend::pin::system_scale().unwrap_or(1.0)
        }
        ClipboardImageScale::Auto if source.is_image() && dpr.is_none() => {
            crate::backend::pin::system_scale().unwrap_or(1.0)
        }
        _ => 1.0,
    };
    resolve_with(policy, dpr, source, system)
}

/// 【贴图】【文本密度】按配置返回文本和 HTML 的渲染密度；auto/system 使用系统缩放，1.0 使用逻辑像素
pub fn text_scale() -> f32 {
    let policy = crate::config::get().pinned.clipboard_image_scale;
    if policy == ClipboardImageScale::One {
        return 1.0;
    }
    text_scale_with(policy, crate::backend::pin::system_scale().unwrap_or(1.0))
}

/// 【贴图】【文本密度策略】按 policy 和 system 返回文本渲染密度；1.0 固定逻辑像素，其他策略使用有效系统比例
fn text_scale_with(policy: ClipboardImageScale, system: f32) -> f32 {
    match policy {
        ClipboardImageScale::One => 1.0,
        ClipboardImageScale::Auto | ClipboardImageScale::System => {
            if system.is_finite() && system > 0.0 {
                system
            } else {
                1.0
            }
        }
    }
}

/// 【贴图】【复制标记】按 png 和 density 返回带 DPR 文本块的 PNG；写入失败时返回原始字节
pub fn with_dpr(png: Vec<u8>, density: f32) -> Vec<u8> {
    match crate::features::capture_history::png_metadata::with_scale(&png, density) {
        Ok(marked) => marked,
        Err(_) => png,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    /// 【贴图比例测试】【策略】无参数，验证 auto 优先 DPR、system 忽略 DPR、1.0 固定逻辑像素，无返回值
    #[test]
    fn policies_choose_the_expected_density() {
        use Source::{ClipboardImage, FileImage, Text};
        assert_eq!(
            resolve_with(ClipboardImageScale::Auto, Some(1.25), ClipboardImage, 2.0),
            1.25
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::Auto, None, ClipboardImage, 2.0),
            2.0
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::Auto, None, FileImage, 2.0),
            2.0
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::Auto, Some(1.0), FileImage, 2.0),
            1.0
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::Auto, None, Text, 2.0),
            1.0
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::System, Some(1.25), ClipboardImage, 1.5),
            1.5
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::System, None, FileImage, 1.5),
            1.5
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::System, None, Text, 1.5),
            1.0
        );
        assert_eq!(
            resolve_with(ClipboardImageScale::One, Some(1.25), ClipboardImage, 1.5),
            1.0
        );
    }

    /// 【贴图比例测试】【非法系统比例】无参数，验证非有限或非正的系统比例回退到 1.0，无返回值
    #[test]
    fn invalid_system_scale_falls_back_to_one() {
        assert_eq!(
            resolve_with(
                ClipboardImageScale::Auto,
                None,
                Source::ClipboardImage,
                f32::NAN
            ),
            1.0
        );
        assert_eq!(
            resolve_with(
                ClipboardImageScale::System,
                None,
                Source::ClipboardImage,
                0.0
            ),
            1.0
        );
    }

    /// 【贴图比例测试】【文本密度】无参数，验证 auto/system 使用系统比例、1.0 固定逻辑像素，无返回值
    #[test]
    fn text_scale_uses_system_for_auto_and_system() {
        assert_eq!(text_scale_with(ClipboardImageScale::Auto, 1.25), 1.25);
        assert_eq!(text_scale_with(ClipboardImageScale::System, 1.5), 1.5);
        assert_eq!(text_scale_with(ClipboardImageScale::One, 2.0), 1.0);
        assert_eq!(text_scale_with(ClipboardImageScale::Auto, f32::NAN), 1.0);
    }

    /// 【贴图比例测试】【内嵌元数据】无参数，验证普通 PNG 没有 DPR，附加后可读回 1.25，无返回值
    #[test]
    fn embedded_reads_only_real_dpr_metadata() {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&[0, 0, 0, 0], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(embedded(&png), None);
        let marked =
            crate::features::capture_history::png_metadata::with_scale(&png, 1.25).unwrap();
        assert_eq!(embedded(&marked), Some(1.25));
    }
}
