//! 【滤镜测试】【配置兼容】验证默认值、旧开关、模式编号和非法字段隔离

use super::read;
use crate::config::Redact;
use crate::features::filter::FilterMode;

/// 【滤镜测试】【配置模式解析】返回 config 中明确模式、旧开关或默认像素化，遵循工具默认值读取顺序
fn mode(config: &Redact) -> FilterMode {
    config
        .mode
        .or_else(|| config.pixelate.map(FilterMode::from_pixelate))
        .unwrap_or_default()
}

/// 【滤镜测试】【内置默认】空配置使用像素化、十四强度和一单位步长，与原版强度范围一致
#[test]
fn filter_config_defaults_match_reference() {
    let config = read("");
    assert_eq!(mode(&config.tools.redact), FilterMode::Pixelate);
    assert_eq!(config.tools.redact.strength, 14.0);
    assert_eq!(config.tools.redact.step, 1.0);
}

/// 【滤镜测试】【旧开关兼容】旧 pixelate 开关保持原含义，明确 mode 优先，不忽略同节强度
#[test]
fn old_pixelate_flag_is_accepted_but_explicit_mode_takes_precedence() {
    for (legacy, expected) in [(false, FilterMode::Blur), (true, FilterMode::Pixelate)] {
        let config = read(&format!(
            "[tools.redact]\npixelate = {legacy}\nstrength = 23\n"
        ));
        assert_eq!(mode(&config.tools.redact), expected);
        assert_eq!(config.tools.redact.strength, 23.0);
    }
    let config = read("[tools.redact]\npixelate = false\nmode = \"invert\"\n");
    assert_eq!(mode(&config.tools.redact), FilterMode::Invert);
}

/// 【滤镜测试】【全部模式配置】五模式同时支持名称和原版整数编号，强度不改变其他工具笔宽
#[test]
fn all_filter_names_and_reference_ids_read_without_affecting_other_tools() {
    for (index, expected) in FilterMode::ALL.into_iter().enumerate() {
        for value in [format!("\"{}\"", expected.key()), index.to_string()] {
            let config = read(&format!(
                "[tools.redact]\nmode = {value}\nstrength = 31\n[tools.stroke]\nwidth = 7\n"
            ));
            assert_eq!(mode(&config.tools.redact), expected);
            assert_eq!(config.tools.redact.strength, 31.0);
            assert_eq!(config.tools.stroke.width, 7.0);
        }
    }
}

/// 【滤镜测试】【字段隔离】未知模式只回退该字段，强度、旧开关和颜色仍然读取
#[test]
fn unknown_filter_modes_do_not_discard_other_preferences() {
    let config = read(
        "[tools]\ncolor = \"#112233\"\n[tools.redact]\nmode = \"other\"\nstrength = 23\npixelate = false\n",
    );
    assert_eq!(mode(&config.tools.redact), FilterMode::Blur);
    assert_eq!(config.tools.redact.strength, 23.0);
    assert_eq!(config.tools.color, crate::theme::Rgba(17, 34, 51, 255));
}

/// 【滤镜测试】【非法数值】非有限强度和步长回退默认值，有限范围越界则限定到四至四十八
#[test]
fn non_finite_filter_values_fall_back_and_finite_strengths_clamp() {
    let config = read("[tools.redact]\nstrength = nan\nstep = inf\n");
    assert_eq!(config.tools.redact.strength, 14.0);
    assert_eq!(config.tools.redact.step, 1.0);
    assert_eq!(
        read("[tools.redact]\nstrength = -100\n")
            .tools
            .redact
            .strength,
        4.0
    );
    assert_eq!(
        read("[tools.redact]\nstrength = 100\n")
            .tools
            .redact
            .strength,
        48.0
    );
}
