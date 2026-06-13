mod adb;
mod args;
mod completions;
mod config;
mod device;
mod diagnostics;
mod path;
mod selector;
mod tty;

use adb::{list_online_devices, run_adb};
use anyhow::Result;
use args::{preprocess_args, WrapperArgs};
use completions::run_completions;
use diagnostics::run_wrapper_info;
use config::{load_config, save_last_serial, Config};
use device::AdbDevice;
use path::resolve_real_adb;
use selector::prompt_select_device;
use std::process::ExitCode;
use tty::is_interactive;

const MANAGEMENT_COMMANDS: &[&str] = &[
    "devices",
    "version",
    "help",
    "start-server",
    "kill-server",
    "reconnect",
];

const WRAPPER_COMMANDS: &[&str] = &["completions"];

const DEVICE_SELECTOR_FLAGS: &[&str] = &["-s", "-d", "-e"];

pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::from(1)
        }
    }
}

fn run_inner() -> Result<u8> {
    let WrapperArgs {
        force_pick,
        wrapper_info,
        adb_args,
    } = preprocess_args(std::env::args().skip(1).collect());

    if wrapper_info {
        return run_wrapper_info();
    }

    if is_wrapper_command(&adb_args) {
        return run_completions(adb_args.get(1..).unwrap_or_default());
    }

    let config = load_config();
    let real_adb = resolve_real_adb(&config)?;

    if should_passthrough(&adb_args) || has_device_selector(&adb_args) {
        return exit_from_status(run_adb(&real_adb, &adb_args)?);
    }

    let devices = list_online_devices(&real_adb)?;
    if devices.len() <= 1 {
        return exit_from_status(run_adb(&real_adb, &adb_args)?);
    }

    if !is_interactive() {
        eprintln!("adb-wrapper: multiple devices found in non-interactive mode.");
        eprintln!("Please specify device explicitly: adb -s <serial> ...");
        return Ok(1);
    }

    let serial = match resolve_serial(&devices, &config, force_pick) {
        Ok(serial) => serial,
        Err(err) if is_selection_cancelled(&err) => {
            eprintln!("{err}");
            return Ok(130);
        }
        Err(err) => return Err(err),
    };

    let mut final_args = vec!["-s".to_string(), serial];
    final_args.extend(adb_args);
    exit_from_status(run_adb(&real_adb, &final_args)?)
}

fn resolve_serial(
    devices: &[AdbDevice],
    config: &Config,
    force_pick: bool,
) -> Result<String> {
    if force_pick {
        return pick_and_remember(devices, config);
    }

    if config.prefer_last_device {
        if let Some(last_serial) = config.last_serial.as_ref() {
            if devices.iter().any(|device| &device.serial == last_serial) {
                return Ok(last_serial.clone());
            }
        }
    }

    pick_and_remember(devices, config)
}

fn pick_and_remember(devices: &[AdbDevice], config: &Config) -> Result<String> {
    let serial = prompt_select_device(devices, config.show_model, config.selector.clone())?;
    save_last_serial(&serial);
    Ok(serial)
}

fn exit_from_status(status: std::process::ExitStatus) -> Result<u8> {
    Ok(status.code().unwrap_or(1) as u8)
}

pub fn should_passthrough(args: &[String]) -> bool {
    if args.is_empty() {
        return true;
    }
    let first_command = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .map(String::as_str);
    match first_command {
        Some(cmd) => MANAGEMENT_COMMANDS.contains(&cmd),
        None => true,
    }
}

fn is_wrapper_command(args: &[String]) -> bool {
    args.first()
        .map(|cmd| WRAPPER_COMMANDS.contains(&cmd.as_str()))
        .unwrap_or(false)
}

fn is_selection_cancelled(err: &anyhow::Error) -> bool {
    err.to_string().contains("device selection cancelled")
}

pub fn has_device_selector(args: &[String]) -> bool {
    args.iter()
        .any(|arg| DEVICE_SELECTOR_FLAGS.iter().any(|flag| arg == *flag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_args_passthrough() {
        assert!(should_passthrough(&[]));
    }

    #[test]
    fn management_commands_passthrough() {
        assert!(should_passthrough(&["devices".into()]));
        assert!(should_passthrough(&["devices".into(), "-l".into()]));
        assert!(should_passthrough(&["version".into()]));
        assert!(should_passthrough(&["help".into()]));
        assert!(!should_passthrough(&["shell".into()]));
    }

    #[test]
    fn detects_device_selector_flags() {
        assert!(has_device_selector(&["-s".into(), "123".into(), "shell".into()]));
        assert!(has_device_selector(&["-d".into(), "shell".into()]));
        assert!(has_device_selector(&["-e".into(), "shell".into()]));
        assert!(!has_device_selector(&["shell".into()]));
    }

    #[test]
    fn wrapper_completions_command() {
        assert!(is_wrapper_command(&["completions".into(), "bash".into()]));
        assert!(!is_wrapper_command(&["shell".into()]));
    }
}
