use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_waybar_outputs_valid_json() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "").unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("waybar").arg("--config").arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"text\":"))
        .stdout(predicate::str::contains("\"tooltip\":"));
}

#[test]
fn test_config_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "[general]\ndefault_view = \"upcoming\"\n").unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("config").arg("--config").arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("default_view = \"upcoming\""));
}

#[test]
fn test_list_command_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let todo_path = temp_dir.path().join("todo.txt");

    fs::write(
        &config_path,
        format!(
            "[backends.local]\nenabled = true\npath = \"{}\"\n",
            todo_path.to_string_lossy()
        ),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("list").arg("all").arg("--config").arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_list_command_with_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let todo_path = temp_dir.path().join("todo.txt");

    fs::write(&todo_path, "Test task 1\nTest task 2\n").unwrap();
    fs::write(
        &config_path,
        format!(
            "[backends.local]\nenabled = true\npath = \"{}\"\n",
            todo_path.to_string_lossy()
        ),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("list").arg("all").arg("--config").arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test task 1"))
        .stdout(predicate::str::contains("Test task 2"));
}

#[test]
fn test_list_command_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let todo_path = temp_dir.path().join("todo.txt");

    fs::write(&todo_path, "Test task\n").unwrap();
    fs::write(
        &config_path,
        format!(
            "[backends.local]\nenabled = true\npath = \"{}\"\n",
            todo_path.to_string_lossy()
        ),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("list")
        .arg("all")
        .arg("--format")
        .arg("json")
        .arg("--config")
        .arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("Test task"));
}

#[test]
fn test_waybar_with_todo_txt() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let todo_path = temp_dir.path().join("todo.txt");

    // Use future date to avoid overdue
    let future_date = chrono::Local::now().date_naive() + chrono::Duration::days(30);
    fs::write(
        &todo_path,
        format!("(p1) Test task 1\n(p2) Test task 2 due:{}\n", future_date),
    )
    .unwrap();

    let config = format!(
        "[backends.local]\nenabled = true\npath = \"{}\"\n",
        todo_path.to_string_lossy()
    );
    fs::write(&config_path, config).unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("waybar").arg("--config").arg(&config_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"text\":\"2\""))
        .stdout(predicate::str::contains("has-tasks"));
}

#[test]
fn test_help_command() {
    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("タスキ"))
        .stdout(predicate::str::contains("waybar"))
        .stdout(predicate::str::contains("tui"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_no_backends_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "").unwrap();

    let mut cmd = cargo_bin_cmd!("tasuki");
    cmd.arg("list").arg("--config").arg(&config_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No backends enabled"));
}
