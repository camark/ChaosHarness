//! Ratatui TUI frontend for Rust Harness
//! Simple synchronous version for better reliability on Windows

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
    _child: Child, // Keep child process alive
}

impl Backend {
    fn new() -> io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();

        // Start backend process - use debug build directly
        let bin_path = if cfg!(windows) {
            "target/debug/rust_harness.exe"
        } else {
            "target/debug/rust_harness"
        };

        let mut child = Command::new(bin_path)
            .arg("--stdio-backend")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Spawn reader thread
        let handle = thread::spawn(move || {
            // Thread 1: Read stdout
            let stdout_thread = thread::spawn(move || {
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
            });

            // Thread 2: Write stdin
            let stdin_thread = thread::spawn(move || {
                for msg in rx {
                    let _ = writeln!(stdin, "{}", msg);
                    let _ = stdin.flush();
                }
            });

            let _ = stdout_thread.join();
            let _ = stdin_thread.join();
        });

        Ok(Self {
            tx,
            rx: out_rx,
            _thread: handle,
            _child: child,
        })
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
}

impl App {
    fn new() -> io::Result<Self> {
        let backend = Backend::new()?;

        Ok(Self {
            input: String::new(),
            transcript: Vec::new(),
            busy: false,
            should_quit: false,
            cursor_visible: true,
            cursor_timer: 0,
            backend,
        })
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
        self.busy = true;

        // Don't add user message here - backend will echo it via transcript_item
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
                        self.transcript.push("[System] Backend connected".to_string());
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
                    _ => {}
                }
            }
        }
    }

    fn ui(&self, f: &mut Frame) {
        let area = f.size();

        // Clear screen
        f.render_widget(Clear, area);

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Rust Harness TUI - Ctrl+C to exit")
            .style(Style::default().fg(Color::Cyan).bold())
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        // Transcript area
        let transcript_lines: Vec<ListItem> = self.transcript
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

        let list = List::new(transcript_lines)
            .block(Block::default()
                .title(" Transcript ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)));
        f.render_widget(list, chunks[1]);

        // Input area
        let input_area = chunks[2];
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(2)])
            .split(input_area);

        // Status line
        let status = if self.busy {
            Paragraph::new("Processing...")
                .style(Style::default().fg(Color::Yellow))
        } else {
            Paragraph::new("Ready - Enter to send, Esc to clear")
                .style(Style::default().fg(Color::DarkGray))
        };
        f.render_widget(status, input_chunks[0]);

        // Input line with cursor
        let cursor = if self.cursor_visible && !self.busy { "█" } else { " " };
        let input_text = format!("> {}{}", self.input, cursor);
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White));
        f.render_widget(input, input_chunks[1]);
    }
}

/// Run the TUI frontend
pub fn run_tui_frontend() -> io::Result<()> {
    match App::new() {
        Ok(mut app) => app.run(),
        Err(e) => {
            eprintln!("Failed to start TUI: {}", e);
            Err(e)
        }
    }
}
