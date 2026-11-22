"""Splits PDFs into passages."""
from pypdf import PdfReader


def passages(path: str, size: int = 800) -> list[str]:
    text = "\n".join(page.extract_text() or "" for page in PdfReader(path).pages)
    return [text[i : i + size] for i in range(0, len(text), size)]


def read_notes(path: str) -> str:
    with open(path) as f:
        return f.read()
