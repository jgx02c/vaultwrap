#!/bin/bash

# Start the Svelte/Vite dev server and Tauri dev process for development
# Usage: ./gui-start-dev.sh

# Start Vite dev server in the background
npm run dev &
VITE_PID=$!
echo "[gui-start-dev] Started Vite dev server (PID $VITE_PID)"

# Wait a bit to ensure Vite is up
sleep 2

# Start Tauri dev (will wait for Vite)
npm run tauri dev &
TAURI_PID=$!
echo "[gui-start-dev] Started Tauri dev process (PID $TAURI_PID)"

# Print instructions
cat <<EOF

[gui-start-dev] Both Vite and Tauri are running.
To stop both, press Ctrl+C in this terminal, or run:
  kill $VITE_PID $TAURI_PID

You can view logs above for both processes.
EOF

# Wait for both processes
wait $VITE_PID $TAURI_PID 