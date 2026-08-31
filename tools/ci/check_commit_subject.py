#!/usr/bin/env python3
"""Check one commit message against the maintained-branch contract."""

from __future__ import annotations

import re
import subprocess
import sys


ALLOWED_TYPES = (
    "core",
    "cli",
    "client",
    "resources",
    "build",
    "deps",
    "tools",
    "docs",
    "ci",
    "test",
    "chore",
    "refactor",
)
SUBJECT_RE = re.compile(r"^(" + "|".join(ALLOWED_TYPES) + r"): \S")


def lint(message: str) -> list[str]:
    errors: list[str] = []
    stripped = message.rstrip()
    lines = stripped.splitlines()
    if not lines or not lines[0].strip():
        return ["commit subject: empty message"]
    subject = lines[0]
    if len(lines) != 1:
        errors.append("commit subject: message must be one line")
    if not subject.isascii():
        errors.append("commit subject: contains non-ASCII text")
    if len(subject) > 50:
        errors.append(f"commit subject: exceeds 50 chars ({len(subject)})")
    if not SUBJECT_RE.match(subject):
        errors.append("commit subject: missing or invalid type")
    if "(" in subject or ")" in subject:
        errors.append("commit subject: contains parentheses")
    return errors


def head_message() -> str:
    result = subprocess.run(
        ["git", "log", "-1", "--format=%B"],
        capture_output=True,
        check=True,
        text=True,
    )
    return result.stdout


def main() -> int:
    if len(sys.argv) != 1:
        print("usage: check_commit_subject.py", file=sys.stderr)
        return 2
    errors = lint(head_message())
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print("Commit subject: passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
