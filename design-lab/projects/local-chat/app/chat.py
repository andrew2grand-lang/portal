"""Answers a question from the closest passages in the index."""
import os

import chromadb
import ollama

MODEL = os.getenv("CHAT_MODEL", "llama3.2")


def answer(question: str, index_dir: str) -> str:
    db = chromadb.PersistentClient(path=index_dir)
    passages = db.get_or_create_collection("docs").query(query_texts=[question], n_results=4)
    context = "\n\n".join(passages["documents"][0])
    reply = ollama.chat(
        model=MODEL,
        messages=[
            {"role": "system", "content": "Answer from the passages only."},
            {"role": "user", "content": f"{context}\n\nQuestion: {question}"},
        ],
    )
    return reply["message"]["content"]
