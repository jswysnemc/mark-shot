use super::*;
mod filter;

fn read(text: &str) -> Config {
    parse(text, "test")
}

#[test]
fn empty_file_gives_defaults() {
    assert_eq!(read(""), Config::default());
}

#[test]
fn the_template_reads_back_as_the_defaults() {
    assert_eq!(read(TEMPLATE), Config::default());
}

#[test]
/// 【配置测试】【模板覆盖】按完整配置路径校验模板，避免同名字段掩盖遗漏
fn the_template_mentions_every_option() {
    let mut documented = std::collections::BTreeSet::new();
    let mut section = "";
    for line in TEMPLATE.lines() {
        let line = line.trim().strip_prefix("# ").unwrap_or(line.trim());
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = name;
        } else if let Some((key, _)) = line.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                documented.insert(format!("{section}.{key}"));
            }
        }
    }

    /// 【配置测试】【字段遍历】检查 table 下 path 的叶字段；documented 为模板路径，missing 返回遗漏项
    fn walk(
        table: &toml::Table,
        path: &str,
        documented: &std::collections::BTreeSet<String>,
        missing: &mut Vec<String>,
    ) {
        for (key, value) in table {
            let full = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            match value {
                // 1. 【配置测试】【字段遍历】快捷键由模板与默认值往返测试单独覆盖
                toml::Value::Table(_) if full == "keys" => {}
                toml::Value::Table(inner) => walk(inner, &full, documented, missing),
                _ => {
                    if !documented.contains(&full) {
                        missing.push(full);
                    }
                }
            }
        }
    }
    let mut missing = Vec::new();
    walk(
        &toml::Table::try_from(Config::default()).unwrap(),
        "",
        &documented,
        &mut missing,
    );
    assert!(missing.is_empty(), "not in TEMPLATE: {missing:?}");
}

#[test]
fn partial_file_keeps_the_rest_default() {
    let config = read("[toolbar]\nbutton_size = 50.0\n");
    assert_eq!(config.toolbar.button_size, 50.0);
    assert_eq!(config.save, Save::default());
}

/// 【配置测试】【形状默认】标记保留原版填充与三像素描边默认值，不复用矩形设置
#[test]
fn marker_defaults_and_overrides_remain_independent() {
    let defaults = Config::default();
    assert_eq!(defaults.tools.marker.width, 3.0);
    assert!(defaults.tools.marker.filled);
    assert_eq!(
        defaults.tools.marker.shape,
        crate::types::MarkerShape::Triangle
    );
    let config = read(
        "[tools]\nfill = true\n[tools.marker]\nshape = \"spade\"\nfilled = false\nwidth = 7.0\n",
    );
    assert!(config.tools.fill);
    assert!(!config.tools.marker.filled);
    assert_eq!(config.tools.marker.shape, crate::types::MarkerShape::Spade);
    assert_eq!(config.tools.marker.width, 7.0);
    assert_eq!(config.tools.stroke, defaults.tools.stroke);
    assert_eq!(config.tools.laser, defaults.tools.laser);
}

/// 【配置测试】【序号默认】独立序号使用原版三像素笔宽、阿拉伯数字与红色默认色，保留其他工具参数
#[test]
fn standalone_number_defaults_and_overrides_are_independent() {
    let defaults = Config::default();
    assert_eq!(defaults.tools.number.width, 3.0);
    assert_eq!(
        defaults.tools.number.style,
        crate::features::number::NumberStyle::Arabic
    );
    assert_eq!(defaults.tools.color, Rgba(255, 77, 77, 255));
    let configured = read("[tools.number]\nwidth = 72.0\nstyle = \"upper_roman\"\n");
    assert_eq!(configured.tools.number.width, 72.0);
    assert_eq!(
        configured.tools.number.style,
        crate::features::number::NumberStyle::UpperRoman
    );
    assert_eq!(configured.tools.stroke, defaults.tools.stroke);
    assert_eq!(configured.tools.magnifier, defaults.tools.magnifier);
    assert_eq!(configured.tools.marker, defaults.tools.marker);
}

