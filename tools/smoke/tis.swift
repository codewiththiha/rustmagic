// tis.swift — proves macOS accepts RustMagic as an input method.
//
//   swiftc -O -framework Carbon tools/smoke/tis.swift -o build/tis-smoke
//   build/tis-smoke
//
// The bundle has to be in ~/Library/Input Methods first (build.sh says how). The
// test then asks the Text Input Source Services, the same API System Settings
// reads, whether the input method is there, whether it can be enabled, and
// whether it can be selected — and it puts the previous input source back
// afterwards, so running it on a real desktop does not leave the user typing
// Burmese in their terminal.
//
// What it cannot do is type a character: that needs a focused text field and a
// keyboard. INSTALL.txt says so rather than implying otherwise.

import Carbon
import Foundation

let expectedBundle = "com.rustmagic.inputmethod.RustMagic"
let expectedConnection = "RustMagic_1_Connection"

func text(_ source: TISInputSource, _ key: CFString) -> String? {
    guard let raw = TISGetInputSourceProperty(source, key) else { return nil }
    let value = Unmanaged<CFString>.fromOpaque(raw).takeUnretainedValue()
    return value as String
}

func flag(_ source: TISInputSource, _ key: CFString) -> Bool {
    guard let raw = TISGetInputSourceProperty(source, key) else { return false }
    let value = Unmanaged<CFBoolean>.fromOpaque(raw).takeUnretainedValue()
    return CFBooleanGetValue(value)
}

func installed() -> [TISInputSource] {
    guard let list = TISCreateInputSourceList(nil, true)?.takeRetainedValue() else { return [] }
    return list as! [TISInputSource]
}

func findRustMagic() -> TISInputSource? {
    for source in installed() where text(source, kTISPropertyBundleID) == expectedBundle {
        return source
    }
    return nil
}

var failures = 0
func pass(_ what: String) { print("PASS  \(what)") }
func fail(_ what: String) {
    print("FAIL  \(what)")
    failures += 1
}

// macOS scans ~/Library/Input Methods lazily, and the agent that does it may
// need a nudge, so this waits rather than reporting a failure after one look.
var source: TISInputSource?
for attempt in 0..<30 {
    source = findRustMagic()
    if source != nil { break }
    if attempt == 2 || attempt == 10 {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/killall")
        task.arguments = ["-HUP", "TextInputMenuAgent"]
        try? task.run()
        task.waitUntilExit()
    }
    Thread.sleep(forTimeInterval: 1)
}

guard let rustmagic = source else {
    fail("the input method is listed by Text Input Source Services")
    print("the bundle has to be in ~/Library/Input Methods; build.sh puts it in build/macos")
    exit(1)
}
pass("the input method is listed by Text Input Source Services")

if let identifier = text(rustmagic, kTISPropertyInputSourceID) {
    print("      input source id: \(identifier)")
}
let keyboardCategory: String = kTISCategoryKeyboardInputSource as String
let inputMethodType: String = kTISTypeKeyboardInputMethodModeEnabled as String

if text(rustmagic, kTISPropertyInputSourceCategory) == keyboardCategory {
    pass("it is in the keyboard input source category")
} else {
    fail("it is in the keyboard input source category")
}
if text(rustmagic, kTISPropertyInputSourceType) == inputMethodType {
    pass("it is a keyboard input method")
} else {
    fail("it is a keyboard input method")
}
if let languages = TISGetInputSourceProperty(rustmagic, kTISPropertyInputSourceLanguages) {
    let list = Unmanaged<CFArray>.fromOpaque(languages).takeUnretainedValue() as? [String] ?? []
    if list.contains("my") {
        pass("it declares Burmese as its language")
    } else {
        fail("it declares Burmese as its language (got \(list))")
    }
} else {
    fail("it declares a language")
}

// The previous source is remembered so the desktop is left as it was found.
var previous: TISInputSource?
for candidate in installed()
where text(candidate, kTISPropertyInputSourceCategory) == keyboardCategory
    && flag(candidate, kTISPropertyInputSourceIsSelected)
{
    previous = candidate
}

let enabled = TISEnableInputSource(rustmagic)
if enabled == noErr {
    pass("it can be enabled")
} else {
    fail("it can be enabled (OSStatus \(enabled))")
}
if flag(rustmagic, kTISPropertyInputSourceIsEnabled) {
    pass("it is enabled")
} else {
    fail("it is enabled")
}

let selected = TISSelectInputSource(rustmagic)
if selected == noErr {
    pass("it can be selected")
} else {
    fail("it can be selected (OSStatus \(selected))")
}
if flag(rustmagic, kTISPropertyInputSourceIsSelected) {
    pass("it is the selected input source")
} else {
    fail("it is the selected input source")
}

if let previous {
    TISSelectInputSource(previous)
    pass("the previous input source was put back")
}

if failures == 0 {
    print("macOS accepts RustMagic as an input method")
    exit(0)
}
print("\(failures) check(s) failed")
exit(1)
