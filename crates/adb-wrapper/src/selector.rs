use crate::config::SelectorKind;
use crate::device::{AdbDevice, DeviceOption};
use anyhow::{Context, Result, bail};
use inquire::error::InquireError;
use inquire::Select;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn prompt_select_device(
    devices: &[AdbDevice],
    show_model: bool,
    selector: SelectorKind,
) -> Result<String> {
    if devices.is_empty() {
        bail!("adb-wrapper: no online devices to select");
    }

    if devices.len() == 1 {
        return Ok(devices[0].serial.clone());
    }

    match selector {
        SelectorKind::Inquire => select_inquire(devices, show_model),
        SelectorKind::Skim => select_skim(devices, show_model)
            .or_else(|err| {
                eprintln!("adb-wrapper: skim failed ({err:#}), falling back to inquire");
                select_inquire(devices, show_model)
            }),
        SelectorKind::Fzf => select_fzf(devices, show_model)
            .or_else(|err| {
                eprintln!("adb-wrapper: fzf failed ({err:#}), falling back to skim");
                select_skim(devices, show_model)
            })
            .or_else(|err| {
                eprintln!("adb-wrapper: skim failed ({err:#}), falling back to inquire");
                select_inquire(devices, show_model)
            }),
    }
}

fn select_inquire(devices: &[AdbDevice], show_model: bool) -> Result<String> {
    let options: Vec<DeviceOption<'_>> = devices
        .iter()
        .map(|device| DeviceOption { device, show_model })
        .collect();
    let selected = Select::new("Select adb device:", options)
        .with_help_message("↑/↓ to move, Enter to select")
        .prompt()
        .map_err(map_inquire_error)?;

    Ok(selected.device.serial.clone())
}

fn select_skim(devices: &[AdbDevice], show_model: bool) -> Result<String> {
    use skim::prelude::*;

    let labels: Vec<String> = devices
        .iter()
        .map(|device| device.selector_label(show_model))
        .collect();
    let mut options = SkimOptions::default();
    options.prompt = "Select adb device> ".into();
    let output = Skim::run_items(options, labels)
        .map_err(|err| anyhow::anyhow!("adb-wrapper: skim selection failed: {err}"))?;

    let selected = output
        .selected_items
        .first()
        .context("adb-wrapper: device selection cancelled")?;

    let selected_label = selected.item.as_ref().text();
    let serial = selected_label
        .split_whitespace()
        .next_back()
        .with_context(|| format!("adb-wrapper: skim returned unknown selection: {selected_label}"))?;

    if devices.iter().any(|device| device.serial == serial) {
        Ok(serial.to_string())
    } else {
        bail!("adb-wrapper: skim returned unknown serial: {serial}");
    }
}

fn select_fzf(devices: &[AdbDevice], show_model: bool) -> Result<String> {
    let labels: Vec<String> = devices
        .iter()
        .map(|device| device.selector_label(show_model))
        .collect();

    let mut child = Command::new("fzf")
        .arg("--prompt=Select adb device> ")
        .arg("--height=40%")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("adb-wrapper: failed to launch fzf (is it installed and on PATH?)")?;

    {
        let stdin = child.stdin.as_mut().context("fzf stdin unavailable")?;
        for label in &labels {
            writeln!(stdin, "{label}")?;
        }
    }

    let output = child
        .wait_with_output()
        .context("adb-wrapper: failed to wait for fzf")?;

    if !output.status.success() {
        bail!("adb-wrapper: device selection cancelled");
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        bail!("adb-wrapper: device selection cancelled");
    }

    labels
        .iter()
        .position(|label| label == &selected)
        .map(|index| devices[index].serial.clone())
        .with_context(|| format!("adb-wrapper: fzf returned unknown selection: {selected}"))
}

fn map_inquire_error(err: InquireError) -> anyhow::Error {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            anyhow::anyhow!("adb-wrapper: device selection cancelled")
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_device_skips_ui() {
        let devices = vec![AdbDevice {
            serial: "abc".into(),
            state: "device".into(),
            model: None,
            product: None,
            device: None,
            transport_id: None,
        }];
        let serial =
            prompt_select_device(&devices, true, SelectorKind::Skim).expect("single device");
        assert_eq!(serial, "abc");
    }
}