/// 【配置测试】【线条默认】箭头头部、箭身和直线线型独立保存，笔宽使用原版三像素默认值和七十二像素上限
#[test]
fn arrow_and_line_styles_have_independent_reference_defaults_and_overrides() {
    use crate::features::{arrow::ArrowStyle, line::LineStyle};

    let defaults = Config::default();
    assert_eq!(defaults.tools.arrow.width, 3.0);
    assert_eq!(defaults.tools.line.width, 3.0);
    assert_eq!(defaults.tools.arrow.style, ArrowStyle::Fletched);
    assert_eq!(defaults.tools.arrow.line_style, LineStyle::Solid);
    assert_eq!(defaults.tools.line.style, LineStyle::Solid);
    let config = read(
        "[tools.arrow]\nwidth = 72.0\nstyle = \"bidirectional_kde\"\nline_style = \"dashed\"\n[tools.line]\nwidth = 11.0\nstyle = \"dotted\"\n",
    );
    assert_eq!(config.tools.arrow.width, 72.0);
    assert_eq!(config.tools.arrow.style, ArrowStyle::BidirectionalKde);
    assert_eq!(config.tools.arrow.line_style, LineStyle::Dashed);
    assert_eq!(config.tools.line.width, 11.0);
    assert_eq!(config.tools.line.style, LineStyle::Dotted);
    assert_eq!(config.tools.stroke, defaults.tools.stroke);
    assert_eq!(config.tools.number, defaults.tools.number);
    assert_eq!(config.tools.marker, defaults.tools.marker);
}

/// 【配置测试】【线条校验】非法头部、线型和非有限笔宽只回退对应字段，合法参数与另一工具仍继续生效
#[test]
fn invalid_line_fields_fall_back_independently_and_widths_remain_finite() {
    use crate::features::{arrow::ArrowStyle, line::LineStyle};

    let config = read(
        "[tools.arrow]\nwidth = 60.0\nstyle = \"missing\"\nline_style = \"dotted\"\n[tools.line]\nwidth = nan\nstyle = \"dashed\"\n",
    );
    assert_eq!(config.tools.arrow.width, 60.0);
    assert_eq!(config.tools.arrow.style, ArrowStyle::Fletched);
    assert_eq!(config.tools.arrow.line_style, LineStyle::Dotted);
    assert_eq!(config.tools.line.width, 3.0);
    assert_eq!(config.tools.line.style, LineStyle::Dashed);
    let config = read(
        "[tools.arrow]\nwidth = inf\nstyle = \"kde\"\nline_style = \"missing\"\n[tools.line]\nwidth = -10.0\nstyle = \"missing\"\n",
    );
    assert_eq!(config.tools.arrow.width, 3.0);
    assert_eq!(config.tools.arrow.style, ArrowStyle::Kde);
    assert_eq!(config.tools.arrow.line_style, LineStyle::Solid);
    assert_eq!(config.tools.line.width, 1.0);
    assert_eq!(config.tools.line.style, LineStyle::Solid);
    assert_eq!(
        read("[tools.line]\nwidth = 1000.0\n").tools.line.width,
        72.0
    );
}

/// 【配置测试】【序号校验】非法样式和非有限笔宽分别回退，合法的同节属性不丢失，序号上限不复用普通笔宽上限
#[test]
fn invalid_number_fields_fall_back_without_discarding_valid_fields() {
    let configured = read("[tools.number]\nwidth = 60.0\nstyle = \"missing\"\n");
    assert_eq!(configured.tools.number.width, 60.0);
    assert_eq!(
        configured.tools.number.style,
        crate::features::number::NumberStyle::Arabic
    );
    let invalid = read("[tools.number]\nwidth = nan\nstyle = \"chinese\"\n");
    assert_eq!(invalid.tools.number.width, 3.0);
    assert_eq!(
        invalid.tools.number.style,
        crate::features::number::NumberStyle::Chinese
    );
    assert_eq!(
        read("[tools.number]\nwidth = 1000.0\n").tools.number.width,
        72.0
    );
    assert_eq!(
        read("[tools.number]\nwidth = -1.0\n").tools.number.width,
        1.0
    );
}

