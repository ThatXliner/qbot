#!/bin/sh

# Start Ollama server in the background
ollama serve &

# Save PID so we can wait at the end
SERVER_PID=$!

# Timeout settings
MAX_WAIT=60   # max seconds to wait for server
INTERVAL=1    # poll every second
ELAPSED=0

echo "Waiting for Ollama server to be ready..."
while ! curl -s http://localhost:11434 >/dev/null 2>&1; do
    sleep $INTERVAL
    ELAPSED=$((ELAPSED + INTERVAL))
    if [ "$ELAPSED" -ge "$MAX_WAIT" ]; then
        echo "Error: Ollama server did not start within $MAX_WAIT seconds."
        kill $SERVER_PID
        exit 1
    fi
done

echo "Server is up, pulling model..."
ollama pull qwen3:1.6b

# Wait for the server process to exit
wait $SERVER_PID
