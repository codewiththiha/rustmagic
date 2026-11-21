#!/usr/bin/env python3
"""Builds the Linux release files from binaries that are already compiled.

    python3 tools/package/linux.py --version 0.2.0

Produces, in `dist/`:

    rustmagic-<v>-linux-<arch>.tar.gz          portable tree, unpack anywhere
    rustmagic-<v>-linux-<arch>.tar.gz.sha256
    rustmagic_<v>_<debarch>.deb                installs with apt, dpkg or gdebi

Both come from one staging function, so the tarball and the package can never
disagree about what exists. Run it after `cargo build --release -p rustmagic
-p rustmagic-cli -p rustmagic-ibus -p km-ffi`.

`--arch` names the architecture of the binaries, not of this machine, so the
same script packages a cross build that was compiled under `target/<triple>`:

    python3 tools/package/linux.py --version 0.2.0 \
        --bin-dir target/aarch64-unknown-linux-gnu/release --arch aarch64

A musl build gets `--flavour musl`, which lands in the file names so a static
tarball and a glibc package of the same architecture cannot be confused.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

BINARIES = ("rustmagic", "rustmagic-cli", "rustmagic-ibus")


def ffi_library(binaries: Path) -> tuple[Path, str]:
    """The C ABI of this build: a shared object, or a static archive on musl.

    Rust has no `cdylib` for a musl target, so a static build has an archive and
    no `.so`. Shipping the archive under its own name is honest; naming it
    libkm_ffi.so would be a lie a linker would later discover.
    """
    for name in ("libkm_ffi.so", "libkm_ffi.a"):
        candidate = binaries / name
        if candidate.exists():
            return candidate, name
    raise SystemExit(f"no C ABI library in {binaries}: neither libkm_ffi.so nor libkm_ffi.a")

# uname-style architecture to the name Debian uses in `Architecture:`.
DEB_ARCH = {"x86_64": "amd64", "aarch64": "arm64", "i686": "i386"}
KEYBOARDS = "keyboards/bundled"
DOCUMENTS = ("README.md", "LICENSE", "docs/LINUX.md")

# Anything the window links against that no Rust dependency declares: the
# loader needs libGL and an X/Wayland stack, and the IBus engine needs the
# component directory IBus reads.
DEB_FALLBACK_DEPENDS = (
    "libc6",
    "libgcc-s1",
    "libgl1",
    "libxkbcommon0",
    "libx11-6",
    "libxcursor1",
    "libxrandr2",
    "libxi6",
    "libwayland-client0",
    "ibus",
)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument(
        "--bin-dir",
        default="target/release",
        help="where the compiled binaries are, e.g. target/<triple>/release",
    )
    parser.add_argument("--arch", default="x86_64", help="architecture of those binaries")
    parser.add_argument(
        "--flavour", default="", help="qualifier for the file names, e.g. musl"
    )
    parser.add_argument(
        "--no-deb", action="store_true", help="tarball only, for a libc the .deb cannot describe"
    )
    parser.add_argument("--out", default="dist")
    parser.add_argument("--maintainer", default="RustMagic contributors <noreply@rustmagic.invalid>")
    arguments = parser.parse_args(argv)

    version = arguments.version
    binaries = ROOT / arguments.bin_dir
    out = ROOT / arguments.out
    out.mkdir(parents=True, exist_ok=True)
    suggestion = f"-{arguments.flavour}" if arguments.flavour else ""
    name = f"rustmagic-{version}-linux-{arguments.arch}{suggestion}"

    portable = out / name
    shutil.rmtree(portable, ignore_errors=True)
    fill_portable(portable, binaries, version)
    tarball = out / f"{name}.tar.gz"
    subprocess.run(
        ["tar", "czf", str(tarball), "-C", str(out), name],
        check=True,
    )
    digest = hashlib.sha256(tarball.read_bytes()).hexdigest()
    (out / f"{name}.tar.gz.sha256").write_text(f"{digest}  {tarball.name}\n")
    print(f"{tarball.relative_to(ROOT)}  {tarball.stat().st_size} bytes")

    if arguments.no_deb:
        return 0
    deb_arch = DEB_ARCH.get(arguments.arch)
    if deb_arch is None:
        print(f"no Debian architecture known for {arguments.arch}", file=sys.stderr)
        return 2
    deb = out / f"rustmagic_{version}_{deb_arch}.deb"
    build_deb(out / "deb", binaries, version, arguments.maintainer, deb, deb_arch)
    print(f"{deb.relative_to(ROOT)}  {deb.stat().st_size} bytes")
    return 0


def fill_portable(tree: Path, binaries: Path, version: str) -> None:
    """The unpack-anywhere layout, which is a prefix: `cp -r . ~/.local/`."""
    share = tree / "share"
    for directory in (
        tree / "bin",
        tree / "lib",
        tree / "include",
        share / "applications",
        share / "icons/hicolor/scalable/apps",
        share / "ibus/component",
        share / "rustmagic/keyboards",
        share / "doc/rustmagic",
        share / "licenses/rustmagic",
    ):
        directory.mkdir(parents=True, exist_ok=True)

    for binary in BINARIES:
        install(binaries / binary, tree / "bin" / binary, 0o755)
    library, library_name = ffi_library(binaries)
    install(library, tree / "lib" / library_name, 0o644)
    # Laid out as a column in the INSTALL text; a musl archive is one character
    # shorter than the shared object it replaces.
    library_column = f"{'lib/' + library_name:<21}"
    install(ROOT / "crates/km-ffi/include/rustmagic.h", tree / "include/rustmagic.h", 0o644)
    install(ROOT / "assets/ibus/rustmagic.xml", share / "ibus/component/rustmagic.xml", 0o644)
    install(
        ROOT / "packaging/linux/rustmagic.desktop",
        share / "applications/rustmagic.desktop",
        0o644,
    )
    for icon in ("rustmagic.svg", "rustmagic-ibus.svg"):
        install(ROOT / f"packaging/linux/{icon}", share / f"icons/hicolor/scalable/apps/{icon}", 0o644)
    for keyboard in sorted((ROOT / KEYBOARDS).glob("*.km2")):
        install(keyboard, share / "rustmagic/keyboards" / keyboard.name, 0o644)
    install(ROOT / "README.md", share / "doc/rustmagic/README.md", 0o644)
    install(ROOT / "docs/LINUX.md", share / "doc/rustmagic/LINUX.md", 0o644)
    install(ROOT / "LICENSE", share / "licenses/rustmagic/LICENSE", 0o644)
    # The Myanmar face is linked into every binary, so its licence travels with
    # the binary: the OFL asks for the text to be handed out, not just to exist.
    install(ROOT / "assets/fonts/OFL.txt", share / "licenses/rustmagic/OFL-NotoSansMyanmar.txt", 0o644)
    install(ROOT / "README.md", tree / "README.md", 0o644)
    install(ROOT / "docs/LINUX.md", tree / "LINUX.md", 0o644)
    install(ROOT / "LICENSE", tree / "LICENSE", 0o644)
    install(ROOT / "assets/fonts/OFL.txt", tree / "OFL-NotoSansMyanmar.txt", 0o644)
    (tree / "INSTALL").write_text(
        f"""RustMagic {version} on Linux

  bin/rustmagic          the window, for managing and trying keyboards
  bin/rustmagic-cli      the same thing in a terminal
  bin/rustmagic-ibus     the input method IBus starts
  share/rustmagic/keyboards/   the keyboards that ship with it
  {library_column} the C ABI, for another input-method framework
  include/rustmagic.h    its header
  LICENSE                the licence of this program
  OFL-NotoSansMyanmar.txt  the licence of the font compiled into it

