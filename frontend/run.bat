@echo off
REM RustHarness React Frontend - Run in new window for TTY support

cd /d "%~dp0"

echo Starting RustHarness React Frontend...
echo.
echo IMPORTANT: This must run in a separate window for TTY support.
echo If this window closes immediately, run this file directly in Windows Terminal.
echo.

REM Check if running in a new window (has window title)
if not defined WINDOW_TITLE (
    echo Launching in new window...
    set WINDOW_TITLE=1
    start "RustHarness Frontend" cmd /c "%~f0"
    exit /b 0
)

REM Check for node_modules
if not exist "node_modules" (
    echo Installing dependencies...
    call npm install
    if errorlevel 1 (
        echo Failed to install dependencies.
        pause
        exit /b 1
    )
)

REM Set backend command
set OPENHARNESS_FRONTEND_CONFIG={"backend_command":["../target/debug/rust_harness.exe","--stdio-backend"]}

REM Start frontend
echo Starting frontend...
npx tsx src/index.tsx
