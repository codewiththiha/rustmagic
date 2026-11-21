import Foundation

/// Small string edits shared with the input controller.
///
enum UnicodeScalarEditing {
    /// Removes `count` Unicode scalar values from the end of `text`.
    ///
    /// The Rust engine reports edits in Unicode scalar values. Swift's `Character`
    /// is an extended grapheme cluster, which can contain several Myanmar scalars.
    static func removingLast(_ count: Int, from text: String) -> String {
        guard count > 0 else { return text }
        let scalars = text.unicodeScalars
        let remaining = max(0, scalars.count - count)
        return String(String.UnicodeScalarView(scalars.prefix(remaining)))
    }
}