To install for the current user:

  cp -r . ~/.local/

To install system wide, or if you want the IBus engine to be picked up by
every user, install the .deb instead (or use packaging/arch/PKGBUILD on Arch).
After copying the component file into place:

  sudo install -Dm644 share/ibus/component/rustmagic.xml \\
       /usr/share/ibus/component/rustmagic.xml
  ibus write-cache && ibus restart

Then add "RustMagic" in your desktop's input-method settings. LINUX.md has the
details if nothing appears.
"""
    )


def fill_debian(
    tree: Path, binaries: Path, version: str, maintainer: str, deb_arch: str = "amd64"
) -> None:
    """The same files in the layout Debian expects, plus `DEBIAN/control`."""
    shutil.rmtree(tree, ignore_errors=True)
    fill_portable(tree, binaries, version)
    # A package keeps documentation in /usr/share/doc and nothing loose at the
    # top; the portable INSTALL text describes the tarball, not the package.
    for loose in ("INSTALL", "README.md", "LINUX.md", "LICENSE", "OFL-NotoSansMyanmar.txt"):
        (tree / loose).unlink(missing_ok=True)
    documentation = tree / "share/doc/rustmagic"
    shutil.copyfile(ROOT / "LICENSE", documentation / "copyright")
    shutil.copyfile(
        ROOT / "assets/fonts/OFL.txt", documentation / "OFL-NotoSansMyanmar.txt"
    )
    (tree / "share/licenses/rustmagic/LICENSE").unlink(missing_ok=True)
    (tree / "share/licenses/rustmagic/OFL-NotoSansMyanmar.txt").unlink(missing_ok=True)
    (tree / "share/licenses/rustmagic").rmdir()
    (tree / "share/licenses").rmdir()

    # /usr/lib/rustmagic rather than /usr/lib: the C ABI has no soname, so a
    # package must not pretend it is a system library.
    library = tree / "usr/lib/rustmagic"
    library.mkdir(parents=True, exist_ok=True)
    _, library_name = ffi_library(binaries)
    shutil.move(str(tree / "lib" / library_name), library / library_name)
    (tree / "lib").rmdir()
    (tree / "usr/include").mkdir(parents=True, exist_ok=True)
    shutil.move(str(tree / "include/rustmagic.h"), tree / "usr/include/rustmagic.h")
    (tree / "include").rmdir()
    for part in ("bin", "share"):
        shutil.move(str(tree / part), tree / "usr" / part)

    control = tree / "DEBIAN"
    control.mkdir(parents=True, exist_ok=True)
    depends = deb_depends(tree)
    (control / "control").write_text(
        f"""Package: rustmagic
