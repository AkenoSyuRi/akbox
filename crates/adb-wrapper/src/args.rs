const FORCE_PICK_ENV: &str = "ADB_WRAPPER_PICK";
const WRAPPER_FLAG_PICK: &str = "--pick";
const WRAPPER_FLAG_INFO: &str = "--wrapper-info";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperArgs {
    pub force_pick: bool,
    pub wrapper_info: bool,
    pub adb_args: Vec<String>,
}

pub fn preprocess_args(args: Vec<String>) -> WrapperArgs {
    let mut force_pick = should_force_pick_from_env();
    let mut wrapper_info = false;
    let mut adb_args = args;

    if let Some(first) = adb_args.first() {
        if first == WRAPPER_FLAG_PICK {
            force_pick = true;
            adb_args.remove(0);
        } else if first == WRAPPER_FLAG_INFO {
            wrapper_info = true;
            adb_args.remove(0);
        }
    }

    WrapperArgs {
        force_pick,
        wrapper_info,
        adb_args,
    }
}

pub fn should_force_pick_from_env() -> bool {
    std::env::var(FORCE_PICK_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn strips_pick_flag() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(FORCE_PICK_ENV); }

        let parsed = preprocess_args(vec!["--pick".into(), "shell".into()]);
        assert!(parsed.force_pick);
        assert_eq!(parsed.adb_args, vec!["shell".to_string()]);
    }

    #[test]
    fn strips_wrapper_info_flag() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(FORCE_PICK_ENV); }

        let parsed = preprocess_args(vec!["--wrapper-info".into(), "shell".into()]);
        assert!(parsed.wrapper_info);
        assert_eq!(parsed.adb_args, vec!["shell".to_string()]);
    }

    #[test]
    fn env_force_pick() {
        let _guard = ENV_LOCK.lock().unwrap();

        unsafe { std::env::set_var(FORCE_PICK_ENV, "1"); }
        let parsed = preprocess_args(vec!["shell".into()]);
        assert!(parsed.force_pick);
        unsafe { std::env::remove_var(FORCE_PICK_ENV); }
    }

    #[test]
    fn keeps_wrapper_like_flags_after_adb_command() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(FORCE_PICK_ENV); }

        let parsed = preprocess_args(vec!["shell".into(), "echo".into(), "--pick".into()]);
        assert!(!parsed.force_pick);
        assert_eq!(
            parsed.adb_args,
            vec![
                "shell".to_string(),
                "echo".to_string(),
                "--pick".to_string()
            ]
        );

        let parsed = preprocess_args(vec!["shell".into(), "--wrapper-info".into()]);
        assert!(!parsed.wrapper_info);
        assert_eq!(
            parsed.adb_args,
            vec!["shell".to_string(), "--wrapper-info".to_string()]
        );
    }
}
