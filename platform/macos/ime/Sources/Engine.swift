import Foundation

/// The engine, through RustMagic's C ABI.
///
/// The rules are not here; this class finds the keyboard the RustMagic window
/// has active, hands keys to `km-ffi`, and turns the answer into "erase this
/// many characters, write this". Everything the engine decides is a rule;
/// everything here is plumbing.
final class Engine {
    /// What to do to the document after a key. `erase` counts characters, not
    /// UTF-16 units: one Myanmar character can be several of those.
    struct Edit {
        var erase: Int = 0
        var insert: String = ""
        var consumed: Bool = false
    }

    private var keyboard: OpaquePointer?
    private var engine: OpaquePointer?
    private(set) var keyboardID: String = ""
    private var loadedPath: String = ""

    var isReady: Bool { engine != nil }

    deinit {
        unload()
    }

    /// Where the manager window keeps its state: the same folder the Rust side
    /// uses on macOS.
    static var stateDirectory: URL {
        let home = FileManager.default.homeDirectoryForCurrentUser
        return home.appendingPathComponent("Library/Application Support/RustMagic")
    }

    /// The keyboard the window has active, else the first one in the library,
    /// else one shipped inside this bundle so a fresh install still types.
    static func activeKeyboardPath() -> URL? {
        let state = stateDirectory
        let keyboards = state.appendingPathComponent("library/keyboards")

        if let id = activeID(in: state.appendingPathComponent("settings.toml")) {
            let candidate = keyboards.appendingPathComponent("\(id).km2")
            if FileManager.default.fileExists(atPath: candidate.path) {
                return candidate
            }
            if let resources = Bundle.main.resourceURL {
                let bundled = resources.appendingPathComponent("keyboards/\(id).km2")
                if FileManager.default.fileExists(atPath: bundled.path) {
                    return bundled
                }
            }
        }
        if let first = firstKeyboard(in: keyboards) {
            return first
        }
        if let resources = Bundle.main.resourceURL,
           let first = firstKeyboard(in: resources.appendingPathComponent("keyboards")) {
            return first
        }
        return nil
    }

    /// Every keyboard this input method can offer: the ones the window manages,
    /// then the ones inside the bundle. The bundled layouts are always there, so a
    /// fresh install has something to type with before the window ever runs.
    static func installedKeyboards() -> [(id: String, url: URL)] {
        var found: [(id: String, url: URL)] = []
        var seen = Set<String>()
        var folders: [URL] = [stateDirectory.appendingPathComponent("library/keyboards")]
        if let resources = Bundle.main.resourceURL {
            folders.append(resources.appendingPathComponent("keyboards"))
        }
        for folder in folders {
            guard let names = try? FileManager.default
                .contentsOfDirectory(atPath: folder.path) else { continue }
            for name in names.sorted() where name.hasSuffix(".km2") {
                let id = String(name.dropLast(4))
                guard seen.insert(id).inserted else { continue }
                found.append((id, folder.appendingPathComponent(name)))
            }
        }
        return found.filter { isVisibleInInputMenu($0.id) }
    }

    /// Reads the GUI's input-menu visibility setting without adding a second
    /// settings store inside the InputMethodKit process.
    private static func isVisibleInInputMenu(_ id: String) -> Bool {
        let settings = stateDirectory.appendingPathComponent("settings.toml")
        guard let text = try? String(contentsOf: settings, encoding: .utf8) else {
            return true
        }
        let mode = text.split(separator: "\n").first(where: {
            String($0).trimmingCharacters(in: .whitespaces).hasPrefix("keyboard_menu_mode")
        })?.split(separator: "=", maxSplits: 1).last.map {
            String($0).trimmingCharacters(in: CharacterSet(charactersIn: " \""))
        } ?? "all"
        switch mode {
        case "active":
            return activeID(in: settings) == id
        case "selected":
            guard let line = text.split(separator: "\n").first(where: {
                String($0).trimmingCharacters(in: .whitespaces).hasPrefix("keyboard_menu_ids")
            }), let raw = line.split(separator: "=", maxSplits: 1).last else {
                return false
            }
            let ids = raw.split(separator: ",").map {
                String($0).trimmingCharacters(in: CharacterSet(charactersIn: " []\""))
            }
            return ids.contains(id)
        default:
            return true
        }
    }