Version: {version}
Architecture: {deb_arch}
Maintainer: {maintainer}
Installed-Size: {installed_size(tree)}
Depends: {', '.join(depends)}
Section: utils
Priority: optional
Homepage: https://github.com/codewiththiha/rustmagic
Description: keyboard layout engine for Myanmar and other scripts, in Rust
 RustMagic is a port of KeyMagic. It reads the same .km2 keyboards, compiles
 the same .kms sources and matches the same rules, and it ships six Myanmar
 layouts so it types out of the box.
 .
 It also provides an IBus engine, so the layouts work in every application on
 a Linux desktop, and a C ABI (libkm_ffi.so) for other input-method
 frameworks.
"""
    )
    write_script(
        control / "postinst",
        """#!/bin/sh
set -e
# IBus caches the component files it finds; without this the engine exists but
# no keyboard chooser lists it.
if [ -x /usr/bin/ibus ]; then
    ibus write-cache >/dev/null 2>&1 || true
fi
exit 0
""",
    )
    write_script(
        control / "postrm",
        """#!/bin/sh
set -e
if [ -x /usr/bin/ibus ]; then
    ibus write-cache >/dev/null 2>&1 || true
fi
exit 0
""",
    )


def write_script(path: Path, text: str) -> None:
    path.write_text(text)
    path.chmod(0o755)


def deb_depends(tree: Path) -> list[str]:
    """Library dependencies, from the binaries themselves when we can ask."""
    if not shutil.which("dpkg-shlibdeps"):
        return list(DEB_FALLBACK_DEPENDS)
    binaries = [tree / "usr/bin" / binary for binary in BINARIES]
    # dpkg-shlibdeps insists on being run inside a source package, so it gets
    # the smallest one that satisfies it.
    with tempfile.TemporaryDirectory() as scratch:
        (Path(scratch) / "debian").mkdir()
        (Path(scratch) / "debian/control").write_text(
            "Source: rustmagic\n\nPackage: rustmagic\nArchitecture: any\n"
        )
        result = subprocess.run(
            ["dpkg-shlibdeps", "-O", *[str(binary) for binary in binaries]],
            capture_output=True,
            text=True,
            cwd=scratch,
        )
    if result.returncode != 0:
        print(f"dpkg-shlibdeps: {result.stderr.strip()}", file=sys.stderr)
        return list(DEB_FALLBACK_DEPENDS)
    found: list[str] = []
    for line in result.stdout.splitlines():
        if line.startswith("shlibs:Depends="):
            found = [item.strip() for item in line.split("=", 1)[1].split(",") if item.strip()]
    if not found:
        return list(DEB_FALLBACK_DEPENDS)
    # dpkg-shlibdeps only sees libraries the loader resolves directly. winit
    # opens libGL and the X/Wayland stack at runtime, so those have to be added
    # by hand or the window fails to start on a fresh install.
    names = {dependency.split()[0] for dependency in found}
    for dependency in DEB_FALLBACK_DEPENDS:
        if dependency not in names:
            found.append(dependency)
    return found


def build_deb(
    tree: Path,
    binaries: Path,
    version: str,
    maintainer: str,
    out: Path,
    deb_arch: str = "amd64",
) -> None:
    fill_debian(tree, binaries, version, maintainer, deb_arch)
    subprocess.run(
        [
            "dpkg-deb",
            "--root-owner-group",
            "-Zxz",
            "--build",
            str(tree),
            str(out),
        ],
        check=True,
    )


def installed_size(tree: Path) -> int:
    total = 0
    for path in tree.rglob("*"):
        if path.is_file() and "DEBIAN" not in path.parts:
            total += path.stat().st_size
    return max(1, (total + 1023) // 1024)


def install(source: Path, target: Path, mode: int) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    os.chmod(target, mode)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
