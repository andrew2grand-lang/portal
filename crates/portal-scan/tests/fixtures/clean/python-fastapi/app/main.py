import os
from pathlib import Path

from fastapi import FastAPI

app = FastAPI()
DATA = Path(__file__).parent / "data"


@app.get("/items")
def items():
    with open(DATA / "items.json", encoding="utf-8") as f:
        return f.read()


API_URL = os.environ.get("API_URL", "https://api.example.org")


def remote_items(host: str):
    import httpx

    return httpx.get(f"{API_URL}/items").json() + httpx.get(f"https://{host}/v1/items").json()
