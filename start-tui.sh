#!/bin/bash
# Start the Rust Harness native TUI frontend (ratatui)
# This provides better cross-platform support

cd "$(dirname "$0")"

echo "Starting Rust Harness TUI frontend..."
echo ""
echo "Controls:"
echo "  - Type your message"
echo "  - Backspace to delete"
echo "  - Enter to send"
echo "  - Esc to clear input"
echo "  - Ctrl+C to exit"
echo ""

exec cargo run -- --tui