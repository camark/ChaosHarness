#!/bin/bash
# Run TUI in a real pseudo-TTY
cd "$(dirname "$0")"

echo "=== Running TUI in Pseudo-TTY ==="
echo "Terminal device: $(tty 2>/dev/null || echo 'not a tty')"
echo ""

# Use setsid to create a new session and script to provide TTY
setsid script -q -c "npm start" /dev/null
