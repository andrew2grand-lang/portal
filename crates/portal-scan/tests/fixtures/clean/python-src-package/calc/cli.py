import os
import tempfile
from pathlib import Path


def run() -> None:
    out = Path(tempfile.gettempdir()) / "calc.txt"
    out.write_text(str(os.cpu_count()), encoding="utf-8")
    print(Path.home())
