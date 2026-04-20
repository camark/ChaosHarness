#!/bin/bash
# RustHarness Launcher - Choose between REPL and Frontend modes

echo ""
echo "===================================="
echo "  RustHarness Launcher"
echo "===================================="
echo ""
echo "  1. Start REPL (Interactive CLI)"
echo "  2. Start Frontend (React TUI)"
echo "  3. Start Backend Only (WebSocket)"
echo ""
read -p "Enter your choice (1-3): " choice

case $choice in
    1)
        echo ""
        echo "Starting REPL mode..."
        cargo run -- "$@"
        ;;
    2)
        echo ""
        echo "Starting Frontend mode..."
        echo "Backend will start automatically."
        cd frontend
        if [ -d "node_modules" ]; then
            npm start
        else
            echo "Installing dependencies..."
            npm install
            npm start
        fi
        ;;
    3)
        echo ""
        echo "Starting Backend Only mode (WebSocket on port 3000)..."
        cargo run -- --backend-only
        ;;
    *)
        echo "Invalid choice. Exiting."
        exit 1
        ;;
esac
