@echo off
REM Start the Rust Harness native TUI frontend (ratatui)
REM This provides better Windows compatibility than the React/Ink frontend

cd /d "%~dp0"

echo Starting Rust Harness TUI frontend...
echo.
echo Controls:
echo   - Type your message
echo   - Backspace to delete
echo   - Enter to send
echo   - Esc to clear input
echo   - Ctrl+C to exit
echo.

cargo run -- --tui
