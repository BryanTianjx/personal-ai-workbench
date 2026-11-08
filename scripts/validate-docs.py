#!/usr/bin/env python3
"""Deterministic documentation checks for Personal AI Workbench.

Uses only the Python standard library. Run from any directory:
    python3 scripts/validate-docs.py
Optional formatting repair:
    python3 scripts/validate-docs.py --fix-format
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
LIST = re.compile(r"^(?:[-+*]|[0-9]+[.)])\s+")
LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})")
ADR_LINK = re.compile(r"\[(ADR-[0-9]{3})\]\(([^)]+)\)")
EXPECTED_ADR_NUMBERS = (
    {f"{i:03d}" for i in range(1, 7)}
    | {f"{i:03d}" for i in range(13, 17)}
    | {f"{i:03d}" for i in range(17, 23)}
)


def fix_format(text: str) -> str:
    lines = text.replace("\r\n", "\n").replace("\r", "\n").splitlines()
    out: list[str] = []
    in_fence = False
    fence_char = ""
    for line in lines:
        marker = FENCE.match(line)
        if marker:
            token = marker.group(1)
            if not in_fence:
                in_fence, fence_char = True, token[0]
            elif token[0] == fence_char:
                in_fence = False
        if not in_fence and LIST.match(line) and out and out[-1].strip():
            if not LIST.match(out[-1]) and not out[-1].startswith((" ", "\t")):
                out.append("")
        out.append(line.rstrip())
    fixed: list[str] = []
    in_fence = False
    fence_char = ""
    for index, line in enumerate(out):
        marker = FENCE.match(line)
        if marker:
            token = marker.group(1)
            if not in_fence:
                in_fence, fence_char = True, token[0]
            elif token[0] == fence_char:
                in_fence = False
        fixed.append(line)
        if in_fence or not LIST.match(line) or index + 1 >= len(out):
            continue
        following = out[index + 1]
        if following.strip() and not LIST.match(following) and not following.startswith((" ", "\t")):
            fixed.append("")
    while fixed and not fixed[-1].strip():
        fixed.pop()
    return "\n".join(fixed) + "\n"


def check_markdown(path: Path, text: str, errors: list[str]) -> None:
    rel = path.relative_to(ROOT).as_posix()
    lines = text.splitlines()
    if not text.endswith("\n"):
        errors.append(f"{rel}: missing final newline (MD047)")
    if text.endswith("\n\n"):
        errors.append(f"{rel}: more than one final newline")
    in_fence = False
    fence_char = ""
    headings: list[tuple[int, int]] = []
    table_width: int | None = None
    for number, line in enumerate(lines, 1):
        if line.rstrip() != line:
            errors.append(f"{rel}:{number}: trailing whitespace (MD009)")
        marker = FENCE.match(line)
        if marker:
            token = marker.group(1)
            if not in_fence:
                in_fence, fence_char = True, token[0]
            elif token[0] == fence_char:
                in_fence = False
            continue
        if in_fence:
            continue
        heading = re.match(r"^(#{1,6})\s+", line)
        if heading:
            headings.append((number, len(heading.group(1))))
        if line.startswith("|"):
            cells = len(line.strip().strip("|").split("|"))
            if table_width is None:
                table_width = cells
            elif cells != table_width:
                errors.append(f"{rel}:{number}: inconsistent table column count")
        elif line.strip() and table_width is not None:
            table_width = None
        if LIST.match(line):
            if number > 1 and lines[number - 2].strip() and not LIST.match(lines[number - 2]) and not lines[number - 2].startswith((" ", "\t")):
                errors.append(f"{rel}:{number}: missing blank line before list (MD032)")
            if number < len(lines):
                nxt = lines[number]
                if nxt.strip() and not LIST.match(nxt) and not nxt.startswith((" ", "\t")):
                    errors.append(f"{rel}:{number}: missing blank line after list (MD032)")
    if in_fence:
        errors.append(f"{rel}: unclosed fenced code block")
    previous = 0
    for number, level in headings:
        if previous and level > previous + 1:
            errors.append(f"{rel}:{number}: heading level jumps from H{previous} to H{level}")
        previous = level


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fix-format", action="store_true", help="repair list spacing and terminal newlines")
    args = parser.parse_args()
    if not DOCS.is_dir():
        print("ERROR: docs/ directory not found", file=sys.stderr)
        return 2

    markdown = sorted(p for p in ROOT.rglob("*.md") if not any(part.startswith(".") for part in p.parts))
    if args.fix_format:
        for path in markdown:
            original = path.read_text(encoding="utf-8")
            repaired = fix_format(original)
            if repaired != original:
                path.write_text(repaired, encoding="utf-8", newline="\n")

    errors: list[str] = []
    for path in markdown:
        check_markdown(path, path.read_text(encoding="utf-8"), errors)

    baseline = DOCS / "01-product" / "requirements-baseline.md"
    trace = DOCS / "02-architecture" / "requirements-architecture-traceability.md"
    baseline_text = baseline.read_text(encoding="utf-8")
    trace_text = trace.read_text(encoding="utf-8")
    baseline_ids = re.findall(r"^\|\s*(PAW-[A-Z]+-[0-9]{3})\s*\|", baseline_text, re.M)
    trace_ids = re.findall(r"^\|\s*(PAW-[A-Z]+-[0-9]{3})\s*\|", trace_text, re.M)
    if len(baseline_ids) != len(set(baseline_ids)):
        errors.append("requirements-baseline.md: duplicate formal requirement IDs")
    if set(baseline_ids) != set(trace_ids):
        errors.append(
            "requirements traceability mismatch: "
            f"missing={sorted(set(baseline_ids) - set(trace_ids))}, "
            f"extra={sorted(set(trace_ids) - set(baseline_ids))}"
        )

    adr_index = DOCS / "03-adr" / "README.md"
    index_text = adr_index.read_text(encoding="utf-8")
    indexed: set[str] = set()
    for adr_id, target in ADR_LINK.findall(index_text):
        target_path = (adr_index.parent / target.split("#", 1)[0]).resolve()
        if not target_path.is_file():
            errors.append(f"ADR index: {adr_id} target missing: {target}")
        indexed.add(adr_id)
    adr_files = sorted(DOCS.joinpath("03-adr").glob("ADR-[0-9][0-9][0-9]-*.md"))
    actual = {p.stem.split("-", 2)[1] for p in adr_files}
    if actual != EXPECTED_ADR_NUMBERS:
        errors.append(f"ADR file set mismatch: expected={sorted(EXPECTED_ADR_NUMBERS)}, actual={sorted(actual)}")
    if indexed != {f"ADR-{n}" for n in EXPECTED_ADR_NUMBERS}:
        errors.append(f"ADR index mismatch: indexed={sorted(indexed)}")
    for path in adr_files:
        body = path.read_text(encoding="utf-8")
        for field in ("Implementation Status:", "Verification Status:"):
            if field not in body:
                errors.append(f"{path.relative_to(ROOT)}: missing {field}")
        if not re.search(r"^##\s+(?:Risks?\b|风险)", body, re.M):
            errors.append(f"{path.relative_to(ROOT)}: missing risks section")
        if not re.search(r"^##\s+(?:Validation\b|待验证事项)", body, re.M):
            errors.append(f"{path.relative_to(ROOT)}: missing validation section")

    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    if "E0.2.2.3 Final Documentation Remediation & Freeze Gate" not in readme:
        errors.append("README.md: phase marker is not E0.2.2.3")

    for path in markdown:
        body = path.read_text(encoding="utf-8")
        for raw in LINK.findall(body):
            target = raw.strip().split()[0].strip("<>")
            parsed = urlparse(target)
            if parsed.scheme or target.startswith(("#", "mailto:")):
                continue
            target_path = (path.parent / target.split("#", 1)[0]).resolve()
            if not target_path.exists():
                errors.append(f"{path.relative_to(ROOT)}: broken relative link: {target}")

    print(f"Checked {len(markdown)} Markdown files; {len(baseline_ids)} requirement IDs; {len(adr_files)} ADRs.")
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        print(f"FAILED: {len(errors)} issue(s).")
        return 1
    print("PASS: documentation structure, links, requirements traceability, ADR governance and phase marker.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
