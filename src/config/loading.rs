use super::*;

pub(super) fn complain(message: String) {
    if let Ok(mut warnings) = WARNINGS.lock() {
        warnings.push(message);
    }
}

/// Logs what went wrong while reading the config; call once the logger is up.
pub fn log_warnings() {
    let warnings = WARNINGS
        .lock()
        .map(|mut w| std::mem::take(&mut *w))
        .unwrap_or_default();
    for message in warnings {
        info!("{message}");
    }
}

pub(super) fn config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("mark-shot").join("config.toml"))
}

pub(super) const MAX_BAD_LINES: usize = 64;

pub(super) fn parse(text: &str, origin: &str) -> Config {
    let mut text = text.to_owned();
    for _ in 0..MAX_BAD_LINES {
        match toml::from_str::<Config>(&text) {
            Ok(mut config) => {
                warn_unknown_keys(&text, origin);
                config.resolve();
                validate(&mut config);
                return config;
            }
            Err(e) => {
                let Some(span) = e.span() else { break };
                let start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
                let end = text[span.start..]
                    .find('\n')
                    .map_or(text.len(), |i| span.start + i);
                let line = text[..start].matches('\n').count() + 1;
                complain(format!(
                    "config: {origin}:{line}: {}, skipping `{}`",
                    e.message().trim(),
                    text[start..end].trim()
                ));
                text.replace_range(start..end, "");
            }
        }
    }
    complain(format!(
        "config: {origin} can't be read, using the defaults"
    ));
    Config::default()
}

/// Options without a default value
pub(super) const UNSET_BY_DEFAULT: &[&str] = &[
    "log.level",
    "log.path",
    "tools.redact.mode",
    "tools.redact.pixelate",
];

pub(super) fn warn_unknown_keys(text: &str, origin: &str) {
    let (Ok(user), Ok(known)) = (
        text.parse::<toml::Table>(),
        toml::Table::try_from(Config::default()),
    ) else {
        return;
    };
    pub(super) fn walk(user: &toml::Table, known: &toml::Table, path: &str, origin: &str) {
        for (key, value) in user {
            let full = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            match (known.get(key), value) {
                (None, _) if UNSET_BY_DEFAULT.contains(&full.as_str()) => {}
                (None, _) => complain(format!("config: {origin}: unknown key {full}, ignoring it")),
                // keys.rs names the unknown actions itself
                (Some(_), _) if full == "keys" => {}
                (Some(toml::Value::Table(known)), toml::Value::Table(user)) => {
                    walk(user, known, &full, origin)
                }
                _ => {}
            }
        }
    }
    walk(&user, &known, "", origin);
}

/// 【配置】【参数校验】约束 config 的数值范围，非法参数回退到对应默认值；无返回值
pub(super) fn validate(config: &mut Config) {
    config.history.normalize();
    config.pinned.border_width = if config.pinned.border_width.is_finite() {
        config.pinned.border_width.clamp(0.0, 16.0)
    } else {
        Pinned::default().border_width
    };
    config.tools.arrow.width = crate::features::line::clamped_width(config.tools.arrow.width, 1.0);
    config.tools.line.width = crate::features::line::clamped_width(config.tools.line.width, 1.0);
    config.tools.number.width =
        crate::features::number::clamped_width(config.tools.number.width, 1.0);
    config.tools.magnifier.zoom =
        crate::features::magnifier::clamped_zoom(config.tools.magnifier.zoom);
    if !config.tools.magnifier.width.is_finite() || config.tools.magnifier.width <= 0.0 {
        complain("【放大镜】【配置校验】边框宽度必须为有限正数，使用默认值".to_string());
        config.tools.magnifier.width = MagnifierTool::default().width;
    }
    let magnifier = &mut config.magnifier;
    if magnifier.cells.is_multiple_of(2) {
        complain("config: magnifier.cells must be odd, using the default".to_string());
        magnifier.cells = Magnifier::default().cells;
    }
    let animation = &mut config.animation;
    if animation.speed < 0.1 {
        complain("config: animation.speed must be at least 0.1, using the default".to_string());
        animation.speed = Animation::default().speed;
    }
    if animation.frame_ms == 0 {
        complain("config: animation.frame_ms must be at least 1, using the default".to_string());
        animation.frame_ms = Animation::default().frame_ms;
    }
    let pen = &mut config.tools.pen;
    if !(0.0..1.0).contains(&pen.smoothing) {
        complain(
            "config: tools.pen.smoothing must be at least 0 and below 1, using the default"
                .to_string(),
        );
        pen.smoothing = Pen::default().smoothing;
    }
    let highlighter = &mut config.tools.highlighter;
    if !(0.0..1.0).contains(&highlighter.smoothing) {
        complain(
            "config: tools.highlighter.smoothing must be at least 0 and below 1, using the default"
                .to_string(),
        );
        highlighter.smoothing = Highlighter::default().smoothing;
    }

    // clamp() panics when min > max, so a flipped range falls back as a whole
    let stroke = &mut config.tools.stroke;
    if stroke.min > stroke.max || stroke.step <= 0.0 {
        complain(
            "config: tools.stroke needs min <= max and a positive step, using the defaults"
                .to_string(),
        );
        *stroke = Stroke::default();
    }
    let font = &mut config.tools.font;
    if font.min > font.max || font.step <= 0.0 {
        complain(
            "config: tools.font needs min <= max and a positive step, using the defaults"
                .to_string(),
        );
        *font = Font::default();
    }
    let redact = &mut config.tools.redact;
    redact.strength = crate::features::filter::clamped_strength(redact.strength);
    if !redact.step.is_finite() || redact.step <= 0.0 {
        complain("【滤镜】【配置校验】强度步长必须为有限正数，使用默认值".to_string());
        redact.step = Redact::default().step;
    }
    let text = &mut config.annotations.text;
    if text.resize_min > text.resize_max {
        complain(
            "config: annotations.text.resize_min is above resize_max, using the defaults"
                .to_string(),
        );
        text.resize_min = TextBox::default().resize_min;
        text.resize_max = TextBox::default().resize_max;
    }
    if config.theme.line_height.is_nan() || config.theme.line_height <= 0.0 {
        complain("config: theme.line_height must be above 0, using the default".to_string());
        config.theme.line_height = Theme::default().line_height;
    }
    if config.annotations.text.line_height.is_nan() || config.annotations.text.line_height <= 0.0 {
        complain(
            "config: annotations.text.line_height must be above 0, using the default".to_string(),
        );
        config.annotations.text.line_height = TextBox::default().line_height;
    }
    let input = &mut config.input;
    if input.corner_min > input.corner_max {
        complain("config: input.corner_min is above corner_max, using the defaults".to_string());
        input.corner_min = Input::default().corner_min;
        input.corner_max = Input::default().corner_max;
    }
}

