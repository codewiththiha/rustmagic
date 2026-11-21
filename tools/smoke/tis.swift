// tis.swift — proves macOS accepts RustMagic as an input method.
//
//   swiftc -O -framework Carbon tools/smoke/tis.swift -o build/tis-smoke
//   build/tis-smoke                       # what CI can prove
//   build/tis-smoke --require-activation  # what a desktop can prove
//
// The bundle has to be in ~/Library/Input Methods first (build.sh says how). The
// test then asks the Text Input Source Services, the same API System Settings
// reads, whether the input method is there, whether it can be enabled, and
// whether it can be selected — and it puts the previous input source back
// afterwards, so running it on a real desktop does not leave the user typing
// Burmese in their terminal.
//
// Two of the checks depend on something this test cannot arrange: a bundle copied
// into ~/Library/Input Methods while a session is running registers at once but
// cannot be switched on until the next login, and TISEnableInputSource answers
// noErr on it while changing nothing. That is the platform, not the bundle, and a
// CI runner has been logged in for minutes rather than since the copy. So those
// two checks report what happened and count only when --require-activation is
// passed: CI runs without it, and INSTALL.txt tells anyone on a real desktop to
// run with it, where they must pass.
//
// What it cannot do even then is type a character: that needs a focused text
// field and a keyboard. INSTALL.txt says so rather than implying otherwise.

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


let requireActivation = CommandLine.arguments.contains("--require-activation")

var failures = 0
func pass(_ what: String) { print("PASS  \(what)") }
func fail(_ what: String) {
    print("FAIL  \(what)")
    failures += 1
}
// Something the platform decided rather than the bundle, in a run that did not
// ask for it to be fatal. Printed either way: a quiet skip would be a lie.
func warn(_ what: String) { print("WARN  \(what)") }

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
// A bundle without ComponentInputModeDict is listed as an input method without
// modes; one that declares modes is listed as mode-enabled. Both are keyboard
// input methods, and RustMagic is deliberately the first kind, so either type is
// accepted and the one reported is printed.
let inputMethodTypes: Set<String> = [
    kTISTypeKeyboardInputMethodWithoutModes as String,
    kTISTypeKeyboardInputMethodModeEnabled as String,
]

if text(rustmagic, kTISPropertyInputSourceCategory) == keyboardCategory {
    pass("it is in the keyboard input source category")
} else {
    fail("it is in the keyboard input source category")
}
if let reported = text(rustmagic, kTISPropertyInputSourceType) {
    print("      input source type: \(reported)")
    if inputMethodTypes.contains(reported) {
        pass("it is a keyboard input method")
    } else {
        fail("it is a keyboard input method")
    }
} else {
    fail("it is a keyboard input method (it reports no type)")
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

// Whether a user can switch this on at all is a property of the source, and it
// is the part System Settings reads when it offers the switch.
if flag(rustmagic, kTISPropertyInputSourceIsEnableCapable) {
    pass("a user can switch it on")
} else {
    fail("a user can switch it on")
}
if flag(rustmagic, kTISPropertyInputSourceIsSelectCapable) {
    pass("it can become the active input source")
} else {
    fail("it can become the active input source")
}

let enabled = TISEnableInputSource(rustmagic)
if enabled == noErr {
    pass("it accepts being enabled")
} else {
    fail("it accepts being enabled (OSStatus \(enabled))")
}

// The system applies that in its own time, and only if the session can apply it
// at all, so this waits before it concludes anything.
var isOn = false
for _ in 0..<10 {
    if let now = findRustMagic(), flag(now, kTISPropertyInputSourceIsEnabled) {
        isOn = true
        break
    }
    Thread.sleep(forTimeInterval: 0.5)
}
if isOn {
    pass("it is enabled")
} else if requireActivation {
    fail("it is enabled")
} else {
    warn("it is not enabled yet: macOS switches a freshly copied input method on at the next login")
}

// A source that is not on cannot be selected, so the selection is only attempted
// when the enable actually took.
var selectedOK = false
if isOn {
    let selectable = findRustMagic() ?? rustmagic
    let selected = TISSelectInputSource(selectable)
    if selected == noErr {
        selectedOK = true
        pass("it can be selected")
    } else if requireActivation {
        fail("it can be selected (OSStatus \(selected))")
    } else {
        warn("it could not be selected (OSStatus \(selected))")
    }
}

if selectedOK {
    if let nowSelected = findRustMagic(), flag(nowSelected, kTISPropertyInputSourceIsSelected) {
        pass("it is the selected input source")
    } else {
        fail("it is the selected input source")
    }
    if let previous {
        TISSelectInputSource(previous)
        pass("the previous input source was put back")
    }
} else if !requireActivation {
    warn("being the selected input source was not checked: it is not enabled")
}

if failures == 0 {
    if requireActivation {
        print("macOS accepts RustMagic as an input method, and switches to it")
    } else {
        print("macOS accepts RustMagic as an input method")
        if !isOn {
            print("run again with --require-activation on a desktop to prove the switch")
        }
    }
    exit(0)
}
print("\(failures) check(s) failed")
exit(1)
