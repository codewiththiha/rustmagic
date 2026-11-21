import Cocoa
import InputMethodKit

/// The connection name must be the one in Info.plist, because that is how the
/// system finds this server for a client.
private let connectionName = "RustMagic_1_Connection"

/// Command line modes.
///
/// An input method is a background process with no window, so everything the
/// manager window needs to do *to* the input method — install it, register it,
/// say whether it is registered, open the pane where it is added — is done by
/// this binary with an argument. One implementation, called by the window and by
/// the shell script alike.
///
/// Returns true when a command was run, so the caller can exit instead of
/// starting a server nobody asked for.
private func runCommandLine() -> Bool {
    let arguments = Array(CommandLine.arguments.dropFirst())
    guard let command = arguments.first, command.hasPrefix("--") else {
        return false
    }

    func say(_ text: String) {
        FileHandle.standardOutput.write(Data((text + "\n").utf8))
    }

    func fail(_ text: String) -> Never {
        FileHandle.standardError.write(Data((text + "\n").utf8))
        exit(1)
    }

    switch command {
    case "--version":
        say(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "?")
    case "--status":
        // Only what can be answered from here: whether the bundle is where macOS
        // looks for input methods. Whether the user has added it to Input
        // Sources is theirs to say, so nothing pretends to know.
        say(Setup.isInstalled() ? "installed" : "not installed")
    case "--instructions":
        say(Setup.instructions)
    case "--open-keyboard-settings":
        Setup.openKeyboardSettings()
    case "--install", "--setup":
        do {
            let installed = try Setup.install()
            say("installed \(installed.path)")
        } catch {
            fail(error.localizedDescription)
        }
        say("")
        say(Setup.instructions)
        if command == "--setup" {
            Setup.openKeyboardSettings()
        }
    default:
        return false
    }
    return true
}

if runCommandLine() {
    exit(0)
}

private let server = IMKServer(name: connectionName,
                               bundleIdentifier: Bundle.main.bundleIdentifier)

// Kept in scope for the lifetime of the process: when the server is released,
// input stops working with no error anywhere.
withExtendedLifetime(server) {
    let application = NSApplication.shared
    application.setActivationPolicy(.accessory)
    application.run()
}
