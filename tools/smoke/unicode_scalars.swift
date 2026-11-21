import Foundation

@main
enum UnicodeScalarSmoke {
    static func main() {
        let vowel = "\u{200C}\u{1031}"
        precondition(UnicodeScalarEditing.removingLast(1, from: vowel) == "\u{200C}")
        precondition(UnicodeScalarEditing.removingLast(2, from: vowel).isEmpty)
        precondition(UnicodeScalarEditing.removingLast(0, from: vowel) == vowel)
        print("Unicode-scalar edits preserve Myanmar composition boundaries")
    }
}
