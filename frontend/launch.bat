@echo off
REM Launch RustHarness Frontend in a new window with proper TTY support

cd /d "%~dp0"

REM Check if node_modules exists
if not exist "node_modules" (
    echo Installing dependencies...
    call npm install
    if errorlevel 1 (
        echo Failed to install dependencies.
        pause
        exit /b 1
    )
)

REM Set environment variable
set OPENHARNESS_FRONTEND_CONFIG={"backend_command":["../target/debug/rust_harness.exe","--stdio-backend"]}

echo.
echo ========================================
echo   RustHarness React Frontend
echo ========================================
echo.
echo Starting in a new terminal window...
echo.
echo If this window closes immediately, manually:
echo   1. Open Windows Terminal
echo   2. Run: cd %CD% ^&^& npm start
echo.
pause

REM Try Windows Terminal first
where wt >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    wt -w 0 nt -p "Command Prompt" cmd /k "cd /d %CD% && set OPENHARNESS_FRONTEND_CONFIG={\"backend_command\":[\"../target/debug/rust_harness.exe\",\"--stdio-backend\"]} && npm start"
) else (
    REM Fallback to PowerShell Start-Process
    powershell -Command "Start-Process cmd -ArgumentList '/k', 'cd /d ''%CD%'' ^&^& set OPENHARNESS_FRONTEND_CONFIG={\"backend_command\":[\"../target/debug/rust_harness.exe\",\"--stdio-backend\"]} ^&^& npm start' -WindowStyle Normal"
)

echo Frontend launched in new window.
echo This window will close automatically.
timeout /t 3 /nobreak >nul
