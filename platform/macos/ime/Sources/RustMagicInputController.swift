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

    /// "Commit each keystroke": the syllable goes to the application after every
    /// key instead of being marked. Some applications compose better that way,
    /// and every input method of this kind offers the switch.
    private var instantCommit: Bool {
        get { UserDefaults.standard.bool(forKey: "RustMagicInstantCommit") }
        set { UserDefaults.standard.set(newValue, forKey: "RustMagicInstantCommit") }
    }

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
        if instantCommit {
            commit(client: client)
        }
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

    /// The engine counts Unicode scalar values, not Swift grapheme clusters.
    /// Myanmar vowel marks and joiners can be grouped into one `Character`, so
    /// `String.removeLast()` would erase more than the edit asked for.
    private func dropLastCharacters(_ count: Int) {
        composing = UnicodeScalarEditing.removingLast(count, from: composing)
    }

    // MARK: - The menu under the input menu

    /// macOS asks for this when the input menu is opened with RustMagic
    /// selected, and lists what comes back under the RustMagic entry. It is the
    /// same place every input method keeps its own switches, and the only place
    /// an input method can offer one: this process has no window of its own.
    override func menu() -> NSMenu! {
        let menu = NSMenu(title: "RustMagic")
        // Every item says for itself whether it can be used; the automatic pass
        // cannot know that a menu item's target is this controller.
        menu.autoenablesItems = false

        let keyboards = Engine.installedKeyboards()
        if keyboards.isEmpty {
            let none = NSMenuItem(title: "No keyboards found", action: nil, keyEquivalent: "")
            none.isEnabled = false
            menu.addItem(none)
        }
        for keyboard in keyboards {
            let item = NSMenuItem(title: Self.name(for: keyboard.id),
                                  action: #selector(chooseKeyboard(_:)),
                                  keyEquivalent: "")
            item.target = self
            item.representedObject = keyboard.url
            item.state = keyboard.id == engine.keyboardID ? .on : .off
            item.isEnabled = true
            menu.addItem(item)
        }

        menu.addItem(.separator())

        let instant = NSMenuItem(title: "Commit each keystroke",
                                 action: #selector(toggleInstantCommit(_:)),
                                 keyEquivalent: "")
        instant.target = self
        instant.state = instantCommit ? .on : .off
        instant.isEnabled = true
        menu.addItem(instant)

        menu.addItem(Self.item("Open keyboards folder",
                               #selector(openKeyboardsFolder(_:)), self))

        menu.addItem(.separator())

        menu.addItem(Self.item("Set up RustMagic…", #selector(openSetup(_:)), self))
        menu.addItem(Self.item("Keyboard settings…",
                               #selector(openKeyboardSettings(_:)), self))
        menu.addItem(Self.item("About RustMagic", #selector(showAbout(_:)), self))
        return menu
    }

    private static func item(_ title: String, _ action: Selector, _ target: AnyObject) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = target
        item.isEnabled = true
        return item
    }

    /// A menu click can arrive as the item or, through IMK's forwarding, as a
    /// dictionary holding it. Both are handled, because both happen.
    private func menuItem(from sender: Any?) -> NSMenuItem? {
        if let item = sender as? NSMenuItem {
            return item
        }
        if let dictionary = sender as? NSDictionary,
           let item = dictionary["IMKCommandMenuItem"] as? NSMenuItem {
            return item
        }
        return nil
    }

    /// The five keyboards that ship, named the way they are named elsewhere, so
    /// the menu does not read `pyidaungsu-mm` where the window reads
    /// "Pyidaungsu MM".
    private static func name(for id: String) -> String {
        switch id {
        case "pyidaungsu-mm": return "Pyidaungsu MM"
        case "my-win": return "my-Win 2.3.0"
        case "myansan": return "MyanSan"
        case "zawgyi": return "Zawgyi"
        case "zawcode": return "ZawCode"
        default: return id
        }
    }

    @objc private func chooseKeyboard(_ sender: Any?) {
        guard let item = menuItem(from: sender),
              let url = item.representedObject as? URL else { return }
        // Anything being composed belongs to the keyboard it was typed on.
        if let client = client() as? IMKTextInput {
            commit(client: client)
        }
        engine.load(url)
        Engine.setActive(id: url.deletingPathExtension().lastPathComponent)
    }

    @objc private func toggleInstantCommit(_ sender: Any?) {
        instantCommit.toggle()
    }

    @objc private func openKeyboardsFolder(_ sender: Any?) {
        let folder = Engine.stateDirectory.appendingPathComponent("keyboards")
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(folder)
    }

    /// Installs and registers this bundle, then puts the user in front of the one
    /// list where it is added on. The clicking is theirs to do: macOS has no API
    /// that adds an input source for them.
    @objc private func openSetup(_ sender: Any?) {
        var message: String
        do {
            let installed = try Setup.install()
            message = "Installed to \(installed.path).\n\n" + Setup.instructions
        } catch {
            message = error.localizedDescription + "\n\n" + Setup.instructions
        }
        Setup.openKeyboardSettings()
        let alert = NSAlert()
        alert.messageText = "Set up RustMagic"
        alert.informativeText = message
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    @objc private func openKeyboardSettings(_ sender: Any?) {
        Setup.openKeyboardSettings()
    }

    @objc private func showAbout(_ sender: Any?) {
        let version = Bundle.main
            .object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "?"
        let keyboard = engine.keyboardID.isEmpty ? "none" : Self.name(for: engine.keyboardID)
        let alert = NSAlert()
        alert.messageText = "RustMagic \(version)"
        alert.informativeText = """
        A keyboard layout engine: it reads KeyMagic .km2 keyboards and types with         whichever one is active.

        Active keyboard: \(keyboard)
        Keyboards folder: \(Engine.stateDirectory.appendingPathComponent("keyboards").path)
        """
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

}
