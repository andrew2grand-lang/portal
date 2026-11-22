import argparse
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("folder", type=Path)
    args = parser.parse_args()
    for path in sorted(args.folder.glob("*.txt")):
        text = path.read_text(encoding="utf-8")
        (args.folder / "out").mkdir(exist_ok=True)
        (args.folder / "out" / path.name).write_text(text.strip(), encoding="utf-8")


if __name__ == "__main__":
    main()
