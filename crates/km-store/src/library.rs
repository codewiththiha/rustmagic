//! A directory of keyboards, with their sources beside them.

use std::path::{Path, PathBuf};

use km_core::{Error, KeyboardMeta, LayoutOptions, Result};
use km_engine::Keyboard;

/// One installed keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Stable id, derived from the file name.
    pub id: String,
    /// The compiled file.
    pub path: PathBuf,
    /// Its metadata, as the file declares it.
    pub meta: KeyboardMeta,
    /// Number of rules.
    pub rules: usize,
    /// Behaviour flags.
    pub options: LayoutOptions,
    /// The `.kms` source, when the keyboard was installed from source.
    pub source: Option<PathBuf>,
    /// Size of the compiled file, in bytes.
    pub size: u64,
}

impl Entry {
    /// The name to show, which is the metadata name when there is one.
    pub fn display_name(&self) -> &str {
        if self.meta.name.is_empty() {
            &self.id
        } else {
            &self.meta.name
        }
    }
}

/// A directory holding compiled keyboards and their sources.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// Uses `root` as the library; nothing is created until something is stored.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory the library lives in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn keyboards(&self) -> PathBuf {
        self.root.join("keyboards")
    }

    fn sources(&self) -> PathBuf {
        self.root.join("sources")
    }

    /// Every readable keyboard, sorted by display name.
    ///
    /// A file that fails to load is skipped rather than failing the listing, so
    /// one damaged keyboard cannot hide the rest.
    pub fn list(&self) -> Vec<Entry> {
        let mut entries: Vec<Entry> = std::fs::read_dir(self.keyboards())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|file| {
                file.path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("km2"))
            })
            .filter_map(|file| self.entry(&file.path()).ok())
            .collect();
        entries.sort_by(|left, right| {
            left.display_name()
                .to_lowercase()
                .cmp(&right.display_name().to_lowercase())
        });
        entries
    }

    /// Reads one entry from a compiled file.
    pub fn entry(&self, path: &Path) -> Result<Entry> {
        let keyboard = Keyboard::from_file(path)?;
        let id = id_of(path)?;
        let source = self.sources().join(format!("{id}.kms"));
        Ok(Entry {
            meta: keyboard.meta.clone(),
            rules: keyboard.rule_count(),
            options: keyboard.options,
            source: source.is_file().then_some(source),
            size: std::fs::metadata(path).map_or(0, |meta| meta.len()),
            path: path.to_owned(),
            id,
        })
    }

    /// Loads a keyboard by id.
    pub fn load(&self, id: &str) -> Result<Keyboard> {
        Keyboard::from_file(self.keyboards().join(format!("{}.km2", safe_id(id)?)))
    }

    /// Reads a keyboard's source, when it was installed from source.
    pub fn source_of(&self, id: &str) -> Option<String> {
        let path = self.sources().join(format!("{}.kms", safe_id(id).ok()?));
        std::fs::read_to_string(path).ok()
    }

    /// Compiles a `.kms` file and installs it, keeping the source.
    pub fn install_source(&self, path: impl AsRef<Path>) -> Result<Entry> {
        let path = path.as_ref();
        let compiled = km_script::compile_file(path)?;
        let id = id_of(path)?;
        let bytes = compiled.to_bytes()?;
        self.store(&id, &bytes, Some(path))?;
        self.entry(&self.keyboards().join(format!("{id}.km2")))
    }

    /// Installs a compiled keyboard from memory; `name` becomes the id.
    pub fn install_bytes(&self, name: &str, bytes: &[u8]) -> Result<Entry> {
        let id = safe_id(name)?;
        // Decoding first means a bad file never becomes a broken library entry.
        let _keyboard = Keyboard::from_bytes(bytes)?;
        self.store(&id, bytes, None)?;
        self.entry(&self.keyboards().join(format!("{id}.km2")))
    }

    /// Installs an already compiled keyboard together with its source text.
    ///
    /// Used by the app for the keyboard it ships with: there is no file on disk
    /// to copy, so the bytes and the source are handed over directly.
    pub fn install_source_text(&self, name: &str, km2: &[u8], source: &str) -> Result<Entry> {
        let id = safe_id(name)?;
        let _keyboard = Keyboard::from_bytes(km2)?;
        self.store(&id, km2, None)?;
        std::fs::create_dir_all(self.sources())
            .map_err(|error| Error::io(self.sources(), &error))?;
        let source_path = self.sources().join(format!("{id}.kms"));
        std::fs::write(&source_path, source).map_err(|error| Error::io(&source_path, &error))?;
        self.entry(&self.keyboards().join(format!("{id}.km2")))
    }

    /// Installs missing bundled keyboards and replaces exact copies of the
    /// broken 0.2.0 Pyidaungsu MM and MyanSan files.
    ///
    /// Other existing files are left alone, including user-edited layouts: an
    /// upgrade only replaces those two byte-for-byte snapshots known to have
    /// shipped with the incorrect rules. Returns new and migrated entries.
    pub fn seed_bundled(&self) -> Result<Vec<Entry>> {
        let mut installed = Vec::new();
        for bundled in crate::BUNDLED {
            let path = self.keyboards().join(format!("{}.km2", bundled.id));
            if path.is_file() {
                let Some(previous) = legacy_snapshot(bundled.id) else {
                    continue;
                };
                let Ok(existing) = std::fs::read(&path) else {
                    continue;
                };
                if existing != previous {
                    continue;
                }
            }
            installed.push(self.install_bytes(bundled.id, bundled.bytes)?);
        }
        Ok(installed)
    }

    /// Every bundled keyboard, whether or not it is installed.
    pub fn bundled() -> &'static [crate::Bundled] {
        crate::BUNDLED
    }

    /// Deletes a keyboard and its source.
    pub fn remove(&self, id: &str) -> Result<()> {
        let id = safe_id(id)?;
        for path in [
            self.keyboards().join(format!("{id}.km2")),
            self.sources().join(format!("{id}.kms")),
        ] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::io(&path, &error)),
            }
        }
        Ok(())
    }

    fn store(&self, id: &str, bytes: &[u8], source: Option<&Path>) -> Result<()> {
        let keyboards = self.keyboards();
        std::fs::create_dir_all(&keyboards).map_err(|error| Error::io(&keyboards, &error))?;
        let target = keyboards.join(format!("{id}.km2"));
        let temporary = keyboards.join(format!("{id}.km2.tmp"));
        std::fs::write(&temporary, bytes).map_err(|error| Error::io(&temporary, &error))?;
        std::fs::rename(&temporary, &target).map_err(|error| Error::io(&target, &error))?;
        if let Some(source) = source {
            let sources = self.sources();
            std::fs::create_dir_all(&sources).map_err(|error| Error::io(&sources, &error))?;
            let target = sources.join(format!("{id}.kms"));
            std::fs::copy(source, &target).map_err(|error| Error::io(&target, &error))?;
        }
        Ok(())
    }
}

