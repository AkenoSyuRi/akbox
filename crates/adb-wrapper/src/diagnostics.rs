use crate::config::{config_path, load_config, Config};
use crate::path::{
    real_adb_display, real_adb_source_label, resolve_real_adb_details, RealAdbResolution,
    RealAdbSource,
};
use anyhow::Result;
use is_terminal::IsTerminal;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run_wrapper_info() -> Result<u8> {
    let config = load_config();
    let wrapper_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("<unknown>"));
    let resolution = resolve_real_adb_details(&config)?;

    let mut out = io::stdout().lock();
    writeln!(out, "adb-wrapper: active")?;
    writeln!(out, "wrapper version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(out, "wrapper path: {}", wrapper_exe.display())?;
    write_real_adb_section(&mut out, &resolution)?;
    writeln!(out, "stdin tty: {}", std::io::stdin().is_terminal())?;
    writeln!(out, "stdout tty: {}", std::io::stdout().is_terminal())?;
    writeln!(out, "stderr tty: {}", std::io::stderr().is_terminal())?;
    write_config_section(&mut out, &config)?;
    write_env_section(&mut out)?;
    write_recursion_section(&mut out, &resolution)?;

    Ok(0)
}

fn write_real_adb_section(
    out: &mut impl Write,
    resolution: &RealAdbResolution,
) -> Result<()> {
    match (&resolution.resolved, &resolution.resolved_source) {
        (Some(path), Some(source)) => {
            writeln!(
                out,
                "real adb: {} (via {})",
                real_adb_display(path),
                real_adb_source_label(source)
            )?;
        }
        _ => writeln!(out, "real adb: <not found>")?,
    }

    if let Some(value) = resolution.env_value.as_ref() {
        writeln!(out, "REAL_ADB env: {value}")?;
    } else {
        writeln!(out, "REAL_ADB env: <unset>")?;
    }

    for candidate in &resolution.candidates {
        let status = if candidate.exists { "ok" } else { "missing" };
        writeln!(
            out,
            "  candidate [{}]: {} ({status})",
            real_adb_source_label(&candidate.source),
            candidate.path.display()
        )?;
    }

    Ok(())
}

fn write_config_section(out: &mut impl Write, config: &Config) -> Result<()> {
    match config_path() {
        Some(path) => writeln!(out, "config path: {}", path.display())?,
        None => writeln!(out, "config path: <unavailable>")?,
    }
    writeln!(out, "prefer_last_device: {}", config.prefer_last_device)?;
    writeln!(out, "show_model: {}", config.show_model)?;
    writeln!(out, "selector: {}", config.selector)?;
    match config.last_serial.as_ref() {
        Some(serial) => writeln!(out, "last_serial: {serial}")?,
        None => writeln!(out, "last_serial: <unset>")?,
    }
    Ok(())
}

fn write_env_section(out: &mut impl Write) -> Result<()> {
    writeln!(
        out,
        "ADB_WRAPPER_PICK env: {}",
        std::env::var("ADB_WRAPPER_PICK").unwrap_or_else(|_| "<unset>".into())
    )?;
    Ok(())
}

fn write_recursion_section(out: &mut impl Write, resolution: &RealAdbResolution) -> Result<()> {
    if resolution.recursion_risk {
        writeln!(out, "recursion risk: yes (real adb resolves to this wrapper)")?;
        writeln!(out, "hint: point REAL_ADB / config / adb-real at the official SDK adb")?;
    } else if resolution.resolved.is_none() {
        writeln!(out, "recursion risk: unknown (real adb not resolved)")?;
    } else {
        writeln!(out, "recursion risk: no")?;
    }

    if resolution
        .candidates
        .iter()
        .any(|c| c.source == RealAdbSource::Env && !c.exists)
    {
        writeln!(out, "note: REAL_ADB is set but the file does not exist")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_info_contains_version() {
        let code = run_wrapper_info().expect("wrapper info");
        assert_eq!(code, 0);
    }
}
