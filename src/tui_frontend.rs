//! Ratatui TUI frontend for Rust Harness
//! Improved UI design with reliable backend communication

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
};
use std::{
    io::{self, Write, BufRead, BufReader},
    time::Duration,
    process::{Command, Stdio, Child, ChildStdin, ChildStdout},
    sync::mpsc::{self, TryRecvError},
    thread,
};

/// Backend communication
struct Backend {
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
    _thread: thread::JoinHandle<()>,
    _child: Option<Child>, // Keep child process alive
}

impl Backend {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();

        let mut child: Option<Child> = None;

        // Start backend process
        match Command::new("cargo")
            .args(["run", "--", "--stdio-backend"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => child = Some(c),
            Err(e) => eprintln!("Failed to start backend: {}", e),
        }

        let mut stdin: Option<ChildStdin> = None;
        let mut stdout: Option<ChildStdout> = None;

        if let Some(ref mut c) = child {
            stdin = c.stdin.take();
            stdout = c.stdout.take();
        }

        // Spawn reader thread - simple and reliable
        let handle = thread::spawn(move || {
            if let Some(mut stdout) = stdout {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let trimmed = line.trim();
                            if let Some(json_str) = trimmed.strip_prefix("OHJSON:") {
                                let _ = out_tx.send(json_str.to_string());
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            // Wait for messages and write to stdin
            for msg in rx {
                if let Some(ref mut stdin) = stdin {
                    let _ = writeln!(stdin, "{}", msg);
                    let _ = stdin.flush();
                }
            }
        });

        Self {
            tx,
            rx: out_rx,
            _thread: handle,
            _child: child,
        }
    }

    fn send(&self, msg: &str) {
        let _ = self.tx.send(msg.to_string());
    }

    fn recv(&self) -> Result<String, TryRecvError> {
        self.rx.try_recv()
    }
}

/// Terminal UI application
struct App {
    input: String,
    transcript: Vec<String>,
    busy: bool,
    should_quit: bool,
    cursor_visible: bool,
    cursor_timer: u32,
    backend: Backend,
    history: Vec<String>,
    history_index: isize,
}

impl App {
    fn new() -> Self {
        let backend = Backend::new();

        Self {
            input: String::new(),
            transcript: Vec::new(),
            busy: false,
            should_quit: false,
            cursor_visible: true,
            cursor_timer: 0,
            backend,
            history: Vec::new(),
            history_index: -1,
        }
    }

    fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        // Main loop
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Handle backend messages (non-blocking)
            loop {
                match self.backend.recv() {
                    Ok(msg) => self.process_backend_message(&msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            // Handle input with timeout
            if event::poll(Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, key.modifiers);
                    }
                }
            }

            if self.should_quit {
                break;
            }

            // Toggle cursor visibility every 5 frames (~500ms)
            self.cursor_timer = (self.cursor_timer + 1) % 5;
            if self.cursor_timer == 0 {
                self.cursor_visible = !self.cursor_visible;
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key {
                self.should_quit = true;
                return;
            }
        }

        match key {
            KeyCode::Enter => {
                if !self.input.trim().is_empty() && !self.busy {
                    self.submit_input();
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Esc => {
                self.input.clear();
            }
            KeyCode::Up => {
                if !self.history.is_empty() {
                    let next_index = (self.history_index + 1).min(self.history.len() as isize - 1);
                    if next_index >= 0 {
                        self.history_index = next_index;
                        self.input = self.history[self.history.len() - 1 - next_index as usize].clone();
                    }
                }
            }
            KeyCode::Down => {
                let next_index = self.history_index - 1;
                if next_index >= -1 {
                    self.history_index = next_index;
                    self.input = if next_index == -1 {
                        String::new()
                    } else {
                        self.history[self.history.len() - 1 - next_index as usize].clone()
                    };
                }
            }
            KeyCode::Char(c) => {
                if !self.busy {
                    self.input.push(c);
                }
            }
            _ => {}
        }
    }

    fn submit_input(&mut self) {
        let line = self.input.clone();
        self.input.clear();
        self.history.push(line.clone());
        self.history_index = -1;
        self.busy = true;

        // Send to backend
        let msg = format!(r#"{{"type":"submit_line","line":"{}"}}"#,
            line.replace('\\', "\\\\").replace('"', "\\\""));
        self.backend.send(&msg);
    }

    fn process_backend_message(&mut self, json_str: &str) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                match event_type {
                    "ready" => {
                        self.transcript.push("[System] Welcome to Rust Harness! Type your message below.".to_string());
                        self.busy = false;
                    }
                    "transcript_item" => {
                        if let Some(item) = event.get("item") {
                            let role = item.get("role")
                                .and_then(|v| v.as_str())
                                .unwrap_or("system");
                            let text = item.get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let line = match role {
                                "user" => format!("> {}", text),
                                "assistant" => format!("AI: {}", text),
                                "system" => format!("[System] {}", text),
                                "tool" => format!("[Tool] {}", text),
                                _ => text.to_string(),
                            };
                            self.transcript.push(line);
                        }
                    }
                    "line_complete" => {
                        self.busy = false;
                    }
                    "error" => {
                        let message = event.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown error");
                        self.transcript.push(format!("[Error] {}", message));
                        self.busy = false;
                    }
                    "clear_transcript" => {
                        self.transcript.clear();
                    }
                    "shutdown" => {
                        self.should_quit = true;
                    }
                    _ => {}
                }
            }
        }
    }

    fn ui(&self, f: &mut Frame) {
        let area = f.size();

        // Clear screen
        f.render_widget(Clear, area);

        // Create main layout with better spacing
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),    // Title bar
                Constraint::Min(1),       // Conversation area (flexible)
                Constraint::Length(1),    // Status separator
                Constraint::Length(3),    // Input area
            ])
            .split(area);

        // Title bar
        self.render_title(f, main_chunks[0]);

        // Conversation area
        self.render_conversation(f, main_chunks[1]);

        // Status separator and input
        self.render_status(f, main_chunks[2]);
        self.render_input(f, main_chunks[3]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = title_block.inner(area);
        f.render_widget(title_block, area);

        let title = Paragraph::new(" Rust Harness ")
            .style(Style::default().fg(Color::Cyan).bold())
            .alignment(Alignment::Center);

        f.render_widget(title, inner);
    }

    fn render_conversation(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Chat ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // Create transcript items
        let items: Vec<ListItem> = self.transcript
            .iter()
            .map(|line| {
                let style = if line.starts_with("> ") {
                    Style::default().fg(Color::Cyan)
                } else if line.starts_with("AI: ") {
                    Style::default().fg(Color::Green)
                } else if line.starts_with("[System]") {
                    Style::default().fg(Color::Yellow)
                } else if line.starts_with("[Tool]") {
                    Style::default().fg(Color::Magenta)
                } else if line.starts_with("[Error]") {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                };
                ListItem::new(line.clone()).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, inner_area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = if self.busy {
            " ⏳ Processing... "
        } else {
            " ✅ Ready "
        };

        let style = if self.busy {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let status = Paragraph::new(status_text).style(style);
        f.render_widget(status, area);
    }

    fn render_input(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // Cursor
        let cursor = if self.cursor_visible && !self.busy { "█" } else { " " };

        let input_text = if self.busy {
            format!("(busy) {}", cursor)
        } else {
            format!(">{}{}", self.input, cursor)
        };

        let style = if self.busy {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let input = Paragraph::new(input_text).style(style);
        f.render_widget(input, inner_area);
    }
}

/// Run the TUI frontend
pub fn run_tui_frontend() -> io::Result<()> {
    let mut app = App::new();
    app.run()
}
