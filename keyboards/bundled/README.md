# The keyboards that ship with RustMagic

These six are what RustMagic installs on first run, so a fresh install types
Myanmar out of the box instead of showing an empty library. The layouts come
from [thantthet/keymagic-keyboards](https://github.com/thantthet/keymagic-keyboards),
which hosts KeyMagic layouts and states that submitted keyboards may be
redistributed.

They are **not** reimplementations. A layout is a program, and an approximation
of one types the wrong thing. The Pyidaungsu MM and MyanSan files shipped in 0.2.0
differed in size from KeyMagic's copies, so the authoritative files replaced them
byte for byte. That provenance correction alone does **not** prove the reported
`အ‌ေး` bug was fixed: KeyMagic's Pyidaungsu file itself emits U+200C for the `a`
key. In the CI engine check, the documented vowel-first sequence `a`, `t`, `;`
produces `အေး` with no invisible character, while the reversed `t`, `a`, `;`
sequence retains U+200C. The KeyMagic layout guide shows vowel-first typing (for
example, `a u s m i f ;` for `ကျောင်း`). Real macOS text-field typing has not
been observed; this is an engine regression check, not an end-to-end claim.

| File | Name inside the file | Bytes | MD5 | Provenance |
| --- | --- | ---: | --- | --- |
| `pyidaungsu-mm.km2` | Pyidaungsu MM | 6280 | `c593e2260ee87613335c1b09ef9823fe` | Official KeyMagic layout |
| `my-win.km2` | my-Win 2.3.0 Unicode 5.2 | 7338 | `89b880faa05921935c043104858d078a` | KeyMagic keyboard repository |
| `myansan.km2` | မြန်စံ (Smart) | 2888 | `5daf660c6f6290d2a588d3c236a2ca91` | Official KeyMagic layout |
| `zawgyi.km2` | ZawGyi | 8436 | `9440d1c945e973913f85e483f12ff996` | KeyMagic keyboard repository |
| `zawcode.km2` | ဇော်ကုဒ် | 8631 | `8883950a9b6474ba342b3d42506b5c9a` | Official KeyMagic layout |
| `myanmar3.km2` | Myanmar3std | 2184 | `4b6c0c76c0268a4eb7e4019cf64bd61b` | [Myanmar3 folder](https://github.com/thantthet/keymagic-keyboards/tree/master/Myanmar3) |

The three layouts KeyMagic's repository ships under `keyboards/bundled` —
Pyidaungsu MM, MyanSan, and ZawCode — are identical to those files; the hashes
above make that claim checkable. Myanmar3 is the official KeyMagic conversion of
the Myanmar3 Keyman layout. Its source maps Shift+G to U+103D (`ွ`). The other
layouts also come from the KeyMagic keyboard repository, not from a hand-written
copy.

The file names here are lowercase and dashed because they become library ids;
the name a user sees is the one inside the file.

They are embedded in the binary with `include_bytes!` by
`crates/km-store/src/bundled.rs` and installed on first run, so the same six
appear on every platform whether RustMagic came from a `.deb`, an `.msi`, a
`.dmg`, a `pkg.tar.zst` or a portable tarball. The app upgrades only the exact
Pyidaungsu MM and MyanSan files shipped by RustMagic 0.2.0; other existing files
are kept, so a user's edited keyboard is not overwritten. Packages also put a
copy in `share/rustmagic/keyboards/` for anyone who wants the files themselves.

Adding or replacing one: put the `.km2` in this directory, update the table, and
keep the file name a library id. `crates/km-store` tests that every listed file
parses, and `crates/km-engine/tests/diagnostics.rs` prints what each key types,
so a layout that decodes but types wrongly shows up in a log instead of in
someone's document.
