@echo off
REM Start the Rust Harness frontend with OHJSON stdio backend
REM Note: Requires TTY support. Will launch in Windows Terminal automatically.

REM Set the backend command (stdio mode with OHJSON protocol)
set OPENHARNESS_FRONTEND_CONFIG={"backend_command":["cargo","run","--","--stdio-backend"]}

REM Check if running in Windows Terminal
if defined WT_SESSION (
    echo Starting frontend in current Windows Terminal session...
    npx tsx src/index.tsx
) else (
    echo Starting frontend in new Windows Terminal window...
    wt -w 0 nt -p "Command Prompt" cmd /c "cd /d %CD% ^&^& set OPENHARNESS_FRONTEND_CONFIG={\"backend_command\":[\"cargo\",\"run\",\"--\",\"--stdio-backend\"]} ^&^& npx tsx src/index.tsx"
)
