@echo off
REM Test frontend-backend communication

echo Testing OHJSON stdio backend...
echo.

echo Sending test message to backend...
echo {"type":"submit_line","line":"test hello"} | cargo run --quiet -- --stdio-backend 2>&1 | findstr "^OHJSON:"

echo.
echo Test complete.
