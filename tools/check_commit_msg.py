#!/usr/bin/env python3
"""Commit message rules.

Run by the `commit-msg` hook and by CI, so a message is checked before it exists
and again after it lands.

    python3 tools/check_commit_msg.py .git/COMMIT_EDITMSG
    echo "engine: match the longest rule" | python3 tools/check_commit_msg.py -

Rules, in the order they are most often broken:

  1. the subject is at most 50 characters;
  2. the subject is imperative and lowercase-starting unless it begins with an
     area prefix such as `engine:` or `docs:`;
  3. the subject does not end in a full stop;
  4. a body, when present, is separated by one blank line and wrapped at 72;
  5. no `wip`, `fixup!`, `squash!` or merge noise left behind.
"""

from __future__ import annotations

import sys

SUBJECT_LIMIT = 50
BODY_LIMIT = 72
AREAS = {
    "core",
    "binary",
    "script",
    "engine",
    "store",
    "session",
    "ffi",
    "ibus",
    "linux",
    "windows",
    "macos",
    "packaging",
    "ui",
    "app",
    "cli",
    "xtask",
    "tools",
    "docs",
    "ci",
    "repo",
    "assets",
    "release",
}
BANNED = ("wip", "fixup!", "squash!", "tmp", "asdf")


def read_message(argv: list[str]) -> str:
    if len(argv) != 2:
        print("usage: check_commit_msg.py <file>|-", file=sys.stderr)
        raise SystemExit(2)
    if argv[1] == "-":
        return sys.stdin.read()
    with open(argv[1], encoding="utf-8") as handle:
        return handle.read()


def check(message: str) -> list[str]:
    problems: list[str] = []
    skip = [line for line in message.splitlines() if not line.startswith("#")]
    while skip and not skip[0].strip():
        skip.pop(0)
    if not skip:
        return ["the message is empty"]

    subject = skip[0].rstrip()
    if len(subject) > SUBJECT_LIMIT:
        problems.append(
            f"subject is {len(subject)} characters, the limit is {SUBJECT_LIMIT}: "
            f"`{subject}`"
        )
    if subject.endswith("."):
        problems.append("subject ends with a full stop")
    lowered = subject.lower()
    for banned in BANNED:
        if lowered.startswith(banned):
            problems.append(f"subject starts with `{banned}`")
            break
    head = subject.split(":", 1)[0].lower() if ":" in subject else ""
    if head and head not in AREAS:
        problems.append(
            f"`{head}:` is not an area; use one of {', '.join(sorted(AREAS))}"
        )
    if head:
        rest = subject.split(":", 1)[1].strip()
        if not rest:
            problems.append("the area prefix has no subject after it")
        elif rest[0].isupper():
            problems.append("the subject after the area starts with a capital")
    elif subject[:1].isupper():
        problems.append("subject starts with a capital letter")

    body = skip[1:]
    if body:
        if body[0].strip():
            problems.append("the body must start after one blank line")
        for line in body:
            if len(line) > BODY_LIMIT and not line.startswith(("http", "  ")):
                problems.append(f"body line is {len(line)} characters: `{line[:40]}…`")
                break
    return problems


def main(argv: list[str]) -> int:
    message = read_message(argv)
    problems = check(message)
    if problems:
        print("commit message rejected:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            f"\nKeep the subject at {SUBJECT_LIMIT} characters and imperative,\n"
            "for example: `engine: match the longest rule first`",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
