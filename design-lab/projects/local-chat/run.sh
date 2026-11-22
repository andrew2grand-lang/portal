#!/bin/bash
# Starts the web app on port 8000 (Ollama must already be running).
export OLLAMA_HOST=127.0.0.1:11434
exec uvicorn app.main:app --reload --port 8000
