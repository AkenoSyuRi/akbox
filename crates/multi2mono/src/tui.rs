use std::io::{self, Stdout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::{Frame, Terminal};

use crate::paths::parse_paths;
use crate::split::split_wav;

struct App {
    input: String,
    log: Vec<LogLine>,
    log_scroll: usize,
    processing: bool,
    should_quit: bool,
}

#[derive(Clone)]
struct LogLine {
    level: LogLevel,
    text: String,
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Success,
    Error,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            log: vec![LogLine {
                level: LogLevel::Info,
                text: "将 WAV 文件拖入输入框，或粘贴路径后按 Enter 处理。".into(),
            }],
            log_scroll: 0,
            processing: false,
            should_quit: false,
        }
    }

    fn push_log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.log.push(LogLine {
            level,
            text: text.into(),
        });
        self.log_scroll = self.log.len().saturating_sub(1);
    }

    fn process_input(&mut self) {
        if self.processing {
            return;
        }

        let paths = parse_paths(&self.input);
        if paths.is_empty() {
            self.push_log(LogLevel::Error, "请输入至少一个 WAV 文件路径。");
            return;
        }

        self.processing = true;
        self.input.clear();

        for path in paths {
            match split_wav(&path) {
                Ok(outputs) => {
                    self.push_log(
                        LogLevel::Success,
                        format!("{} → {} 个单声道文件", path.display(), outputs.len()),
                    );
                    for out in outputs {
                        self.push_log(LogLevel::Info, format!("  已写入: {}", out.display()));
                    }
                }
                Err(err) => {
                    self.push_log(
                        LogLevel::Error,
                        format!("处理 {} 失败: {err}", path.display()),
                    );
                }
            }
        }

        self.processing = false;
    }

    fn handle_paste(&mut self, text: &str) {
        if self.processing {
            return;
        }
        self.input.push_str(text.trim());
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if self.processing {
            return;
        }

        match code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Enter => self.process_input(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(ch) => self.input.push(ch),
            _ => {}
        }
    }
}

pub fn run() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    let mut app = App::new();

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode().map_err(|e| e.to_string())?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| e.to_string())?;
    terminal.show_cursor().map_err(|e| e.to_string())?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|e| e.to_string())?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key.code, key.modifiers);
                }
                Event::Paste(text) => app.handle_paste(&text),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new("multi2mono")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    let input_label = if app.processing {
        "处理中…"
    } else {
        "输入（拖放 / 粘贴路径，Enter 处理）"
    };
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .title(input_label)
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[1]);

    let log_lines: Vec<Line> = app
        .log
        .iter()
        .map(|line| {
            let style = match line.level {
                LogLevel::Info => Style::default(),
                LogLevel::Success => Style::default().fg(Color::Green),
                LogLevel::Error => Style::default().fg(Color::Red),
            };
            Line::from(Span::styled(line.text.as_str(), style))
        })
        .collect();

    let log_height = chunks[2].height.saturating_sub(2) as usize;
    let scroll = app.log_scroll.saturating_sub(log_height.saturating_sub(1));

    let log = Paragraph::new(log_lines)
        .block(Block::default().title("日志").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));
    frame.render_widget(log, chunks[2]);

    let mut scrollbar_state = ScrollbarState::new(app.log.len()).position(app.log_scroll);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chunks[2].inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    let footer = Paragraph::new("Enter 处理  Esc / Ctrl+C 退出")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}
