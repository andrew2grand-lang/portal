# local-chat

Ask questions about your PDFs. Everything runs on this computer: the model
runs in Ollama, the index is stored in `data/index`.

## Run

```sh
python3 -m venv .venv && source .venv/bin/activate
pip install -e .
./run.sh
```

Then open http://localhost:8000.

## Add documents

```sh
python3 scripts/build_index.py ~/Documents/papers
```
