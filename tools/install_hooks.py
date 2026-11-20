#!/usr/bin/env python3
"""Puts the repository's hooks where git looks for them.

    python3 tools/install_hooks.py

Hooks are copied rather than symlinked so a checkout on Windows behaves the same
way, and each one is made executable.
"""

from __future__ import annotations

import shutil
import stat
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def git_dir() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--git-dir"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return (ROOT / result.stdout.strip()).resolve()


def main() -> int:
    if not (ROOT / ".git").exists() and not git_dir().exists():
        print("not a git repository", file=sys.stderr)
        return 1
    hooks = git_dir() / "hooks"
    hooks.mkdir(parents=True, exist_ok=True)
    installed = 0
    for source in sorted((ROOT / "tools" / "hooks").iterdir()):
        target = hooks / source.name
        shutil.copyfile(source, target)
        target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        print(f"installed {source.name}")
        installed += 1
    print(f"{installed} hook(s) installed in {hooks}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
