"""Indexes every PDF in a folder: python3 scripts/build_index.py <folder>"""
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import chromadb

from app.ingest import passages

folder = Path(sys.argv[1]).expanduser()
pdfs = sorted(folder.glob("**/*.pdf"))
db = chromadb.PersistentClient(path="data/index")
docs = db.get_or_create_collection("docs")

with ProcessPoolExecutor() as pool:
    for pdf, parts in zip(pdfs, pool.map(passages, map(str, pdfs))):
        docs.add(documents=parts, ids=[f"{pdf.name}-{i}" for i in range(len(parts))])

print(f"Indexed {len(pdfs)} PDFs")
