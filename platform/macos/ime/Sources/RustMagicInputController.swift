import Cocoa
import InputMethodKit

/// The input method itself: what macOS talks to when RustMagic is the selected
/// input source.
///
/// Every key press on this input source arrives in `handle(_:client:)`. The
/// answer is either "not mine" — the event goes on to the application untouched
/// — or a change to the marked text, which is the text the application draws
/// underlined because it is not committed yet. When a syllable ends, the marked
/// text is replaced by ordinary text and the application owns it.
@objc(RustMagicInputController)
final class RustMagicInputController: IMKInputController {
    private let engine = Engine()

    /// The text of the syllable being typed, which is exactly what the client
    /// is showing as marked text.
    private var composing: String = ""

    /// The range that means "the text you are already marking".
    private static let markedRange = NSRange(location: NSNotFound, length: 0)

    // MARK: - Life cycle

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        engine.reload()
    }

    override func deactivateServer(_ sender: Any!) {
        // Committing rather than discarding: the user pressed a key that changed
        // the input source, they did not ask to throw their word away.
        commit(client: sender as? IMKTextInput)
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        commit(client: sender as? IMKTextInput)
    }

    override func inputControllerWillClose() {
        engine.reset()
        composing = ""
        super.inputControllerWillClose()
    }

    // MARK: - Keys

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else {
            return false
        }
        guard let client = sender as? IMKTextInput else {
            return false
        }
        if !engine.isReady {
            engine.reload()
        }
        guard engine.isReady else {
            return false
        }

        let flags = event.modifierFlags
        let shift = flags.contains(.shift)
        let caps = flags.contains(.capsLock)
        let control = flags.contains(.control)
        let option = flags.contains(.option)
        let command = flags.contains(.command)
        let key = Keycodes.virtualKey(for: event.keyCode)

        // Escape throws away what is composing, the way every input method does.
        if key == 0x1B {
            guard !composing.isEmpty else { return false }
            cancel(client: client)
            return true
        }

        // Enter, tab and space end the syllable and belong to the application.
        if Keycodes.endsSyllable(key) {
            if !composing.isEmpty {
                commit(client: client)
            }
            return false
        }

        // Shortcuts are not text, and neither is navigation. Commit and let the
        // application see the key.
        if command || control || Keycodes.isNavigation(key) {
            if !composing.isEmpty {
                commit(client: client)
            }
            return false
        }

        // Backspace: the engine may want it (a Myanmar syllable deletes by
        // rule); when it does not, it deletes one character of the composition.
        // With nothing composing it belongs to the application.
        if key == 0x08 {
            guard !composing.isEmpty else { return false }
            let edit = engine.process(key: key, character: nil, shift: shift, control: false,
                                      option: false, caps: caps)
            if edit.consumed {
                apply(edit, client: client)
            } else {
                dropLastCharacter()
                show(client: client)
            }
            return true
        }

        let character = event.charactersIgnoringModifiers?.unicodeScalars.first
        let edit = engine.process(key: key, character: character, shift: shift, control: control,
                                  option: option, caps: caps)
        guard edit.consumed else {
            // No rule for this key, so it is ordinary text: the syllable is over
            // and the key goes through untouched.
            if !composing.isEmpty {
                commit(client: client)
            }
            return false
        }
        apply(edit, client: client)
        return true
    }

    // MARK: - Marked text

    /// Applies one engine edit to the composition and shows the result.
    private func apply(_ edit: Engine.Edit, client: IMKTextInput) {
        dropLastCharacters(edit.erase)
        composing += edit.insert
        if composing.isEmpty {
            // An edit that erased everything and inserted nothing: there is
            // nothing left to mark, so the client is told the composition is
            // over rather than being left with an empty underline.
            client.insertText("", replacementRange: Self.markedRange)
            engine.reset()
            return
        }
        show(client: client)
    }

    private func show(client: IMKTextInput) {
        let caret = NSRange(location: (composing as NSString).length, length: 0)
        client.setMarkedText(composing, selectionRange: caret, replacementRange: Self.markedRange)
    }

    /// Turns the marked text into ordinary text. Nothing else changes: this is
    /// the moment the application owns what the user typed.
    private func commit(client: IMKTextInput?) {
        guard let client, !composing.isEmpty else {
            composing = ""
            return
        }
        let text = composing
        composing = ""
        client.insertText(text, replacementRange: Self.markedRange)
        engine.reset()
    }

    /// Drops the composition without inserting it, which is what Escape means.
    private func cancel(client: IMKTextInput) {
        composing = ""
        client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0),
                             replacementRange: Self.markedRange)
        engine.reset()
    }

    private func dropLastCharacter() {
        dropLastCharacters(1)
    }

    /// Removes `count` characters, counting a composed character (which can be
    /// several UTF-16 units) as one.
    private func dropLastCharacters(_ count: Int) {
        guard count > 0 else { return }
        for _ in 0..<count {
            guard !composing.isEmpty else { return }
            composing.removeLast()
        }
    }
}
