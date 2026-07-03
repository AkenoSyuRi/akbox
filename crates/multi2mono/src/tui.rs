use std::io::{self, Stdout};

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::{Frame, Terminal};

use crate::paths::parse_paths;
use crate::split::split_wav;

struct App {
    input: String,
    log: Vec<LogLine>,
    log_scroll: usize,
    gain_db: f64,
    gain_input: String,
    gain_replace: bool,
    focus: Focus,
    title_rect: Rect,
    input_rect: Rect,
    log_rect: Rect,
    processing: bool,
    should_quit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Main,
    PathInput,
    Log,
    GainDialog,
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
            log: Vec::new(),
            log_scroll: 0,
            gain_db: 0.0,
            gain_input: String::new(),
            gain_replace: false,
            focus: Focus::Main,
            title_rect: Rect::default(),
            input_rect: Rect::default(),
            log_rect: Rect::default(),
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

    fn open_gain_dialog(&mut self) {
        self.gain_input = self.gain_db.to_string();
        self.gain_replace = true;
        self.focus = Focus::GainDialog;
    }

    fn gain_step(&mut self, delta: f64) {
        let current = self.gain_input.trim().parse::<f64>().unwrap_or(0.0);
        self.gain_input = (current + delta).to_string();
        self.gain_replace = false;
    }

    fn gain_type_char(&mut self, ch: char) {
        if self.gain_replace {
            self.gain_input.clear();
            self.gain_input.push(ch);
            self.gain_replace = false;
        } else {
            self.gain_input.push(ch);
        }
    }

    fn close_gain_dialog(&mut self) {
        self.focus = Focus::Main;
    }

    fn focus_log(&mut self) {
        if self.focus == Focus::GainDialog {
            self.close_gain_dialog();
        }
        self.focus = Focus::Log;
    }

    fn blur_to_main(&mut self) {
        self.focus = Focus::Main;
    }

    fn focus_path_input(&mut self) {
        self.focus = Focus::PathInput;
    }

    fn confirm_gain(&mut self) {
        let trimmed = self.gain_input.trim();
        if trimmed.is_empty() {
            self.gain_db = 0.0;
            self.close_gain_dialog();
            return;
        }
        match trimmed.parse::<f64>() {
            Ok(db) => {
                self.gain_db = db;
                self.close_gain_dialog();
            }
            Err(_) => {
                self.push_log(LogLevel::Error, format!("无效的增益值: {trimmed}"));
            }
        }
    }

    fn reset_panels(&mut self) {
        self.input.clear();
        self.log.clear();
        self.log_scroll = 0;
    }

    fn process_input(&mut self) {
        if self.processing {
            return;
        }

        let paths = parse_paths(&self.input);
        self.log.clear();
        self.log_scroll = 0;

        if paths.is_empty() {
            self.push_log(LogLevel::Error, "请输入至少一个 WAV 文件路径。");
            return;
        }

        self.processing = true;
        let gain_db = self.gain_db;

        for path in paths {
            match split_wav(&path, gain_db) {
                Ok(outputs) => {
                    let gain_note = if gain_db == 0.0 {
                        String::new()
                    } else {
                        format!(" (增益 {gain_db} dB)")
                    };
                    self.push_log(
                        LogLevel::Success,
                        format!(
                            "{} → {} 个单声道文件{gain_note}",
                            path.display(),
                            outputs.len()
                        ),
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
        self.focus = Focus::Main;
    }

    fn handle_paste(&mut self, text: &str) {
        if self.processing {
            return;
        }
        if self.focus == Focus::GainDialog {
            self.close_gain_dialog();
        }
        self.set_path_input(text.trim());
    }

    /// 拖放路径常以逐字符输入而非 Paste 事件，需识别并整段替换。
    fn accept_path_char(&mut self, ch: char, from_main: bool) {
        let replace = from_main || ch == '"';
        if replace {
            self.input.clear();
        }
        self.focus_path_input();
        self.input.push(ch);
    }

    fn scroll_log(&mut self, delta: i32) {
        if self.log.is_empty() {
            return;
        }
        let max = self.log.len().saturating_sub(1);
        if delta < 0 {
            self.log_scroll = self.log_scroll.saturating_sub((-delta) as usize);
        } else {
            self.log_scroll = (self.log_scroll + delta as usize).min(max);
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.processing {
            return;
        }

        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if rect_contains(self.title_rect, col, row) {
                    self.open_gain_dialog();
                } else if rect_contains(self.input_rect, col, row) {
                    if self.focus == Focus::GainDialog {
                        self.close_gain_dialog();
                    }
                    self.focus_path_input();
                } else if rect_contains(self.log_rect, col, row) {
                    self.focus_log();
                } else if self.focus == Focus::GainDialog {
                    self.close_gain_dialog();
                } else {
                    self.blur_to_main();
                }
            }
            MouseEventKind::ScrollUp if rect_contains(self.log_rect, col, row) => {
                self.scroll_log(-1);
            }
            MouseEventKind::ScrollDown if rect_contains(self.log_rect, col, row) => {
                self.scroll_log(1);
            }
            _ => {}
        }
    }

    fn set_path_input(&mut self, text: &str) {
        self.input = text.to_string();
        self.focus = Focus::Main;
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if self.processing {
            return;
        }

        if modifiers.contains(KeyModifiers::CONTROL) {
            if code == KeyCode::Char('c') {
                self.should_quit = true;
            }
            return;
        }

        match self.focus {
            Focus::GainDialog => self.handle_gain_key(code),
            Focus::PathInput => self.handle_path_key(code),
            Focus::Log => self.handle_log_key(code),
            Focus::Main => self.handle_main_key(code),
        }
    }

    fn handle_log_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.blur_to_main(),
            KeyCode::Up => self.scroll_log(-1),
            KeyCode::Down => self.scroll_log(1),
            KeyCode::Char('g') => self.open_gain_dialog(),
            _ => {}
        }
    }

    fn handle_gain_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.close_gain_dialog(),
            KeyCode::Enter => self.confirm_gain(),
            KeyCode::Up => self.gain_step(1.0),
            KeyCode::Down => self.gain_step(-1.0),
            KeyCode::Backspace => {
                if self.gain_replace {
                    self.gain_input.clear();
                    self.gain_replace = false;
                } else {
                    self.gain_input.pop();
                }
            }
            KeyCode::Char(ch) if ch == '-' || ch == '.' || ch.is_ascii_digit() => {
                self.gain_type_char(ch);
            }
            _ => {}
        }
    }

    fn handle_main_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {}
            KeyCode::Char('g') => self.open_gain_dialog(),
            KeyCode::Char('r') => self.reset_panels(),
            KeyCode::Char('i') => self.focus_path_input(),
            KeyCode::Enter => self.process_input(),
            KeyCode::Char(ch) => self.accept_path_char(ch, true),
            _ => {}
        }
    }

    fn handle_path_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.blur_to_main(),
            KeyCode::Enter => self.process_input(),
            KeyCode::Backspace => {
                self.input.pop();
                if self.input.is_empty() {
                    self.blur_to_main();
                }
            }
            KeyCode::Char(ch) => self.accept_path_char(ch, false),
            _ => {}
        }
    }
}