pub(super) fn write_template(path: &PathBuf) {
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, TEMPLATE));
    if let Err(e) = written {
        complain(format!("config: cannot write {}: {e}", path.display()));
    }
}

pub(super) const OLD_GENERAL_COMMENTS: &[&str] = &[
    "# by default both pin and copy still saves screenshot in the folder",
    "# what to do with screenshot after finishing capture without an explicit bind",
    "# like pressing enter or double click on the selection zone",
    "# c (copy), p (pin), s(save), in any order",
];

// honestly more of a temp solution
// i want to make a better migration system, but i guess it will be implemented later
pub(super) fn migrate(text: &str) -> Option<String> {
    let mut section = "";
    let mut drop = Vec::new();
    let mut accept = None;
    let mut save_always = None;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.split('#').next().unwrap_or("").trim();
            continue;
        }
        if section != "[general]" {
            continue;
        }
        let key = line.split('=').next().unwrap_or("").trim();
        match key {
            "to" => return None,
            "accept" | "save_always" => {
                let value = line.parse::<toml::Table>().ok()?.remove(key)?;
                if key == "accept" {
                    accept = Some(Outputs::parse(value.as_str()?).ok()?);
                } else {
                    save_always = Some(value.as_bool()?);
                }
                drop.push(i);
            }
            _ if OLD_GENERAL_COMMENTS.contains(&line) => drop.push(i),
            _ => {}
        }
    }
    if accept.is_none() && save_always.is_none() {
        return None;
    }
    let mut to = accept.unwrap_or(Outputs {
        copy: true,
        ..Outputs::default()
    });
    to.save |= save_always.unwrap_or(true);

    let comments = TEMPLATE
        .split_once("[general]\n")?
        .1
        .split_once("\nto = ")?
        .0;
    let block = format!("{comments}\nto = {}", toml::Value::try_from(to).ok()?);
    let mut lines: Vec<&str> = text.lines().collect();
    lines[drop[0]] = &block;
    for &i in drop[1..].iter().rev() {
        lines.remove(i);
    }
    Some(lines.join("\n") + "\n")
}

/// Config overide path set via `--config`. Only the first call takes effect
pub fn init(override_path: Option<PathBuf>, overrides: impl FnOnce(&mut Config)) {
    if let Some(path) = &override_path {
        let _ = OVERRIDE_PATH.set(std::path::absolute(path).unwrap_or_else(|_| path.clone()));
    }
    let path = override_path.or_else(config_path);
    let mut config = match &path {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(mut text) => {
                if let Some(migrated) = migrate(&text) {
                    match std::fs::write(path, &migrated) {
                        Ok(()) => complain(format!(
                            "config: {}: general.accept and general.save_always were replaced with general.to",
                            path.display()
                        )),
                        Err(e) => complain(format!(
                            "config: cannot update {}: {e}, replace general.accept and general.save_always with general.to there",
                            path.display()
                        )),
                    }
                    text = migrated;
                }
                parse(&text, &path.display().to_string())
            }
            Err(_) => {
                write_template(path);
                Config::default()
            }
        },
        None => Config::default(),
    };
    overrides(&mut config);
    let _ = CONFIG.set(config);
}

pub fn get() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

pub fn pass_to(command: &mut std::process::Command) {
    if let Some(path) = OVERRIDE_PATH.get() {
        command.arg(CONFIG_ARG).arg(path);
    }
}
