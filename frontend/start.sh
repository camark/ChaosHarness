#!/bin/bash
# Start the Rust Harness frontend

# Set the backend command to run the Rust binary with stdio backend
export OPENHARNESS_FRONTEND_CONFIG='{"backend_command":["../target/debug/rust_harness","--stdio-backend"]}'

# Start the frontend
npm start
