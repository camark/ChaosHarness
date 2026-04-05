@echo off
REM RustHarness Launcher - Choose between REPL and Frontend modes

echo.
echo ====================================
echo   RustHarness Launcher
echo ====================================
echo.
echo   1. Start REPL (Interactive CLI)
echo   2. Start Frontend (React TUI)
echo   3. Start Backend Only (WebSocket)
echo.
set /p choice="Enter your choice (1-3): "

if "%choice%"=="1" goto REPL
if "%choice%"=="2" goto FRONTEND
if "%choice%"=="3" goto BACKEND
echo Invalid choice. Exiting.
exit /b 1

:REPL
echo.
echo Starting REPL mode...
cargo run -- %*
exit /b

:FRONTEND
echo.
echo Starting Frontend mode...
echo Note: Requires TTY support (Windows Terminal or CMD)
cd frontend
if exist node_modules (
    call start.bat
) else (
    echo Installing dependencies...
    npm install
    call start.bat
)
exit /b

:BACKEND
echo.
echo Starting Backend Only mode (WebSocket on port 3000)...
cargo run -- --backend-only
exit /b