pub fn run() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableMouseCapture
    )
    .map_err(|e| e.to_string())?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    let mut app = App::new();

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode().map_err(|e| e.to_string())?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste,
        DisableMouseCapture
    )
    .map_err(|e| e.to_string())?;
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
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    app.title_rect = chunks[0];
    app.input_rect = chunks[1];
    app.log_rect = chunks[2];

    let title_border = if app.focus == Focus::GainDialog {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = Paragraph::new(format!("multi2mono    增益: {} dB（点击设置）", app.gain_db))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(title_border),
        );
    frame.render_widget(title, chunks[0]);

    let input_label = if app.processing {
        "处理中…"
    } else if app.focus == Focus::PathInput {
        "输入（编辑中，Esc 取消焦点）"
    } else if app.input.is_empty() {
        "输入（拖放 / 粘贴路径，Enter 处理）"
    } else {
        "输入（已就绪，Enter 处理）"
    };
    let input_style = if app.focus == Focus::PathInput {
        Style::default().fg(Color::Cyan)
    } else if app.input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Gray)
    };
    let input_border = if app.focus == Focus::PathInput {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input = Paragraph::new(app.input.as_str())
        .style(input_style)
        .block(
            Block::default()
                .title(input_label)
                .borders(Borders::ALL)
                .border_style(input_border),
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

    let log_border = if app.focus == Focus::Log {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let log = Paragraph::new(log_lines)
        .block(
            Block::default()
                .title("日志")
                .borders(Borders::ALL)
                .border_style(log_border),
        )
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

    let footer = Paragraph::new(footer_text(app.focus))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: true });
    frame.render_widget(footer, chunks[3]);

    if app.focus == Focus::GainDialog {
        draw_gain_dialog(frame, app);
    }
}

fn footer_text(focus: Focus) -> &'static str {
    match focus {
        Focus::Main => "Enter 处理  g/点击标题 增益  r 清空  i 编辑  Ctrl+C 退出",
        Focus::PathInput => "Enter 处理  Esc 取消焦点  Ctrl+C 退出",
        Focus::Log => "↑↓ 滚动  g/点击标题 增益  Esc 取消焦点  Ctrl+C 退出",
        Focus::GainDialog => "↑ +1 dB  ↓ -1 dB  Enter 确认  Esc 取消焦点",
    }
}

fn draw_gain_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 5, frame.size());
    frame.render_widget(Clear, area);

    let text_style = if app.gain_replace {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let dialog = Paragraph::new(app.gain_input.as_str())
        .style(text_style)
        .block(
            Block::default()
                .title("增益调整 (dB)")
                .borders(Borders::ALL),
        );
    frame.render_widget(dialog, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}
