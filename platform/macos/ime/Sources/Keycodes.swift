/// macOS hardware key codes as the virtual-key codes the keyboards are written
/// against.
///
/// A keyboard layout written for KeyMagic names keys the Windows way — 0x41 is
/// A, 0x0D is Return, 0xBE is the period key — because that is the numbering the
/// rules use. A mac reports a hardware position instead, and the two numbering
/// schemes disagree, so every event is translated here before the engine sees
/// it.
///
/// The left column is Apple's `kVK_*` value (Events.h); the right column is the
/// Windows `VK_*` value from `WinUser.h`. No behaviour is invented between them:
/// this is the same table every cross-platform input method carries.
enum Keycodes {
    static func virtualKey(for hardware: UInt16) -> UInt16 {
        switch hardware {
        // Letters
        case 0x00: return 0x41  // A
        case 0x01: return 0x53  // S
        case 0x02: return 0x44  // D
        case 0x03: return 0x46  // F
        case 0x04: return 0x48  // H
        case 0x05: return 0x47  // G
        case 0x06: return 0x5A  // Z
        case 0x07: return 0x58  // X
        case 0x08: return 0x43  // C
        case 0x09: return 0x56  // V
        case 0x0A: return 0     // ISO section key, no Windows equivalent
        case 0x0B: return 0x42  // B
        case 0x0C: return 0x51  // Q
        case 0x0D: return 0x57  // W
        case 0x0E: return 0x45  // E
        case 0x0F: return 0x52  // R
        case 0x10: return 0x59  // Y
        case 0x11: return 0x54  // T
        case 0x12: return 0x31  // 1
        case 0x13: return 0x32  // 2
        case 0x14: return 0x33  // 3
        case 0x15: return 0x34  // 4
        case 0x16: return 0x36  // 6
        case 0x17: return 0x35  // 5
        case 0x18: return 0xBB  // = +
        case 0x19: return 0x39  // 9
        case 0x1A: return 0x37  // 7
        case 0x1B: return 0xBD  // - _
        case 0x1C: return 0x38  // 8
        case 0x1D: return 0x30  // 0
        case 0x1E: return 0xDD  // ] }
        case 0x1F: return 0x4F  // O
        case 0x20: return 0x55  // U
        case 0x21: return 0xDB  // [ {
        case 0x22: return 0x49  // I
        case 0x23: return 0x50  // P
        case 0x24: return 0x0D  // Return
        case 0x25: return 0x4C  // L
        case 0x26: return 0x4A  // J
        case 0x27: return 0xDE  // ' "
        case 0x28: return 0x4B  // K
        case 0x29: return 0xBA  // ; :
        case 0x2A: return 0xDC  // \ |
        case 0x2B: return 0xBC  // , <
        case 0x2C: return 0xBF  // / ?
        case 0x2D: return 0x4E  // N
        case 0x2E: return 0x4D  // M
        case 0x2F: return 0xBE  // . >
        case 0x30: return 0x09  // Tab
        case 0x31: return 0x20  // Space
        case 0x32: return 0xC0  // ` ~
        case 0x33: return 0x08  // Delete (backspace)
        case 0x35: return 0x1B  // Escape
        case 0x37, 0x36: return 0x5B  // Command -> VK_LWIN
        case 0x38, 0x3C: return 0x10  // Shift
        case 0x39: return 0x14  // Caps Lock
        case 0x3A, 0x3D: return 0x12  // Option -> VK_MENU
        case 0x3B, 0x3E: return 0x11  // Control
        case 0x41: return 0x6E  // Keypad .
        case 0x43: return 0x6A  // Keypad *
        case 0x45: return 0x6B  // Keypad +
        case 0x47: return 0x90  // Num Lock
        case 0x4B: return 0x6F  // Keypad /
        case 0x4C: return 0x0D  // Keypad Enter
        case 0x4E: return 0x6D  // Keypad -
        case 0x51: return 0xBB  // Keypad =
        // Keypad digits, in the order the hardware numbers them
        case 0x52: return 0x60  // Keypad 0
        case 0x53: return 0x61  // Keypad 1
        case 0x54: return 0x62  // Keypad 2
        case 0x55: return 0x63  // Keypad 3
        case 0x56: return 0x64  // Keypad 4
        case 0x57: return 0x65  // Keypad 5
        case 0x58: return 0x66  // Keypad 6
        case 0x59: return 0x67  // Keypad 7
        case 0x5B: return 0x68  // Keypad 8
        case 0x5C: return 0x69  // Keypad 9
        case 0x60: return 0x74  // F5
        case 0x61: return 0x75  // F6
        case 0x62: return 0x76  // F7
        case 0x63: return 0x77  // F3
        case 0x64: return 0x78  // F8
        case 0x65: return 0x79  // F9
        case 0x67: return 0x7A  // F11
        case 0x69: return 0x7C  // F13
        case 0x6B: return 0x7D  // F14
        case 0x6D: return 0x7E  // F10
        case 0x6F: return 0x7F  // F12
        case 0x71: return 0x80  // F15
        case 0x73: return 0x24  // Home
        case 0x74: return 0x21  // Page Up
        case 0x75: return 0x2E  // Forward Delete
        case 0x76: return 0x7B  // F16
        case 0x77: return 0x23  // End
        case 0x78: return 0x7C  // F17
        case 0x79: return 0x22  // Page Down
        case 0x7A: return 0x70  // F1
        case 0x7B: return 0x25  // Left
        case 0x7C: return 0x27  // Right
        case 0x7D: return 0x28  // Down
        case 0x7E: return 0x26  // Up
        default: return 0
        }
    }

    /// Keys that end a syllable and belong to the application rather than to the
    /// layout: what the user means by them is "I am finished with this word".
    static func endsSyllable(_ key: UInt16) -> Bool {
        switch key {
        case 0x20, 0x0D, 0x09: return true
        default: return false
        }
    }

    /// Keys that are navigation or editing rather than text.
    static func isNavigation(_ key: UInt16) -> Bool {
        switch key {
        case 0x25, 0x26, 0x27, 0x28, 0x24, 0x23, 0x21, 0x22, 0x2E, 0x2D: return true
        default: return false
        }
    }
}
