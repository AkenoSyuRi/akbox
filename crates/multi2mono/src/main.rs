mod paths;
mod split;
mod tui;
mod wav_reader;

use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process;

use paths::parse_paths;
use split::split_wav;

const HELP: &str = "\
multi2mono - 将多声道 WAV 拆分为单声道文件

用法:
    multi2mono                   进入交互式 TUI（推荐，不污染 shell 历史）
    multi2mono <wav文件> [更多文件...]

    也可将 wav 文件拖放到本程序上（直接处理，不经 shell）。

输出:
    在与源文件同级的 out/ 目录下生成单声道文件。

选项:
    -h, --help    显示此帮助信息
";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return;
    }
    if args.is_empty() {
        if io::stdout().is_terminal() {
            if let Err(e) = tui::run() {
                eprintln!("{e}");
                pause_on_windows();
                process::exit(1);
            }
            return;
        }
        eprint!("{HELP}");
        pause_on_windows();
        process::exit(1);
    }

    let paths: Vec<PathBuf> = args.iter().flat_map(|a| parse_paths(a)).collect();
    let mut had_error = false;
    for path in paths {
        match split_wav(&path, 0.0) {
            Ok(outputs) => {
                for out in outputs {
                    println!("已写入: {}", out.display());
                }
            }
            Err(e) => {
                eprintln!("处理 {} 失败: {e}", path.display());
                had_error = true;
            }
        }
    }

    if had_error {
        pause_on_windows();
        process::exit(1);
    }
}

#[cfg(windows)]
fn pause_on_windows() {
    use std::io::Write;
    eprint!("按 Enter 键退出...");
    let _ = io::stderr().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

#[cfg(not(windows))]
fn pause_on_windows() {}