/// The id a path implies: its file stem, restricted to safe characters.
fn id_of(path: &Path) -> Result<String> {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(Error::BadPath {
            path: path.display().to_string(),
        });
    };
    safe_id(stem)
}

/// Exact layouts shipped in RustMagic 0.2.0; only these are eligible for
/// replacement by the authoritative KeyMagic files.
fn legacy_snapshot(id: &str) -> Option<&'static [u8]> {
    match id {
        "pyidaungsu-mm" => Some(include_bytes!("migrations/pyidaungsu-mm-0.2.0.km2")),
        "myansan" => Some(include_bytes!("migrations/myansan-0.2.0.km2")),
        _ => None,
    }
}

/// Restricts an id to lowercase letters, digits and dashes.
///
/// Ids become file names, so anything that could escape the library directory or
/// collide with another id is refused rather than rewritten silently.
pub fn safe_id(name: &str) -> Result<String> {
    let id: String = name
        .trim()
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = id.trim_matches('-').to_owned();
    if trimmed.is_empty() || trimmed.len() > 96 {
        return Err(Error::BadPath {
            path: name.to_owned(),
        });
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_library(name: &str) -> Library {
        let root = std::env::temp_dir().join(format!("rustmagic-library-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        Library::new(root)
    }

    #[test]
    fn installing_a_source_keeps_both_files() {
        let library = temp_library("source");
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let entry = library
            .install_source(fixtures.join("basic.kms"))
            .expect("installs");
        assert_eq!(entry.id, "basic");
        assert_eq!(entry.display_name(), "Fixture Basic");
        assert!(entry.source.is_some());
        assert_eq!(library.list().len(), 1);
        let loaded = library.load("basic").expect("loads");
        assert_eq!(loaded.rule_count(), 14);
        let source = library.source_of("basic").expect("source kept");
        assert!(source.contains("@NAME"));
        library.remove("basic").expect("removes");
        assert_eq!(library.list().len(), 0);
        let _ = std::fs::remove_dir_all(library.root());
    }

    #[test]
    fn installing_bytes_checks_the_file_first() {
        let library = temp_library("bytes");
        let fixture =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/minimal.km2"))
                .expect("fixture");
        assert!(library.install_bytes("Minimal", &fixture).is_ok());
        assert_eq!(library.list()[0].id, "minimal");
        assert!(library.install_bytes("Broken", b"not a keyboard").is_err());
        assert_eq!(library.list().len(), 1, "the broken file was not stored");
        let _ = std::fs::remove_dir_all(library.root());
    }

    #[test]
    fn bundled_seed_migrates_only_the_exact_broken_0_2_0_files() {
        let library = temp_library("migrate-bundled");
        let keyboards = library.keyboards();
        std::fs::create_dir_all(&keyboards).expect("keyboard directory");
        std::fs::write(
            keyboards.join("pyidaungsu-mm.km2"),
            include_bytes!("migrations/pyidaungsu-mm-0.2.0.km2"),
        )
        .expect("old Pyidaungsu snapshot");
        std::fs::write(
            keyboards.join("myansan.km2"),
            include_bytes!("migrations/myansan-0.2.0.km2"),
        )
        .expect("old MyanSan snapshot");
        let custom = b"a user's unrelated keyboard file";
        std::fs::write(keyboards.join("custom.km2"), custom).expect("custom file");

        let migrated = library.seed_bundled().expect("seed and migrate");
        assert_eq!(migrated.len(), 6, "four missing plus two migrated");
        for bundled in crate::BUNDLED {
            assert_eq!(
                std::fs::read(keyboards.join(format!("{}.km2", bundled.id))).unwrap(),
                bundled.bytes
            );
        }
        assert_eq!(
            std::fs::read(keyboards.join("custom.km2")).unwrap(),
            custom,
            "a non-matching user file is never overwritten"
        );
        let _ = std::fs::remove_dir_all(library.root());
    }

    #[test]
    fn ids_cannot_escape_the_library() {
        assert!(safe_id("../../etc/passwd").is_ok());
        assert_eq!(safe_id("../../etc/passwd").unwrap(), "etc-passwd");
        assert!(safe_id("../..").is_err());
        assert!(safe_id("   ").is_err());
        assert!(safe_id(&"a".repeat(120)).is_err());
    }

    #[test]
    fn an_empty_library_lists_nothing() {
        let library = temp_library("empty");
        assert_eq!(library.list().len(), 0);
        assert!(library.load("nothing").is_err());
        assert!(library.source_of("nothing").is_none());
    }
}