/// 【配置测试】【尺子默认】尺子保留三像素独立笔宽与长度默认模式，未知模式仅回退自身字段
#[test]
fn ruler_defaults_modes_and_widths_remain_independent() {
    let defaults = Config::default();
    assert_eq!(defaults.tools.ruler.width, 3.0);
    assert_eq!(
        defaults.tools.ruler.mode,
        crate::features::ruler::RulerMode::Length
    );
    let config = read("[tools.ruler]\nwidth = 8.0\nmode = \"angle\"\n");
    assert_eq!(config.tools.ruler.width, 8.0);
    assert_eq!(
        config.tools.ruler.mode,
        crate::features::ruler::RulerMode::Angle
    );
    assert_eq!(config.tools.marker, defaults.tools.marker);
    assert_eq!(config.tools.stroke, defaults.tools.stroke);
    let invalid = read("[tools.ruler]\nwidth = 9.0\nmode = \"missing\"\n");
    assert_eq!(invalid.tools.ruler.mode, defaults.tools.ruler.mode);
    assert_eq!(invalid.tools.ruler.width, 9.0);
}

/// 【配置测试】【放大镜默认】双框放大镜独立保存倍率、框形和边框宽度，不修改光标取色放大镜或其他工具
#[test]
fn annotation_magnifier_settings_remain_independent_from_the_cursor_loupe() {
    let defaults = Config::default();
    assert_eq!(defaults.tools.magnifier.width, 3.0);
    assert_eq!(defaults.tools.magnifier.zoom, 2.75);
    assert_eq!(
        defaults.tools.magnifier.shape,
        crate::features::magnifier::MagnifierShape::Circle
    );
    let configured = read("[tools.magnifier]\nwidth = 9.0\nzoom = 4.25\nshape = \"rectangle\"\n");
    assert_eq!(configured.tools.magnifier.width, 9.0);
    assert_eq!(configured.tools.magnifier.zoom, 4.25);
    assert_eq!(
        configured.tools.magnifier.shape,
        crate::features::magnifier::MagnifierShape::Rectangle
    );
    assert_eq!(configured.magnifier, defaults.magnifier);
    assert_eq!(configured.tools.marker, defaults.tools.marker);
    assert_eq!(configured.tools.ruler, defaults.tools.ruler);
}

/// 【配置测试】【放大镜校验】非法形状、非有限数值和倍率范围各自回退，不丢弃同一节其他合法参数
#[test]
fn invalid_magnifier_fields_fall_back_without_discarding_valid_fields() {
    let configured = read("[tools.magnifier]\nwidth = 8.0\nzoom = 20.0\nshape = \"missing\"\n");
    assert_eq!(configured.tools.magnifier.width, 8.0);
    assert_eq!(configured.tools.magnifier.zoom, 6.0);
    assert_eq!(
        configured.tools.magnifier.shape,
        crate::features::magnifier::MagnifierShape::Circle
    );
    let invalid = read("[tools.magnifier]\nwidth = nan\nzoom = inf\nshape = \"rectangle\"\n");
    assert_eq!(invalid.tools.magnifier.width, 3.0);
    assert_eq!(invalid.tools.magnifier.zoom, 2.75);
    assert_eq!(
        invalid.tools.magnifier.shape,
        crate::features::magnifier::MagnifierShape::Rectangle
    );
}

/// 【配置测试】【非法形状】未知形状名称单独回退，其他合法标记配置继续生效
#[test]
fn an_unknown_marker_shape_does_not_discard_other_marker_options() {
    let config = read("[tools.marker]\nshape = \"missing\"\nfilled = false\nwidth = 8.0\n");
    assert_eq!(
        config.tools.marker.shape,
        crate::types::MarkerShape::Triangle
    );
    assert!(!config.tools.marker.filled);
    assert_eq!(config.tools.marker.width, 8.0);
}

#[test]
fn a_bad_line_is_skipped_and_the_rest_applies() {
    let config = read(
        "[toolbar]\nbutton_size = \"big\"\nbutton_gap = 9.0\n[theme]\naccent = \"nope\"\nhover = \"#112233\"\n",
    );
    assert_eq!(config.toolbar.button_size, Toolbar::default().button_size);
    assert_eq!(config.toolbar.button_gap, 9.0);
    assert_eq!(config.theme.accent, Theme::default().accent);
    assert_eq!(config.theme.hover, Rgba(0x11, 0x22, 0x33, 255));
}

#[test]
fn colors_parse_with_and_without_alpha() {
    let config = read("[theme]\nhover = \"#112233\"\nbackground = \"#11223344\"\n");
    assert_eq!(config.theme.hover, Rgba(0x11, 0x22, 0x33, 255));
    assert_eq!(config.theme.background, Rgba(0x11, 0x22, 0x33, 0x44));
}

