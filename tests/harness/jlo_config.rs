use std::fs;
use std::path::Path;

pub(crate) const DEFAULT_TEST_CRON: &str = "0 20 * * *";

pub(crate) fn write_jlo_config(root: &Path, crons: &[&str], wait_minutes_default: u32) {
    let jlo_dir = root.join(".jlo");
    fs::create_dir_all(&jlo_dir).expect("create .jlo/");

    let cron_entries =
        crons.iter().map(|cron| format!("\"{}\"", cron)).collect::<Vec<_>>().join(", ");

    let content = format!(
        r#"[run]
jlo_target_branch = "main"
jules_worker_branch = "jules"

[workflow]
cron = [{}]
wait_minutes_default = {}
"#,
        cron_entries, wait_minutes_default
    );

    fs::write(jlo_dir.join("config.toml"), content).expect("write .jlo/config.toml");
}

pub(crate) fn ensure_jlo_config(root: &Path) {
    let config_path = root.join(".jlo/config.toml");
    if !config_path.exists() {
        write_jlo_config(root, &[DEFAULT_TEST_CRON], 30);
    }
}
