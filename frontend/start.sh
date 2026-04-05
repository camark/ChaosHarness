#!/bin/bash
# Start the Rust Harness frontend

# Set the backend command to run cargo
export OPENHARNESS_FRONTEND_CONFIG='{"backend_command":["cargo","run","--","--backend-only"]}'

# Start the frontend
npm start