#[test]
fn unset_colors_follow_the_theme() {
    let config = read(
        "[theme]\nforeground = \"#112233\"\n[toolbar]\nicon_selected = \"#445566\"\nicon_hovered = \"\"\n",
    );
    assert_eq!(config.toolbar.icon.get(), Rgba(0x11, 0x22, 0x33, 255));
    assert_eq!(
        config.toolbar.icon_selected.get(),
        Rgba(0x44, 0x55, 0x66, 255)
    );
    assert_eq!(config.toolbar.icon_hovered.get(), config.theme.on_accent);
}

#[test]
fn unknown_keys_are_ignored() {
    assert_eq!(
        read("[general]\nfuture_key = 1\n[nope]\nx = 1\n"),
        Config::default()
    );
}

#[test]
fn to_takes_letters_and_can_be_turned_off() {
    let config = read("[general]\nto = \"ps\"\n");
    assert_eq!(
        config.general.to,
        Outputs {
            pin: true,
            save: true,
            ..Outputs::default()
        }
    );
    assert!(read("[general]\nto = \"\"\n").general.to.is_empty());
    assert_eq!(
        read("[general]\nto = \"x\"\n").general.to,
        General::default().to
    );
}

#[test]
fn old_accept_and_save_always_become_to() {
    let old = "[general]\n\
            # by default both pin and copy still saves screenshot in the folder\n\
            save_always = true\n\
            # what to do with screenshot after finishing capture without an explicit bind\n\
            # like pressing enter or double click on the selection zone\n\
            # c (copy), p (pin), s(save), in any order\n\
            accept = \"p\"\n\
            \n\
            [keys]\n\
            accept = \"Space\"\n";
    for old in [old.to_owned(), old.replace('\n', "\r\n")] {
        let config = read(&migrate(&old).unwrap());
        assert_eq!(config.general.to, Outputs::parse("ps").unwrap());
        assert_eq!(config.keys.0["accept"].chords(), ["Space"]);
    }
}

#[test]
fn overlay_is_window_layer_or_auto() {
    let overlay = |value: &str| {
        read(&format!("[general]\noverlay = \"{value}\"\n"))
            .general
            .overlay
    };
    assert_eq!(overlay("window"), OverlayShell::Window);
    assert_eq!(overlay("layer"), OverlayShell::Layer);
    assert_eq!(overlay("fullscreen"), OverlayShell::Auto);
}

#[test]
fn keys_take_a_string_or_a_list() {
    let config = read("[keys]\nsave = \"Ctrl+Shift+S\"\nredo = [\"Ctrl+Y\", \"\"]\n");
    assert_eq!(config.keys.0["save"].chords(), ["Ctrl+Shift+S"]);
    assert_eq!(config.keys.0["redo"].chords(), ["Ctrl+Y"]);
}

#[test]
fn even_magnifier_cells_fall_back_to_the_default() {
    assert_eq!(
        read("[magnifier]\ncells = 20\n").magnifier.cells,
        Magnifier::default().cells
    );
}

#[test]
fn too_slow_animation_speed_falls_back_to_the_default() {
    assert_eq!(
        read("[animation]\nspeed = 0.0\n").animation.speed,
        Animation::default().speed
    );
}

#[test]
fn a_flipped_range_falls_back_to_the_default() {
    let config = read("[tools.stroke]\nmin = 50.0\nmax = 10.0\n");
    assert_eq!(config.tools.stroke, Stroke::default());
}

#[test]
fn magnifier_size_matches_cells_times_zoom() {
    assert_eq!(Magnifier::default().size(), 210);
}

/// 【贴图配置测试】【边框约束】原版默认边框保留两像素；超界与非有限宽度不产生异常绘制区域
#[test]
fn pinned_border_width_is_normalized() {
    assert_eq!(
        read("[pinned]\nborder_width = -3.0").pinned.border_width,
        0.0
    );
    assert_eq!(
        read("[pinned]\nborder_width = 100.0").pinned.border_width,
        16.0
    );
    assert_eq!(
        read("[pinned]\nborder_width = inf").pinned.border_width,
        2.0
    );
    let pinned = read("[pinned]\nalways_on_top = false\nborder_enabled = false").pinned;
    assert!(!pinned.always_on_top && !pinned.border_enabled);
}
