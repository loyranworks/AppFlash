#[path = "../src/config.rs"]
mod config;
use config::{AppEntry, Config};

fn entry(letter: &str) -> AppEntry {
    AppEntry {
        id: "one".into(),
        name: "Chrome".into(),
        path: std::env::current_exe().unwrap().to_string_lossy().into(),
        letter: letter.into(),
    }
}

#[test]
fn defaults_are_shared_and_delay_does_not_gate_switching() {
    let config = Config::default();
    assert_eq!(config.hint.color, "#244b70");
    assert_eq!(serde_json::to_value(&config).unwrap()["hint"]["scale"], 100);
    assert_eq!(config.prefix, vec!["Alt"]);
    assert_eq!(config.hint.delay, 0.5);
    assert_eq!(config.hint.order, "manual");
    assert_eq!(config.language, "zh-CN");
    assert!(config.validate().is_ok());
}

#[test]
fn rejects_duplicates_bad_letters_and_invalid_delay() {
    let mut config = Config::default();
    config.apps.push(entry("A"));
    assert!(config.validate().is_ok());
    config.apps.push(entry("A"));
    assert!(config.validate().is_err());
    config.apps.pop();
    config.apps[0].letter = "AA".into();
    assert!(config.validate().is_err());
    config.apps[0].letter.clear();
    config.hint.delay = -1.0;
    assert!(config.validate().is_err());
}

#[test]
fn rejects_unknown_modifiers_bad_color_and_relative_path() {
    let mut config = Config::default();
    config.prefix = vec!["Unknown".into()];
    assert!(config.validate().is_err());
    config.prefix = vec!["Control".into()];
    config.hint.color = "url(remote)".into();
    assert!(config.validate().is_err());
    config.hint.color = "#71899a".into();
    config.apps.push(entry("B"));
    config.apps[0].path = "relative.exe".into();
    assert!(config.validate().is_err());
}

#[test]
fn legacy_arguments_are_removed_without_losing_bindings_or_custom_delay() {
    let directory =
        std::env::temp_dir().join(format!("app-flash-config-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("config.json");
    let mut value = Config::default();
    value.prefix = vec!["Control".into(), "Shift".into()];
    value.apps.push(entry("B"));
    value.apps[0].path = directory.join("missing.exe").to_string_lossy().into();
    value.hint.delay = 2.0;
    value.hint.color = "#81958a".into();
    let mut legacy = serde_json::to_value(&value).unwrap();
    legacy["apps"][0]["args"] = serde_json::json!(["--profile-directory=Work"]);
    legacy["hint"].as_object_mut().unwrap().remove("order");
    legacy["hint"].as_object_mut().unwrap().remove("scale");
    legacy.as_object_mut().unwrap().remove("language");
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let loaded = config::read(&path).unwrap();
    assert_eq!(loaded.prefix, value.prefix);
    assert_eq!(loaded.apps[0].path, value.apps[0].path);
    assert_eq!(loaded.hint.delay, 2.0);
    assert_eq!(loaded.hint.color, "#81958a");
    assert_eq!(serde_json::to_value(&loaded).unwrap()["hint"]["scale"], 100);
    assert_eq!(loaded.hint.order, "manual");
    assert_eq!(loaded.language, "zh-CN");
    assert!(serde_json::to_value(&loaded).unwrap()["apps"][0]
        .get("args")
        .is_none());
    assert_eq!(loaded.shortcuts().len(), 1);
    value.hint.color = "#81958a".into();
    value.hint.order = "alphabetical".into();
    value.hint.scale = 80;
    value.language = "en".into();
    config::write(&path, &value).unwrap();
    assert_eq!(config::read(&path).unwrap().hint.color, "#81958a");
    assert_eq!(config::read(&path).unwrap().hint.order, "alphabetical");
    assert_eq!(config::read(&path).unwrap().language, "en");
    assert_eq!(config::read(&path).unwrap().hint.scale, 80);
    value.hint.order = "unsupported".into();
    assert!(value.validate().is_err());
    value.hint.order = "manual".into();
    value.language = "unsupported".into();
    assert!(value.validate().is_err());
    legacy["apps"][0]["unexpected"] = serde_json::json!(true);
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert!(config::read(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}

#[test]
fn every_supported_prefix_and_letter_parses_as_a_native_shortcut() {
    let modifiers = ["Control", "Alt", "Shift", "Super"];
    for mask in 1..16 {
        let mut config = Config::default();
        config.prefix = modifiers
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, name)| name.to_string())
            .collect();
        for letter in b'A'..=b'Z' {
            config.apps = vec![entry(&(letter as char).to_string())];
            let (key, _) = config.shortcuts().pop().unwrap();
            assert!(
                key.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    .is_ok(),
                "{key}"
            );
        }
    }
}

#[test]
fn accepts_only_integer_scale_percentages_between_fifty_and_one_fifty() {
    for scale in [50, 80, 100, 150] {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value["hint"]["scale"] = serde_json::json!(scale);
        let config: Config = serde_json::from_value(value).unwrap();
        assert!(config.validate().is_ok(), "{scale}%");
        assert_eq!(
            serde_json::to_value(config).unwrap()["hint"]["scale"],
            scale
        );
    }
    for scale in [
        serde_json::json!(0),
        serde_json::json!(49),
        serde_json::json!(151),
        serde_json::json!(65536),
        serde_json::json!(-1),
        serde_json::json!(80.5),
        serde_json::json!("80"),
    ] {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value["hint"]["scale"] = scale.clone();
        let valid = serde_json::from_value::<Config>(value)
            .map(|config| config.validate().is_ok())
            .unwrap_or(false);
        assert!(!valid, "must reject {scale}");
    }
}
