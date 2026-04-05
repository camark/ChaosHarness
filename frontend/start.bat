@echo off
REM Start the Rust Harness frontend with OHJSON stdio backend

cd /d "%~dp0"

echo Starting frontend...
echo.

REM Set environment variable
set "OPENHARNESS_FRONTEND_CONFIG={\"backend_command\":[\"cargo\",\"run\",\"--\",\"--stdio-backend\"]}"

REM Start in new window
start "RustHarness Frontend" cmd /k "cd /d %CD% && npx tsx src/index.tsx"

echo Frontend started in new window.
