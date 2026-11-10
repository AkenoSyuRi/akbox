//! Terminal/session scope used to remember the selected device.
//!
//! The goal is to remember one choice per terminal session (or per script run),
//! instead of either prompting on every invocation or storing a single global
//! serial that leaks across parallel scripts.

use std::path::PathBuf;

const SESSION_ENV: &str = "ADB_WRAPPER_SESSION";

/// Stable identifier for the current terminal/session.
///
/// Resolution order:
/// 1. `ADB_WRAPPER_SESSION` — explicit scope for scripts and CI.
/// 2. Platform terminal session id.
/// 3. Current working directory — fallback for hosts without a usable
///    terminal id (for example mintty on Windows).
pub fn current_scope() -> String {
    if let Ok(value) = std::env::var(SESSION_ENV) {
        let value = value.trim();
        if !value.is_empty() {
            return format!("env:{value}");
        }
    }

    if let Some(id) = platform_session_id() {
        return format!("tty:{id}");
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    format!("cwd:{}", cwd.display())
}

#[cfg(unix)]
fn platform_session_id() -> Option<String> {
    // The session id is inherited by subshells and command substitutions, so it
    // stays stable for the whole script running in a terminal.
    let sid = unsafe { libc::getsid(0) };
    (sid > 0).then(|| sid.to_string())
}

#[cfg(windows)]
fn platform_session_id() -> Option<String> {
    use windows_sys::Win32::System::Console::GetConsoleWindow;

    // Each console window gets a distinct HWND. Processes attached to the same
    // console (the shell, its subshells and scripts) share it.
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() {
        None
    } else {
        Some(format!("{:x}", hwnd as usize))
    }
}

#[cfg(not(any(unix, windows)))]
fn platform_session_id() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn explicit_session_env_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var(SESSION_ENV, "  my-script  ") };
        assert_eq!(current_scope(), "env:my-script");
        unsafe { std::env::remove_var(SESSION_ENV) };
    }

    #[test]
    fn empty_session_env_falls_back() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var(SESSION_ENV, "   ") };
        assert!(!current_scope().starts_with("env:"));
        unsafe { std::env::remove_var(SESSION_ENV) };
    }
}
