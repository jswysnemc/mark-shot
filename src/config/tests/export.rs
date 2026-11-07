use super::*;

/// 【外框配置测试】【原版字段】用 value 构造独立配置文档，返回外框参数，不读取全局配置
fn parse(value: Value) -> ImageFrame {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    read_at(&path).unwrap()
}

/// 【外框配置测试】【布尔开关】验证原版布尔、数字、字符串开关及缺省参数，无参数、无返回值
#[test]
fn compatible_switch_values_keep_the_original_defaults() {
    for (value, expected) in [
        (Value::Null, false),
        (Value::Bool(false), false),
        (Value::Bool(true), true),
        (serde_json::json!(0), false),
        (serde_json::json!(-2), true),
        (serde_json::json!(1e-13), false),
        (serde_json::json!(" ON "), true),
        (serde_json::json!("disabled"), false),
        (serde_json::json!("unknown"), false),
    ] {
        assert_eq!(
            parse(serde_json::json!({"export": {"imageFrame": value}})),
            ImageFrame {
                enabled: expected,
                ..ImageFrame::default()
            }
        );
    }
}

/// 【外框配置测试】【原版范围】验证整数、实数及字符串夹紧规则，不因未知参数或非有限字符串删除完整配置
#[test]
fn original_aliases_and_clamping_rules_apply_to_both_formats() {
    let directory = tempfile::tempdir().unwrap();
    let json = serde_json::json!({"export":{"imageFrame":{
        "show":"yes", "margin":-12, "radius":"-7", "shadowBlur":999,
        "shadowYOffset":"-4", "shadowAlpha":2.5, "unknown":"keep"
    }}});
    let toml = "[export.imageFrame]\nshow='yes'\nmargin=-12\nradius='-7'\nshadowBlur=999\nshadowYOffset='-4'\nshadowAlpha=2.5\nunknown='keep'";
    for (extension, text) in [("json", json.to_string()), ("toml", toml.into())] {
        let path = directory.path().join(format!("config.{extension}"));
        std::fs::write(&path, &text).unwrap();
        assert_eq!(
            read_at(&path).unwrap(),
            ImageFrame {
                enabled: true,
                padding: 0,
                corner_radius: 0.0,
                shadow_radius: 128,
                shadow_offset_y: 0,
                shadow_opacity: 1.0,
            }
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }
    let frame = ImageFrame::from_value(&serde_json::json!({
        "enabled":null, "show":true, "padding":null, "margin":5,
        "cornerRadius":"NaN", "shadowOpacity":"inf", "shadowRadius":4.5
    }));
    assert_eq!(
        frame,
        ImageFrame {
            shadow_radius: 0,
            ..ImageFrame::default()
        }
    );
}

/// 【外框配置测试】【完整配置】验证 TOML 的两种键名、布尔形式与默认模板能进入生产配置解析，无参数、无返回值
#[test]
fn native_configuration_accepts_tables_and_boolean_image_frame() {
    for text in [
        "[export]\nimage_frame=true",
        "[export]\nimageFrame=true",
        "[export.image_frame]\nenabled=true",
        "[export.imageFrame]\nenabled=true",
    ] {
        let config: crate::config::Config = toml::from_str(text).unwrap();
        assert!(config.export.image_frame.enabled);
    }
    let config: crate::config::Config = toml::from_str(crate::config::TEMPLATE).unwrap();
    assert_eq!(config.export, Export::default());
    assert_eq!(
        toml::from_str::<crate::config::Config>("[export.image_frame]\npadding=nan")
            .unwrap()
            .export,
        Export::default()
    );
}

/// 【外框配置测试】【最新读取】验证编辑期间改动文件立即生效，读取失败不改写配置，无参数、无返回值
#[test]
fn rereading_configuration_observes_changes_without_mutating_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "[export]\nimage_frame=false").unwrap();
    assert!(!read_at(&path).unwrap().enabled);
    std::fs::write(&path, "[export.image_frame]\nenabled=true\npadding=9").unwrap();
    assert_eq!(read_at(&path).unwrap().padding, 9);
    std::fs::write(&path, "invalid [").unwrap();
    assert!(read_at(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid [");
}
