use dirtrack::config::config_dir;

#[test]
fn test_config_dir_uses_xdg_config_home() {
    let tmp = tempfile::TempDir::new().unwrap();
    let custom = tmp.path().join("xdg-config");
    std::fs::create_dir_all(&custom).unwrap();

    let prev = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", &custom);

    assert_eq!(config_dir(), custom);

    if let Some(value) = prev {
        std::env::set_var("XDG_CONFIG_HOME", value);
    } else {
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}