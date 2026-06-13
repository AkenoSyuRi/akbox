use crate::device::{parse_devices_output, AdbDevice};
use crate::path::real_adb_display;
use anyhow::{Context, Result, bail};
use is_terminal::IsTerminal;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(windows)]
use std::sync::Once;

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

#[cfg(windows)]
static WINDOWS_CTRL_HANDLER_INIT: Once = Once::new();

#[cfg(windows)]
static CHILD_PROCESS_GROUP: AtomicU32 = AtomicU32::new(0);

pub fn list_online_devices(real_adb: &Path) -> Result<Vec<AdbDevice>> {
    let output = Command::new(real_adb)
        .args(["devices", "-l"])
        .output()
        .with_context(|| format!("adb-wrapper: failed to run real adb devices -l"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "adb-wrapper: failed to run real adb devices -l\n{}",
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_devices_output(&stdout))
}

pub fn run_adb(real_adb: &Path, args: &[String]) -> Result<ExitStatus> {
    #[cfg(windows)]
    if std::io::stdout().is_terminal() {
        return run_adb_windows(real_adb, args);
    }

    run_adb_simple(real_adb, args)
}

fn run_adb_simple(real_adb: &Path, args: &[String]) -> Result<ExitStatus> {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    Command::new(real_adb)
        .args(&arg_refs)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "adb-wrapper: failed to execute real adb: {}",
                real_adb_display(real_adb)
            )
        })
}

#[cfg(windows)]
fn run_adb_windows(real_adb: &Path, args: &[String]) -> Result<ExitStatus> {
    init_windows_ctrl_handler();

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut child = Command::new(real_adb)
        .args(&arg_refs)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .creation_flags(CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .with_context(|| {
            format!(
                "adb-wrapper: failed to execute real adb: {}",
                real_adb_display(real_adb)
            )
        })?;

    CHILD_PROCESS_GROUP.store(child.id(), Ordering::SeqCst);
    let status = child
        .wait()
        .context("adb-wrapper: failed to wait for real adb")?;
    CHILD_PROCESS_GROUP.store(0, Ordering::SeqCst);
    Ok(status)
}

#[cfg(windows)]
fn init_windows_ctrl_handler() {
    WINDOWS_CTRL_HANDLER_INIT.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            let process_group = CHILD_PROCESS_GROUP.load(Ordering::SeqCst);
            if process_group != 0 {
                send_ctrl_break_to_process_group(process_group);
            }
        });
    });
}

#[cfg(windows)]
fn send_ctrl_break_to_process_group(process_group: u32) {
    use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};

    // SAFETY: Win32 API call with a valid process group id from our child process.
    unsafe {
        let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group);
    }
}
