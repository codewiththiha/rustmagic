import Cocoa
import InputMethodKit

/// The input method's entry point.
///
/// An input method is a background app: macOS launches it when the input source
/// is selected and it exists to answer key events, not to show a window. The
/// connection name here must be the one in Info.plist, because that is how the
/// system finds this server for a client.
private let connectionName = "RustMagic_1_Connection"

private let server = IMKServer(name: connectionName,
                               bundleIdentifier: Bundle.main.bundleIdentifier)

// Kept in scope for the lifetime of the process: when the server is released,
// input stops working with no error anywhere.
withExtendedLifetime(server) {
    let application = NSApplication.shared
    application.setActivationPolicy(.accessory)
    application.run()
}
