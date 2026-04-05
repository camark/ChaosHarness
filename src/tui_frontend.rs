//! Ratatui TUI frontend for Rust Harness

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};
use std::{
    io::{self, Write},
    time::Duration,
    sync::mpsc::{self, TryRecvError},
    thread,
};

/// Terminal UI application
pub struct TuiApp {
    input: String,
    transcript: Vec<String>,
    busy: bool,
    should_quit: bool,
    frontend_rx: mpsc::Receiver<String>,
    cursor_visible: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        let (_, _) = mpsc::channel::<String>();
        let (frontend_tx, frontend_rx) = mpsc::channel();

        // Spawn backend reader thread
        thread::spawn(move || {
            use std::io::BufRead;
            let stdin = io::stdin();
            let mut reader = std::io::BufReader::new(stdin);
            let mut line = String::new();

            loop {
                line.clear();
                if reader.read_line(&mut line).is_ok() {
                    if line.starts_with("OHJSON:") {
                        let json_str = line.trim_start_matches("OHJSON:");
                        if frontend_tx.send(json_str.to_string()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            input: String::new(),
            transcript: Vec::new(),
            busy: false,
            should_quit: false,
            frontend_rx,
            cursor_visible: true,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
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

            // Handle backend messages
            loop {
                match self.frontend_rx.try_recv() {
                    Ok(msg) => self.process_backend_message(&msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            // Handle input
            if event::poll(Duration::from_millis(50))? {
                if let event::Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, key.modifiers);
                    }
                }
            }

            if self.should_quit {
                break;
            }

            // Toggle cursor visibility for blink effect
            self.cursor_visible = !self.cursor_visible;
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

        // Add user message to transcript
        self.transcript.push(format!("> {}", line));

        // Send to backend
        let msg = serde_json::json!({
            "type": "submit_line",
            "line": line
        });
        let _ = writeln!(io::stdout(), "{}", msg);
        let _ = io::stdout().flush();
    }

    fn process_backend_message(&mut self, json_str: &str) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                match event_type {
                    "ready" => {
                        self.transcript.push("[System] Backend ready".to_string());
                        self.busy = false;
                    }
                    "transcript_item" => {
                        if let Some(item) = event.get("item") {
                            let role = item.get("role").and_then(|v| v.as_str()).unwrap_or("system");
                            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            match role {
                                "user" => self.transcript.push(format!("> {}", text)),
                                "assistant" => self.transcript.push(format!("AI: {}", text)),
                                "system" => self.transcript.push(format!("[System] {}", text)),
                                "tool" => {
                                    let tool = item.get("tool_name").and_then(|v| v.as_str()).unwrap_or("tool");
                                    self.transcript.push(format!("[Tool: {}] {}", tool, text));
                                }
                                _ => self.transcript.push(text.to_string()),
                            }
                        }
                    }
                    "line_complete" => {
                        self.busy = false;
                    }
                    "error" => {
                        if let Some(msg) = event.get("message").and_then(|v| v.as_str()) {
                            self.transcript.push(format!("[Error] {}", msg));
                        }
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
                Constraint::Min(1),
                Constraint::Length(1),
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
                } else if line.starts_with("[Tool:") {
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
        let input_text = if self.busy {
            format!("> {}", self.input)
        } else {
            format!("> {}{}", self.input, cursor)
        };
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White));
        f.render_widget(input, input_chunks[1]);
    }
}

/// Run the TUI frontend
pub fn run_tui_frontend() -> io::Result<()> {
    let mut app = TuiApp::new();
    app.run()
}
