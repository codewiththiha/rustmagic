import Cocoa

/// What has to happen outside the keyboard before RustMagic can type at all:
/// the bundle has to sit where macOS looks for input methods, the system has to
/// be told it is there, and the user has to be put in front of the one list
/// where it is added on.
///
/// Every step here is a public macOS mechanism. Nothing is faked, and nothing
/// claims to have worked: `install()` reports what it did, and the last step —
/// the user clicking + in Input Sources — is theirs, so the call says so.
enum Setup {
    /// `~/Library/Input Methods`, per user and without admin rights.
    static var inputMethodsDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Input Methods", isDirectory: true)
    }

    static var installedBundle: URL {
        inputMethodsDirectory.appendingPathComponent("RustMagicIME.app")
    }

    static func isInstalled() -> Bool {
        FileManager.default.fileExists(atPath: installedBundle.path)
    }

    enum Failure: LocalizedError {
        case notAnAppBundle(String)
        case copyFailed(String)
        case registerFailed(Int32)

        var errorDescription: String? {
            switch self {
            case let .notAnAppBundle(path):
                return "\(path) is not an .app bundle, so there is nothing to install"
            case let .copyFailed(reason):
                return "could not copy the input method into Input Methods: \(reason)"
            case let .registerFailed(status):
                return "TISRegisterInputSource refused the bundle (OSStatus \(status))"
            }
        }
    }

    /// Copies this bundle into `~/Library/Input Methods`, replacing an older
    /// copy, and registers the result with Text Input Source Services.
    ///
    /// Returns the installed location. When the running bundle is already the
    /// installed one, only the registration is repeated, which is what fixes a
    /// system that stopped listing it.
    @discardableResult
    static func install() throws -> URL {
        let source = Bundle.main.bundleURL
        guard source.pathExtension == "app" else {
            throw Failure.notAnAppBundle(source.path)
        }
        if source.standardizedFileURL.path == installedBundle.standardizedFileURL.path {
            try register(installedBundle)
            return installedBundle
        }
        do {
            try FileManager.default.createDirectory(at: inputMethodsDirectory,
                                                    withIntermediateDirectories: true)
            // Replace in one move: an input method that is half copied is an
            // input source the user cannot use and cannot see why.
            if FileManager.default.fileExists(atPath: installedBundle.path) {
                try FileManager.default.removeItem(at: installedBundle)
            }
            try FileManager.default.copyItem(at: source, to: installedBundle)
        } catch {
            throw Failure.copyFailed(error.localizedDescription)
        }
        try register(installedBundle)
        return installedBundle
    }

    /// `TISRegisterInputSource`. macOS otherwise notices a new bundle at the next
    /// login, which is why an input method copied during a session so often
    /// "installs but never appears".
    ///
    /// Called through dlsym rather than linked directly: HIToolbox exports it,
    /// but not every SDK declares it, and a symbol that is missing at link time
    /// would stop the whole bundle building.
    static func register(_ bundle: URL) throws {
        typealias Register = @convention(c) (CFURL) -> OSStatus
        guard let handle = dlopen("/System/Library/Frameworks/Carbon.framework/Carbon",
                                  RTLD_NOW),
              let symbol = dlsym(handle, "TISRegisterInputSource") else {
            // No framework, no registration call. The bundle is still in the
            // right folder, so the next login finds it; say nothing and let the
            // caller print the instructions it would have printed anyway.
            return
        }
        let call = unsafeBitCast(symbol, to: Register.self)
        let status = call(bundle as CFURL)
        if status != noErr {
            throw Failure.registerFailed(status)
        }
    }

    /// Opens System Settings with Keyboard selected. There is no API that adds an
    /// input source on the user's behalf — the list is the user's — so this is
    /// the closest honest thing: put them in front of the list.
    static func openKeyboardSettings() {
        let panes = [
            "x-apple.systempreferences:com.apple.Keyboard-Settings.extension",
            "x-apple.systempreferences:com.apple.preference.keyboard",
        ]
        for pane in panes {
            if let url = URL(string: pane), NSWorkspace.shared.open(url) {
                return
            }
        }
    }

    /// The steps, in order, for a panel or a terminal. Kept here so the app, the
    /// installer script and `--setup` all say the same thing.
    static var instructions: String {
        """
        RustMagic is installed as an input method. One step is left, and it is \
        the system's to allow:

          1. System Settings > Keyboard > Text Input > Input Sources
          2. press +, choose Burmese (or English), and pick RustMagic
          3. switch to it with Control+Space or the input menu

        If RustMagic is listed but will not turn on, log out and back in once: \
        macOS registers a copy made during a session immediately and only lets it \
        become the active input source at the next login.
        """
    }
}
