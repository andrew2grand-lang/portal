"""The web app: one page, one chat endpoint."""
import os
from pathlib import Path

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi.responses import HTMLResponse
from pydantic import BaseModel

from .chat import answer

load_dotenv()

DATA_DIR = os.getenv("LOCAL_CHAT_DATA", str(Path(__file__).parent.parent / "data"))
INDEX_DIR = DATA_DIR + "/index"
PAGE = Path(__file__).parent / "page.html"

app = FastAPI(title="local-chat")


class Question(BaseModel):
    text: str


@app.get("/", response_class=HTMLResponse)
def home():
    with open(PAGE) as f:
        return f.read()


@app.post("/ask")
def ask(q: Question):
    return {"answer": answer(q.text, INDEX_DIR)}


def run():
    import uvicorn

    uvicorn.run(app, port=8000)