    /// Records the choice where the window will read it, so switching in the
    /// input menu is not a second, private setting.
    static func setActive(id: String) {
        let state = stateDirectory
        let file = state.appendingPathComponent("settings.toml")
        try? FileManager.default.createDirectory(at: state, withIntermediateDirectories: true)
        var lines = (try? String(contentsOf: file, encoding: .utf8))?
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map(String.init) ?? []
        let entry = "active = \"\(id)\""
        if let index = lines.firstIndex(where: {
            $0.trimmingCharacters(in: .whitespaces).hasPrefix("active")
        }) {
            lines[index] = entry
        } else {
            lines.append(entry)
        }
        try? (lines.joined(separator: "\n") + "\n")
            .write(to: file, atomically: true, encoding: .utf8)
    }

    /// `active = "my-win"` in settings.toml. The window writes that file and
    /// this reads the one key it needs, so a hand-edited file cannot crash the
    /// input method.
    private static func activeID(in settings: URL) -> String? {
        guard let text = try? String(contentsOf: settings, encoding: .utf8) else {
            return nil
        }
        for line in text.split(separator: "\n", omittingEmptySubsequences: false) {
            /* Substring is not a String: trimming, which the settings file needs
             * because the window writes `active = "id"` with spaces, is a String
             * method. */
            let trimmed = String(line).trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix("active") else { continue }
            guard let equals = trimmed.firstIndex(of: "=") else { continue }
            let value = String(trimmed[trimmed.index(after: equals)...])
                .trimmingCharacters(in: .whitespaces)
            if value.hasPrefix("\""), value.hasSuffix("\""), value.count >= 2 {
                return String(value.dropFirst().dropLast())
            }
        }
        return nil
    }

    /// The first .km2 in a folder, sorted so the choice is stable across runs.
    private static func firstKeyboard(in folder: URL) -> URL? {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: folder.path) else {
            return nil
        }
        return names.filter { $0.hasSuffix(".km2") }.sorted().first.map {
            folder.appendingPathComponent($0)
        }
    }

    /// Loads whatever keyboard is active now, if that has changed since last
    /// time. Cheap enough to call on every focus change and commit, which is how
    /// switching in the window switches typing.
    @discardableResult
    func reload() -> Bool {
        guard let path = Engine.activeKeyboardPath() else {
            unload()
            return false
        }
        return load(path)
    }

    /// Loads one particular keyboard, whether the window chose it or the input
    /// menu did. Switching in the menu is a switch in the window too, because
    /// both read the same file.
    @discardableResult
    func load(_ path: URL) -> Bool {
        if path.path == loadedPath, engine != nil {
            return true
        }
        unload()

        var error: UnsafeMutablePointer<CChar>?
        guard let loaded = rm_keyboard_load(path.path, &error) else {
            if let error {
                rm_string_free(error)
            }
            return false
        }
        guard let created = rm_engine_new(loaded) else {
            rm_keyboard_free(loaded)
            return false
        }
        keyboard = loaded
        engine = created
        loadedPath = path.path
        keyboardID = path.deletingPathExtension().lastPathComponent
        return true
    }

    func reset() {
        if let engine {
            rm_engine_reset(engine)
        }
    }

    /// Feeds one key. `key` is a Windows virtual-key code, which is what the
    /// keyboards are written against; `character` is what the key produced on
    /// this layout, or nil when it produced nothing.
    ///
    /// macOS has no AltGr, so Option takes that part: a rule written for
    /// `VK_ALT_GR` matches Option+key here, which is how the layouts reach the
    /// characters the plain keyboard does not have.
    func process(key: UInt16, character: Unicode.Scalar?, shift: Bool, control: Bool,
                 option: Bool, caps: Bool) -> Edit {
        guard let engine else {
            return Edit()
        }
        var output = rm_edit()
        let consumed = rm_engine_key(engine, key, character?.value ?? 0,
                                     shift ? 1 : 0, control ? 1 : 0,
                                     0, option ? 1 : 0, caps ? 1 : 0, &output)
        var edit = Edit()
        edit.consumed = consumed != 0
        edit.erase = Int(output.erase)
        if let insert = output.insert {
            edit.insert = String(cString: insert)
            rm_string_free(insert)
        }
        return edit
    }

    private func unload() {
        if let engine {
            rm_engine_free(engine)
            self.engine = nil
        }
        if let keyboard {
            rm_keyboard_free(keyboard)
            self.keyboard = nil
        }
        loadedPath = ""
        keyboardID = ""
    }
}
