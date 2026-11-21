#!/usr/bin/env python3
"""Repository shape rules, checked without compiling anything.

These are the rules that a compiler cannot see but a reviewer always can: where
`unsafe` is allowed, which way the crate graph points, what a placeholder left
behind, whether the documents exist. CI runs this first because it is instant.

    python3 tools/check_repo.py          # check
    python3 tools/check_repo.py --list   # list the rules
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import plistlib
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
APPS = ROOT / "apps"

# The only file that may name `unsafe`: the C ABI, where it is unavoidable.
UNSAFE_ALLOWED = {"crates/km-ffi/src/lib.rs"}

# A crate may not depend on a crate to its right.
LAYERS = [
    ["km-core"],
    ["km-binary", "km-script"],
    ["km-engine", "km-store"],
    ["km-session"],
    ["km-ffi", "ui-egui", "km-ibus"],
    ["rustmagic", "rustmagic-cli", "rustmagic-ibus"],
]

REQUIRED_DOCS = [
    "README.md",
    "AGENTS.md",
    "CLAUDE.md",
    "LICENSE",
    "docs/ARCHITECTURE.md",
    "docs/ENGINE_SEMANTICS.md",
    "docs/PORTING.md",
    "docs/AGENT_WORKFLOW.md",
    "docs/LINUX.md",
    "docs/RELEASING.md",
]

PLACEHOLDER = re.compile(r"//!\s*Placeholder\.|fn main\(\) \{\}")
UNSAFE_USE = re.compile(r"\bunsafe\b")


def run(*command: str) -> str:
    return subprocess.run(
        command, cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def manifests() -> list[Path]:
    return sorted(CRATES.glob("*/Cargo.toml")) + sorted(APPS.glob("*/Cargo.toml"))


def package_name(manifest: Path) -> str:
    return manifest.parent.name


def check_unsafe() -> list[str]:
    problems = []
    for path in [*CRATES.rglob("*.rs"), *APPS.rglob("*.rs")]:
        relative = path.relative_to(ROOT).as_posix()
        if relative in UNSAFE_ALLOWED:
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("///"):
                continue
            if UNSAFE_USE.search(line):
                problems.append(f"{relative}:{number}: `unsafe` outside the C ABI")
    return problems


def check_layers() -> list[str]:
    rank = {name: index for index, layer in enumerate(LAYERS) for name in layer}
    problems = []
    for manifest in manifests():
        name = package_name(manifest)
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        for table in ("dependencies", "dev-dependencies"):
            for dependency in data.get(table, {}):
                if dependency in rank and rank[dependency] > rank.get(name, 0):
                    problems.append(
                        f"{name} depends on {dependency}, which sits above it"
                    )
    # The window crate is the top of the graph and may of course have one; every
    # crate the engine runs through may not.
    windowed = {"ui-egui", "rustmagic"}
    for manifest in [*CRATES.glob("*/Cargo.toml"), *APPS.glob("*/Cargo.toml")]:
        name = package_name(manifest)
        if name in windowed:
            continue
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        deps = set(data.get("dependencies", {}))
        if {"egui", "eframe"} & deps:
            problems.append(f"{name} has a window dependency")
    return problems


def check_placeholders() -> list[str]:
    problems = []
    for path in [*CRATES.rglob("*.rs"), *APPS.rglob("*.rs"), *ROOT.glob("xtask/src/*.rs")]:
        text = path.read_text(encoding="utf-8")
        if PLACEHOLDER.search(text):
            problems.append(f"{path.relative_to(ROOT).as_posix()}: still a placeholder")
        code = [
            line
            for line in text.splitlines()
            if line.strip()
            and not line.strip().startswith(("//", "#[", "/*", "*", "*/"))
        ]
        if len(code) < 5:
            problems.append(
                f"{path.relative_to(ROOT).as_posix()}: "
                f"{len(code)} lines of code is a stub"
            )
    return problems


def check_docs() -> list[str]:
    problems = []
    for document in REQUIRED_DOCS:
        path = ROOT / document
        if not path.exists():
            problems.append(f"{document} is missing")
        elif path.stat().st_size < 400:
            problems.append(f"{document} is too thin to be useful")
    agents = ROOT / "AGENTS.md"
    claude = ROOT / "CLAUDE.md"
    if agents.exists() and claude.exists():
        if agents.read_text(encoding="utf-8") != claude.read_text(encoding="utf-8"):
            problems.append("CLAUDE.md is a copy of AGENTS.md and has drifted")
    for document in ROOT.glob("docs/*.md"):
        text = document.read_text(encoding="utf-8")
        for target in re.findall(r"\]\((?!https?:)([^)#]+)", text):
            if not (ROOT / "docs").joinpath(target).exists() and not (ROOT / target).exists():
                problems.append(
                    f"{document.relative_to(ROOT).as_posix()} links to missing {target}"
                )
    return problems


def check_edition() -> list[str]:
    """The workspace targets edition 2024, and nothing quietly downgrades it."""
    problems = []
    data = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    package = data.get("workspace", {}).get("package", {})
    if package.get("edition") != "2024":
        problems.append("the workspace must set edition = 2024")
    rust_version = package.get("rust-version", "0.0")
    try:
        major, minor = (int(part) for part in rust_version.split(".")[:2])
    except ValueError:
        major, minor = 0, 0
    if (major, minor) < (1, 85):
        problems.append(f"edition 2024 needs rust-version >= 1.85, not {rust_version}")
    for manifest in manifests():
        text = manifest.read_text(encoding="utf-8")
        for downgrade in ("2021", "2018", "2015"):
            if f'edition = "{downgrade}"' in text:
                problems.append(
                    f"{manifest.relative_to(ROOT).as_posix()}: "
                    f"edition {downgrade} overrides the workspace"
                )
    return problems


def check_release() -> list[str]:
    """What a release promises, checked before it can be tagged."""
    problems = []
    workflow = ROOT / ".github" / "workflows" / "release.yml"
    if not workflow.exists():
        return ["the release workflow is missing"]
    text = workflow.read_text(encoding="utf-8")
    for platform in ("linux", "rpm", "macos", "windows", "arch"):
        if f"\n  {platform}:\n" not in text:
            problems.append(f"the release workflow has no {platform} job")
    if "action-gh-release" not in text:
        problems.append("the release workflow never uploads anything")
    plist = ROOT / "packaging" / "macos" / "Info.plist"
    if not plist.exists():
        problems.append("packaging/macos/Info.plist is missing")
    else:
        try:
            bundle = plistlib.loads(plist.read_bytes())
        except Exception as error:  # noqa: BLE001 - any parse failure is the problem
            problems.append(f"packaging/macos/Info.plist does not parse: {error}")
        else:
            for key in ("CFBundleExecutable", "CFBundleIdentifier", "CFBundleIconFile"):
                if key not in bundle:
                    problems.append(f"packaging/macos/Info.plist has no {key}")
    header = ROOT / "crates" / "km-ffi" / "include" / "rustmagic.h"
    if not header.exists():
        problems.append("crates/km-ffi/include/rustmagic.h is missing")
    else:
        declarations = header.read_text(encoding="utf-8")
        for symbol in ("rm_keyboard_load", "rm_engine_key", "rm_string_free"):
            if symbol not in declarations:
                problems.append(f"rustmagic.h does not declare {symbol}")
    for installer in (
        "packaging/rpm/rustmagic.spec",
        "packaging/windows/rustmagic.iss",
        "packaging/windows/rustmagic.wxs",
        "packaging/brand/rustmagic.svg",
        "tools/package/linux.py",
        "tools/package/dmg.sh",
        "tools/package/icons.sh",
        "tools/package/check_icon.py",
    ):
        if not (ROOT / installer).exists():
            problems.append(f"{installer} is missing")
    pkgbuild = ROOT / "packaging" / "arch" / "PKGBUILD"
    if pkgbuild.exists():
        text = pkgbuild.read_text(encoding="utf-8")
        for line in text.splitlines():
            if line.startswith("install="):
                script = line.split("=", 1)[1].strip()
                # A PKGBUILD that names an install script it does not come with
                # fails at `makepkg -si`, which is how a user finds out.
                if not (pkgbuild.parent / script).exists():
                    problems.append(f"PKGBUILD installs {script}, which is not beside it")
    example = ROOT / "tools" / "smoke" / "abi.c"
    if not example.exists():
        problems.append("tools/smoke/abi.c is missing")
    elif '#include "rustmagic.h"' not in example.read_text(encoding="utf-8"):
        problems.append("tools/smoke/abi.c does not use the installed header")
    return problems


def check_bundled() -> list[str]:
    """The keyboards on disk and the ones compiled in are the same five."""
    problems = []
    directory = ROOT / "keyboards" / "bundled"
    files = {path.name for path in directory.glob("*.km2")} if directory.exists() else set()
    if not files:
        return ["keyboards/bundled has no keyboards"]
    module = ROOT / "crates" / "km-store" / "src" / "bundled.rs"
    text = module.read_text(encoding="utf-8") if module.exists() else ""
    for name in sorted(files):
        if name not in text:
            problems.append(f"keyboards/bundled/{name} is not embedded by km-store")
    for name in re.findall(r'include_bytes!\("../../../(keyboards/bundled/[^"]+)"\)', text):
        if Path(name).name not in files:
            problems.append(f"km-store embeds {name}, which does not exist")
    return problems


def check_manifests() -> list[str]:
    problems = []
    for manifest in manifests():
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        package = data.get("package", {})
        for field in ("name", "description", "license", "repository"):
            if field not in package:
                problems.append(
                    f"{manifest.relative_to(ROOT).as_posix()}: no `{field}`"
                )
        if package.get("publish") is not False:
            problems.append(
                f"{manifest.relative_to(ROOT).as_posix()}: must set publish = false"
            )
    return problems


def check_tools() -> list[str]:
    problems = []
    for script in (ROOT / "tools").glob("*.py"):
        text = script.read_text(encoding="utf-8")
        if not text.startswith("#!/usr/bin/env python3"):
            problems.append(f"{script.name} has no shebang")
        if "def main(" not in text:
            problems.append(f"{script.name} has no main()")
    hooks = ROOT / "tools" / "hooks"
    for hook in hooks.iterdir() if hooks.exists() else []:
        if not hook.read_text(encoding="utf-8").startswith("#!/bin/sh"):
            problems.append(f"tools/hooks/{hook.name} is not a shell hook")
    return problems


def check_workspace() -> list[str]:
    data = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    members = data["workspace"]["members"]
    installed = {
        path.parent.name
        for path in [*CRATES.glob("*/Cargo.toml"), *APPS.glob("*/Cargo.toml")]
    }
    problems = []
    for member in members:
        path = ROOT / member
        if "*" in member:
            continue
        if not (path / "Cargo.toml").exists():
            problems.append(f"workspace member {member} has no manifest")
        elif path.parent == ROOT and path.name not in installed - set():
            # Root-level tools such as xtask are checked by their manifest alone.
            continue
    lints = data["workspace"]["lints"]["rust"]
    if lints.get("unsafe_code") == "forbid":
        problems.append("`forbid(unsafe_code)` cannot be relaxed by km-ffi; use deny")
    return problems


def check_fixtures() -> list[str]:
    problems = []
    fixtures = ROOT / "fixtures"
    if not fixtures.exists():
        return ["fixtures/ is missing"]
    for name in ("minimal.kms", "minimal.km2", "basic.kms", "basic.km2"):
        if not (fixtures / name).exists():
            problems.append(f"fixtures/{name} is missing")
    assets = ROOT / "assets" / "keyboards"
    if not any(assets.glob("*.kms")):
        problems.append("assets/keyboards has no keyboard to ship")
    return problems


def check_assets() -> list[str]:
    """The files a desktop reads, checked for the mistakes a desktop would hit."""
    problems = []
    try:
        import xml.dom.minidom

        for document in (ROOT / "assets").rglob("*.xml"):
            try:
                xml.dom.minidom.parse(str(document))
            except Exception as error:  # noqa: BLE001 - the message is the point
                problems.append(f"{document.relative_to(ROOT).as_posix()}: {error}")
    except ImportError:  # pragma: no cover - python always ships minidom
        pass
    component = ROOT / "assets" / "ibus" / "rustmagic.xml"
    if not component.exists():
        problems.append("assets/ibus/rustmagic.xml is missing; IBus cannot find us")
    else:
        text = component.read_text(encoding="utf-8")
        for required in ("<engines>", "<exec>", "<name>rustmagic</name>"):
            if required not in text:
                problems.append(f"assets/ibus/rustmagic.xml has no {required}")
    for name in ("rustmagic.desktop", "rustmagic.svg", "rustmagic-ibus.svg"):
        if not (ROOT / "packaging" / "linux" / name).exists():
            problems.append(f"packaging/linux/{name} is missing")
    for name in ("PKGBUILD", "rustmagic.install"):
        if not (ROOT / "packaging" / "arch" / name).exists():
            problems.append(f"packaging/arch/{name} is missing")
    problems.extend(check_icon_set())
    desktop = ROOT / "packaging" / "linux" / "rustmagic.desktop"
    if desktop.exists():
        text = desktop.read_text(encoding="utf-8")
        for required in ("[Desktop Entry]", "Exec=", "Icon=", "Categories="):
            if required not in text:
                problems.append(f"packaging/linux/rustmagic.desktop has no {required}")
    return problems


def check_icon_set() -> list[str]:
    """The icon set the window is built from.

    `crates/ui-egui/src/icons.rs` names one file per `Icon` variant and embeds
    it with `include_bytes!`, so a deleted file fails the build. What the build
    cannot see is a file that is there but is not a tintable 24 point SVG, and
    a bitmap creeping back into a set that is supposed to be vectors.
    """
    problems = []
    directory = ROOT / "assets" / "icons"
    if not directory.is_dir():
        return ["assets/icons/ is missing; the window has no icons"]
    bitmaps = [
        path.relative_to(ROOT).as_posix()
        for path in directory.iterdir()
        if path.suffix.lower() in {".png", ".ico", ".jpg", ".jpeg", ".webp", ".bmp"}
    ]
    for name in bitmaps:
        problems.append(f"{name}: the icon set is SVG only, not bitmaps")
    for path in sorted(directory.glob("*.svg")):
        text = path.read_text(encoding="utf-8")
        name = path.relative_to(ROOT).as_posix()
        if "<svg" not in text:
            problems.append(f"{name}: is not an SVG document")
        if 'viewBox="0 0 24 24"' not in text:
            problems.append(f"{name}: is not drawn on the 24 point grid")
        if "currentColor" not in text:
            problems.append(f"{name}: cannot be tinted, it does not use currentColor")
    source = ROOT / "crates" / "ui-egui" / "src" / "icons.rs"
    if source.exists():
        text = source.read_text(encoding="utf-8")
        for referenced in set(__import__("re").findall(r'assets/icons/([a-z-]+\.svg)', text)):
            if not (directory / referenced).exists():
                problems.append(f"assets/icons/{referenced}: named by icons.rs but missing")
    fonts = ROOT / "assets" / "fonts"
    if not fonts.is_dir():
        problems.append("assets/fonts/ is missing; Myanmar would draw as boxes")
    elif not (fonts / "OFL.txt").exists():
        problems.append("assets/fonts/OFL.txt is missing; the font cannot ship without it")
    return problems


RULES = {
    "unsafe": check_unsafe,
    "layers": check_layers,
    "placeholders": check_placeholders,
    "docs": check_docs,
    "manifests": check_manifests,
    "tools": check_tools,
    "workspace": check_workspace,
    "fixtures": check_fixtures,
    "assets": check_assets,
    "edition": check_edition,
    "bundled": check_bundled,
    "release": check_release,
}


def main(argv: list[str]) -> int:
    if "--list" in argv:
        for name in RULES:
            print(name)
        return 0
    if "--json" in argv:
        report = {name: rule() for name, rule in RULES.items()}
        print(json.dumps(report, indent=2))
        return 0 if not any(report.values()) else 1

    failed = 0
    for name, rule in RULES.items():
        problems = rule()
        if problems:
            failed += len(problems)
            print(f"✗ {name}")
            for problem in problems:
                print(f"    {problem}")
        else:
            print(f"✓ {name}")
    if failed:
        print(f"\n{failed} problem(s). Fix them before committing.")
        return 1
    print("\nthe repository is in shape")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
