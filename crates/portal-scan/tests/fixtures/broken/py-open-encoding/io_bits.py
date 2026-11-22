from pathlib import Path

with open("notes.txt") as f:
    notes = f.read()
with open("out.csv", "w") as f:
    f.write(notes)
with open("image.png", "rb") as f:
    raw = f.read()
text = Path("readme.md").read_text()
Path("ok.txt").write_text("fine", encoding="utf-8")
Path("bad.txt").write_text("caf\u00e9")
with open("fine.txt", encoding="utf-8") as f:
    pass
