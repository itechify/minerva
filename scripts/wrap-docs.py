"""Wraps the prose of long-lined Markdown docs (DESIGN.md, GLOSSARY.md) at 120 columns.

A paragraph kept on one line made each grep of DESIGN.md print thousands of characters. This
wraps paragraphs, list items and quotes; front matter, code, tables, headings and HTML stay as
they are, and the rendered Markdown doesn't change.

    python scripts/wrap-docs.py DESIGN.md GLOSSARY.md           wrap them in place
    python scripts/wrap-docs.py --check DESIGN.md GLOSSARY.md   name each line to wrap, and fail if any
"""
import re
import sys

WIDTH = 120
# Inline code spans and links stay whole, so a wrap never splits `a b` or [a b](c).
TOKEN = re.compile(r"(?:`[^`]*`|\[[^\]]*\]\([^)]*\)|[^\s`\[])+|\S+")
# A line may not begin with these after a wrap: Markdown would read it as a new block.
BLOCK_START = re.compile(r"^(?:[-+*>#|]|\d+[.)]$|=+$)")
ITEM = re.compile(r"^(\s*)([-+*]|\d+[.)])(\s+)")
QUOTE = re.compile(r"^\s*>\s?")


def wrap(line: str) -> list[str]:
    """`line` wrapped at WIDTH, its continuation lines indented to fit its list item or quote."""
    if len(line) <= WIDTH:
        return [line]
    hard_break = line.endswith("  ")
    body = line.rstrip()
    item, quote = ITEM.match(body), QUOTE.match(body)
    if item:
        first, indent = body[: item.end()], " " * item.end()
    elif quote:
        first = indent = quote.group(0)
    else:
        first = indent = body[: len(body) - len(body.lstrip())]
    rows: list[list[str]] = [[]]
    width = len(first)
    for word in TOKEN.findall(body[len(first):]):
        row = rows[-1]
        if row and width + 1 + len(word) > WIDTH:
            rows.append([])
            width = len(indent)
            # A row that would open with "-" or "1." takes the word before it along.
            while BLOCK_START.match(word) and len(row) > 1:
                moved = row.pop()
                rows[-1].insert(0, moved)
                width += len(moved) + 1
                word_check = moved
                if not BLOCK_START.match(word_check):
                    break
            row = rows[-1]
        width += len(word) + (1 if row else 0)
        row.append(word)
    lines = [first + " ".join(rows[0])] + [indent + " ".join(r) for r in rows[1:]]
    if hard_break:
        lines[-1] += "  "
    return lines


def wrapped(text: str) -> str:
    out, in_front, in_code = [], False, False
    for n, line in enumerate(text.split("\n")):
        stripped = line.lstrip()
        if n == 0 and line == "---":
            in_front = True
        elif in_front and line == "---":
            in_front = False
        elif stripped.startswith(("```", "~~~")):
            in_code = not in_code
            out.append(line)
            continue
        keep = in_front or in_code or line == "---" or stripped.startswith(("#", "|", "<"))
        out.extend([line] if keep else wrap(line))
    return "\n".join(out)


def main() -> int:
    args = sys.argv[1:]
    check = "--check" in args
    paths = [a for a in args if a != "--check"]
    failed = False
    for path in paths:
        text = open(path, encoding="utf-8").read()
        new = wrapped(text)
        if new == text:
            continue
        if check:
            kept = set(new.split("\n"))
            for n, line in enumerate(text.split("\n")):
                if line not in kept:
                    print(f"{path}:{n + 1}: longer than {WIDTH} columns")
            failed = True
        else:
            open(path, "w", encoding="utf-8", newline="\n").write(new)
            print(f"wrapped {path}")
    if failed:
        print(f"Wrap them with: python scripts/wrap-docs.py {' '.join(paths)}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
