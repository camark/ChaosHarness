#!/bin/bash
# Test script to run TUI in pseudo-TTY
cd "$(dirname "$0")"
echo "=== TUI Test ==="
echo "Running in pseudo-TTY mode..."
echo "Type some characters, press backspace, and press enter to test input"
echo "Press Ctrl+C to exit"
echo ""

# Use unbuffer to create a pseudo-TTY if available
if command -v unbuffer &> /dev/null; then
    unbuffer npm start
else
    # Fall back to script command
    script -q -c "npm start" /dev/null
fi
