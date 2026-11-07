//! 【外部动作测试】【参数与生命周期】原版配置、引用、真实进程与列表输入回归，不连接用户桌面

use mark_shot::{
    features::external_actions::{
        Action,
        desktop_apps::{self, DesktopApp},
        desktop_exec, extensions, launch,
    },
    keys::Key,
    types::MouseButton,
    ui::{
        external_panel::{ExternalPanel, Kind, layout::Layout, state::Event},
        panel::UiPanel,
    },
};
use std::path::Path;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

/// 【外部动作测试】【桌面样本】按 exec 返回独立桌面应用，不扫描用户系统目录
fn app(exec: &str) -> DesktopApp {
    DesktopApp {
        name: "Image Editor".into(),
        desktop_path: "/apps/image editor.desktop".into(),
        exec: exec.into(),
        icon: String::new(),
    }
}

/// 【外部动作测试】【扩展样本】按 command 返回默认关闭、不额外保存图片的扩展配置
fn extension(command: &str) -> extensions::ExtensionCommand {
    extensions::parse(&serde_json::json!({"name":"Command", "command":command}).to_string())
        .unwrap()
        .remove(0)
}

/// 【外部动作测试】【进程结果】在限时内读取 path 完整 JSON，返回值；原生子进程未写入时报告失败
#[cfg(target_os = "linux")]
fn result(path: &Path) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(value) = serde_json::from_slice(&bytes)
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "external process did not write {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 【外部动作测试】【桌面过滤】隐藏、无图片 MIME、非应用和动作分组不进入原版打开方式列表
#[test]
fn desktop_entries_filter_visibility_mime_and_action_sections() {
    let text = "[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer %f\nIcon=view\nMimeType=image/jpeg;image/png;\n[Desktop Action Edit]\nName=Wrong\nExec=wrong\n";
    let parsed = desktop_apps::parse(text, Path::new("/test.desktop")).unwrap();
    assert_eq!(parsed.name, "Viewer");
    assert_eq!(parsed.exec, "viewer %f");
    for (key, value) in [
        ("Hidden", "TRUE"),
        ("NoDisplay", "1"),
        ("Type", "Link"),
        ("MimeType", "text/plain"),
        ("Name", ""),
        ("Exec", ""),
    ] {
        let changed = format!(
            "[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer\nMimeType=image/*\n{key}={value}"
        );
        // 1. 【外部动作测试】【首次字段】原版保留首次出现字段，样本直接替换原字段而非使用重复键覆盖
        let changed = changed.replacen(
            &format!(
                "{key}={}",
                match key {
                    "Type" => "Application",
                    "Name" => "Viewer",
                    "Exec" => "viewer",
                    "MimeType" => "image/*",
                    _ => "unused",
                }
            ),
            "",
            1,
        );
        assert!(
            desktop_apps::parse(&changed, Path::new("/test.desktop")).is_none(),
            "{key}={value}"
        );
    }
}

/// 【外部动作测试】【应用目录】多级目录可发现应用，同路径不会重复，符号链接目录环不会阻塞枚举
#[test]
fn discovery_is_recursive_deduplicated_and_does_not_follow_directory_loops() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("nested")).unwrap();
    for (name, directory) in [
        ("Zulu", root.path().to_owned()),
        ("Alpha", root.path().join("nested")),
    ] {
        std::fs::write(
            directory.join(format!("{name}.desktop")),
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nExec=viewer\nMimeType=image/png"
            ),
        )
        .unwrap();
    }
    #[cfg(target_os = "linux")]
    std::os::unix::fs::symlink(root.path(), root.path().join("nested/loop")).unwrap();
    #[cfg(target_os = "windows")]
    std::os::windows::fs::symlink_dir(root.path(), root.path().join("nested/loop")).unwrap();
    let apps = desktop_apps::discover(&[root.path().to_owned(), root.path().to_owned()]);
    assert_eq!(
        apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
        ["Alpha", "Zulu"]
    );
}

