//! The keyboards every install starts with.
//!
//! KeyMagic ships five Myanmar layouts; so does RustMagic. They are compiled
//! `.km2` files embedded in the binary, which keeps one list working for a
//! portable tarball, a `.deb`, a `.rpm`, a `.pkg.tar.zst`, an `.msi` and a
//! `.dmg` — there is no install path to find at runtime, and no way for a
//! package to forget one.
//!
//! The provenance of each file, and where they came from upstream, is in
//! `keyboards/bundled/README.md`.

/// One keyboard that travels inside the binary.
#[derive(Debug)]
pub struct Bundled {
    /// Library id: lowercase, so it doubles as a file name.
    pub id: &'static str,
    /// The `.km2` bytes.
    pub bytes: &'static [u8],
}

/// Every bundled keyboard, in the order a fresh library shows them.
pub const BUNDLED: &[Bundled] = &[
    Bundled {
        id: "pyidaungsu-mm",
        bytes: include_bytes!("../../../keyboards/bundled/pyidaungsu-mm.km2"),
    },
    Bundled {
        id: "my-win",
        bytes: include_bytes!("../../../keyboards/bundled/my-win.km2"),
    },
    Bundled {
        id: "myansan",
        bytes: include_bytes!("../../../keyboards/bundled/myansan.km2"),
    },
    Bundled {
        id: "zawgyi",
        bytes: include_bytes!("../../../keyboards/bundled/zawgyi.km2"),
    },
    Bundled {
        id: "zawcode",
        bytes: include_bytes!("../../../keyboards/bundled/zawcode.km2"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use km_engine::Keyboard;

    #[test]
    fn every_bundled_keyboard_parses_and_has_rules() {
        for bundled in BUNDLED {
            let keyboard = Keyboard::from_bytes(bundled.bytes)
                .unwrap_or_else(|error| panic!("{} does not load: {error}", bundled.id));
            assert!(keyboard.rule_count() > 0, "{} has no rules", bundled.id);
        }
    }

    #[test]
    fn ids_are_unique_and_lowercase() {
        let mut seen = std::collections::BTreeSet::new();
        for bundled in BUNDLED {
            assert!(seen.insert(bundled.id), "{} appears twice", bundled.id);
            assert_eq!(
                bundled.id,
                bundled.id.to_lowercase(),
                "{} is not lowercase",
                bundled.id
            );
        }
    }
}
