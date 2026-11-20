# The keyboards that ship with RustMagic

These five are what KeyMagic itself installs, so a fresh install types Myanmar
out of the box instead of showing an empty library. They are copied here
unmodified from
[keymagic-keyboards](https://github.com/thantthet/keymagic-keyboards), which is
where every KeyMagic layout lives and which states that a submitted keyboard is
redistributable:

| File | Name inside the file | Rules |
| --- | --- | --- |
| `pyidaungsu-mm.km2` | Pyidaungsu MM | 72 |
| `my-win.km2` | my-Win 2.3.0 Unicode 5.2 | 113 |
| `myansan.km2` | မြန်စံ (Smart) | 76 |
| `zawgyi.km2` | ZawGyi | 257 |
| `zawcode.km2` | ဇော်ကုဒ် | 263 |

The file names here are lowercase and dashed because they become library ids;
the name a user sees is the one inside the file.

They are embedded in the binary with `include_bytes!` by
`crates/km-store/src/bundled.rs` and installed on first run, so the same five
appear on every platform whether RustMagic came from a `.deb`, an `.msi`, a
`.dmg`, a `pkg.tar.zst` or a portable tarball. Packages also put a copy in
`share/rustmagic/keyboards/` for anyone who wants the files themselves.

Adding one: drop the `.km2` in this directory, add a row to the table above and
a line to `BUNDLED`. `crates/km-store` has a test that every listed file parses,
so a broken file fails the build rather than a user's first run.
