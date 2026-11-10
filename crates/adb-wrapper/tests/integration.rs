use std::path::PathBuf;
use std::process::Command;

fn mock_adb_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mock_adb.cmd"
        ))
    } else {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mock_adb.sh"
        ))
    }
}

fn adb_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_adb"))
}

#[test]
fn passthrough_devices_to_mock() {
    let output = Command::new(adb_bin())
        .env("REAL_ADB", mock_adb_path())
        .args(["devices", "-l"])
        .output()
        .expect("run adb devices -l");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AAA111"));
    assert!(stdout.contains("BBB222"));
}

#[test]
fn non_interactive_multi_device_errors() {
    let output = Command::new(adb_bin())
        .env("REAL_ADB", mock_adb_path())
        .env_remove("ANDROID_SERIAL")
        .args(["shell", "getprop"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("run adb shell");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("non-interactive mode"));
}

#[test]
fn android_serial_env_passes_through_non_interactive() {
    let output = Command::new(adb_bin())
        .env("REAL_ADB", mock_adb_path())
        .env("ANDROID_SERIAL", "AAA111")
        .args(["shell", "getprop"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("run adb shell with ANDROID_SERIAL");

    assert!(output.status.success());
}

#[test]
fn wrapper_info_exits_zero() {
    let output = Command::new(adb_bin())
        .arg("--wrapper-info")
        .output()
        .expect("run adb --wrapper-info");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("adb-wrapper: active"));
    assert!(stdout.contains("wrapper version:"));
    assert!(stdout.contains("stdin tty:"));
}

#[test]
fn completions_bash_exits_zero() {
    let output = Command::new(adb_bin())
        .args(["completions", "bash"])
        .output()
        .expect("run adb completions bash");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("complete -F _adb adb"));
}

#[test]
fn real_adb_not_found_exits_one() {
    let output = Command::new(adb_bin())
        .env("REAL_ADB", "/nonexistent/adb")
        .arg("version")
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("run adb version");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("real adb not found"));
}

#[test]
fn real_adb_pointing_to_wrapper_is_rejected() {
    let output = Command::new(adb_bin())
        .env("REAL_ADB", adb_bin())
        .arg("version")
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("run adb version");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("real adb resolves to this wrapper"));
}