/// 【外部动作测试】【桌面参数】文件、地址、名称、入口和转义字段保持单参数，未声明文件字段时自动附加图片
#[test]
fn desktop_field_codes_preserve_argument_boundaries() {
    let image = Path::new("/tmp/a b%u'$(echo bad).png");
    let args = desktop_exec::expand(
        &app("\"/opt/Image Editor\" --name %c --entry %k %i --file=%f --url=%U %% %d"),
        image,
    )
    .unwrap();
    assert_eq!(
        args,
        [
            "/opt/Image Editor",
            "--name",
            "Image Editor",
            "--entry",
            "/apps/image editor.desktop",
            "--file=/tmp/a b%u'$(echo bad).png",
            "--url=file:///tmp/a b%25u'$(echo bad).png",
            "%"
        ]
    );
    assert_eq!(
        desktop_exec::expand(&app("viewer --fullscreen"), image).unwrap(),
        ["viewer", "--fullscreen", image.to_str().unwrap()]
    );
    assert_eq!(
        desktop_exec::split("program \"a b\" \"\"\"literal\"\"\" 'a b'"),
        ["program", "a b", "\"literal\"", "'a", "b'"]
    );
    assert!(desktop_exec::expand(&app("%i %d"), image).is_err());
}

/// 【外部动作测试】【配置形式】数组、单对象、commands、字段别名和默认值与原版一致，非法字段类型不启用动作
#[test]
fn extension_config_supports_all_original_shapes_and_defaults() {
    let object = serde_json::json!({"name":" Sample ","command":" echo {slurp} ","cwd":" /tmp ","description":" Info ","closeOnStart":false});
    let plain = extensions::parse(&object.to_string()).unwrap();
    assert_eq!(
        plain,
        extensions::parse(&serde_json::json!([object.clone()]).to_string()).unwrap()
    );
    assert_eq!(
        plain,
        extensions::parse(&serde_json::json!({"commands":[object]}).to_string()).unwrap()
    );
    assert_eq!(plain[0].name, "Sample");
    assert_eq!(plain[0].description, "Info");
    assert_eq!(plain[0].working_directory, "/tmp");
    assert!(!plain[0].save_image);
    assert!(!plain[0].close_on_start);
    for field in ["{image}", "{imagePath}", "{imageUrl}"] {
        let command = extension(&format!("viewer {field}"));
        assert!(command.save_image && command.close_on_start);
    }
    let commands = extensions::parse(r#"[{"name":"Alias","command":"viewer","needsImage":true,"saveImage":"true","closeOnStart":"false"},17,{"name":42,"command":"bad"},{"name":"Empty","command":" "}]"#).unwrap();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].save_image && commands[0].close_on_start);
    for text in ["null", "[]", "{}", "{broken"] {
        assert!(extensions::parse(text).is_err(), "{text}");
    }
    let root = tempfile::tempdir().unwrap();
    assert!(
        extensions::load(&root.path().join("absent.json"))
            .unwrap()
            .is_empty()
    );
}

/// 【外部动作测试】【命令引用】图片和区域包含 shell 元字符时仍作为数据，替换后的字段不能再次展开
#[test]
fn extension_substitution_quotes_values_once() {
    let image = Path::new("/tmp/quote' {slurp} $(touch bad).png");
    let command = extension("viewer {image} {imagePath} {imageUrl} {slurp}");
    let expanded = launch::expand_extension(&command, Some(image), "-40,20 100x80").unwrap();
    assert_eq!(
        expanded,
        format!(
            "viewer {} {} {} {}",
            launch::shell_quote(image.to_str().unwrap()),
            launch::shell_quote(image.to_str().unwrap()),
            launch::shell_quote(&desktop_exec::file_url(image).unwrap()),
            launch::shell_quote("-40,20 100x80")
        )
    );
    assert!(launch::expand_extension(&command, None, "0,0 20x20").is_err());
    assert!(launch::expand_extension(&extension("echo {slurp}"), None, "").is_err());
    assert_eq!(
        launch::expand_extension(&extension("echo {slurp}"), None, "0,0 20x20").unwrap(),
        "echo '0,0 20x20'"
    );
}

/// 【外部动作测试】【实际子进程】桌面直接参数和扩展 shell 参数、工作目录与图片完整交付，外部进程脱离父进程等待关系
#[test]
#[cfg(target_os = "linux")]
fn real_external_process_receives_original_png_arguments_and_cwd() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    let capture = root.path().join("capture' {slurp} $(bad).png");
    let png = tiny_skia::Pixmap::new(31, 17)
        .unwrap()
        .encode_png()
        .unwrap();
    std::fs::write(&capture, &png).unwrap();
    let script = root.path().join("receiver.py");
    std::fs::write(&script, "import sys, os, json, pathlib\na=sys.argv[2:]\npathlib.Path(sys.argv[1]).write_text(json.dumps({'args':a,'cwd':os.getcwd(),'png':pathlib.Path(a[0]).read_bytes().hex(),'ppid':os.getppid()}))\n").unwrap();
    let python = "/usr/bin/python3";
    let first = root.path().join("desktop.json");
    let mut desktop = app(&format!(
        "{python} \"{}\" \"{}\" %f %c",
        script.display(),
        first.display()
    ));
    desktop.name = "Image receiver".into();
    launch::start(&Action::Desktop(desktop), Some(&capture), "-40,20 100x80").unwrap();
    let received = result(&first);
    assert_eq!(
        received["args"],
        serde_json::json!([capture.to_str().unwrap(), "Image receiver"])
    );
    assert_eq!(
        received["png"],
        png.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    assert_ne!(received["ppid"], std::process::id());
    let second = root.path().join("extension.json");
    let mut command = extension(&format!(
        "{python} {} {} {{image}} {{slurp}}",
        launch::shell_quote(script.to_str().unwrap()),
        launch::shell_quote(second.to_str().unwrap())
    ));
    command.working_directory = root.path().to_str().unwrap().into();
    command.close_on_start = false;
    launch::start(&Action::Extension(command), Some(&capture), "-40,20 100x80").unwrap();
    let received = result(&second);
    assert_eq!(
        received["args"],
        serde_json::json!([capture.to_str().unwrap(), "-40,20 100x80"])
    );
    assert_eq!(received["cwd"], root.path().to_str().unwrap());
    let temporary = launch::temporary_image(&png, root.path()).unwrap();
    let path = temporary.path().to_owned();
    assert_eq!(
        temporary.as_file().metadata().unwrap().mode() & 0o777,
        0o600
    );
    let error = launch::start(
        &Action::Desktop(app("/absent/mark-shot-viewer %f")),
        Some(&path),
        "0,0 31x17",
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    drop(temporary);
    assert!(!path.exists());
    let mut command = extension("echo data");
    command.working_directory = root.path().join("absent").to_str().unwrap().into();
    assert!(launch::start(&Action::Extension(command), None, "0,0 31x17").is_err());
}

/// 【外部动作测试】【列表样本】返回 count 行扩展面板，position 和 monitor 固定用于输入隔离验证
fn panel(count: usize) -> ExternalPanel {
    let actions = (0..count)
        .map(|index| {
            let mut command = extension("echo data");
            command.name = format!("Command {index}");
            Action::Extension(command)
        })
        .collect();
    ExternalPanel::open(
        Kind::Extensions,
        actions,
        String::new(),
        Layout::new(Kind::Extensions, count, 90.0, 36.0, (640.0, 480.0)),
        (12.0, 18.0),
        0,
    )
}

/// 【外部动作测试】【输入职责】左键完整点击才交付动作，右键、异屏、空白、拖出及滚动中断不执行
#[test]
fn panel_separates_buttons_and_requires_matching_press_release() {
    let mut panel = panel(3);
    let point = (50.0, 60.0);
    assert_eq!(panel.hit(0, point), Some(0));
    assert_eq!(
        panel.button(0, point, MouseButton::Right, true),
        Event::None
    );
    assert_eq!(
        panel.button(0, point, MouseButton::Right, false),
        Event::None
    );
    assert_eq!(
        panel.button(0, point, MouseButton::Left, false),
        Event::None
    );
    assert_eq!(panel.button(0, point, MouseButton::Left, true), Event::None);
    assert_eq!(
        panel.button(1, point, MouseButton::Left, false),
        Event::None
    );
    panel.button(0, point, MouseButton::Left, true);
    assert_eq!(
        panel.button(0, (50.0, 97.0), MouseButton::Left, false),
        Event::None
    );
    panel.button(0, point, MouseButton::Left, true);
    assert_eq!(
        panel.button(0, point, MouseButton::Left, false),
        Event::Activate(0)
    );
    assert_eq!(
        panel.button(0, (0.0, 0.0), MouseButton::Left, true),
        Event::Close
    );
}

/// 【外部动作测试】【完整列表】键盘和滚动可访问超过十行的命令，滑块只改变范围，极小输出仍保留面板边界
#[test]
fn long_lists_scroll_navigate_and_stay_within_small_outputs() {
    let mut panel = panel(24);
    assert!(panel.max_offset() > 0.0);
    assert!(panel.thumb().is_some());
    assert_eq!(panel.key(Key::Enter), Event::None);
    assert_eq!(panel.key(Key::End), Event::Changed);
    assert_eq!(panel.active, Some(23));
    assert_eq!(panel.offset, panel.max_offset());
    assert_eq!(panel.key(Key::Enter), Event::Activate(23));
    panel.key(Key::Home);
    assert_eq!(panel.active, Some(0));
    assert_eq!(panel.offset, 0.0);
    panel.button(0, (50.0, 60.0), MouseButton::Left, true);
    panel.scroll_to(100.0);
    assert_eq!(
        panel.button(0, (50.0, 60.0), MouseButton::Left, false),
        Event::None
    );
    assert_eq!(panel.key(Key::Escape), Event::Close);
    let layout = Layout::new(Kind::OpenWith, 24, 500.0, 40.0, (200.0, 150.0));
    assert!(layout.size.0 <= 184.0 && layout.size.1 <= 134.0);
    let anchor = tiny_skia::Rect::from_xywh(170.0, 60.0, 22.0, 50.0).unwrap();
    let position = layout.position(anchor, (200.0, 150.0));
    assert!(position.0 >= 8.0 && position.0 + layout.size.0 <= 192.0);
    assert!(position.1 >= 8.0 && position.1 + layout.size.1 <= 142.0);
    assert!(panel.rect().unwrap().width() >= 300.0);
}

/// 【外部动作测试】【真实 Qt 对照】比较未修改 Qt 函数采集的参数、URL、配置和非空列表几何，不能以 Rust 自身输出作为期望
#[test]
fn original_qt_function_samples_match_commands_config_and_nonempty_geometry() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/qt-external-actions.json")).unwrap();
    for sample in reference["exec"].as_array().unwrap() {
        let args = desktop_exec::expand(
            &app(sample["exec"].as_str().unwrap()),
            Path::new("/tmp/image name.png"),
        )
        .unwrap();
        assert_eq!(serde_json::json!(args), sample["args"]);
    }
    for sample in reference["urls"].as_array().unwrap() {
        assert_eq!(
            desktop_exec::file_url(Path::new(sample["path"].as_str().unwrap())).unwrap(),
            sample["url"].as_str().unwrap()
        );
    }
    for sample in reference["configurations"].as_array().unwrap() {
        let commands = extensions::parse(sample["source"].as_str().unwrap());
        if !sample["error"].as_str().unwrap().is_empty() {
            assert!(commands.is_err());
            continue;
        }
        let commands: Vec<_> = commands.unwrap().iter().map(|command| serde_json::json!({"name":command.name,"command":command.command,
            "working_directory":command.working_directory,"description":command.description,"save_image":command.save_image,"close_on_start":command.close_on_start})).collect();
        assert_eq!(serde_json::json!(commands), sample["commands"]);
    }
    for sample in reference["panels"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sample| sample["count"] != 0)
    {
        let kind = if sample["kind"] == "open" {
            Kind::OpenWith
        } else {
            Kind::Extensions
        };
        let count = sample["count"].as_u64().unwrap() as usize;
        let layout = Layout::new(kind, count, 90.0, 36.0, (1067.0, 667.0));
        let anchor = tiny_skia::Rect::from_xywh(960.0, 220.0, 40.0, 440.0).unwrap();
        let position = layout.position(anchor, (1067.0, 667.0));
        assert_eq!(
            serde_json::json!([
                position.0 as u32,
                position.1 as u32,
                layout.size.0 as u32,
                layout.size.1 as u32
            ]),
            sample["panel"]
        );
        assert_eq!(
            serde_json::json!([
                layout.list.x() as u32,
                layout.list.y() as u32,
                layout.list.width() as u32,
                layout.list.height() as u32
            ]),
            sample["list"]["rect"]
        );
        assert_eq!(
            (count as f32 * mark_shot::ui::external_panel::layout::ROW - layout.list.height())
                .max(0.0) as u32,
            sample["list"]["scroll_max"].as_u64().unwrap() as u32
        );
    }
}
