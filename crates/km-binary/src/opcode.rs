//! Opcodes used inside a rule's left and right hand sides.

/// A literal string follows: a `u16` length and that many UTF-16 units.
pub const OP_STRING: u16 = 0x00F0;
/// A variable reference: a 1-based index into the string table.
pub const OP_VARIABLE: u16 = 0x00F1;
/// A capture reference (`$1`): a 1-based index into the left side's segments.
pub const OP_REFERENCE: u16 = 0x00F2;
/// A predefined key: a 1-based index into the predefined table.
pub const OP_PREDEFINED: u16 = 0x00F3;
/// Modifies the preceding string: `OP_ANYOF`, `OP_NANYOF` or a capture index.
pub const OP_MODIFIER: u16 = 0x00F4;
/// Match one character from the preceding string.
pub const OP_ANYOF: u16 = 0x00F5;
/// Virtual-key states follow, one `OP_PREDEFINED` each.
pub const OP_AND: u16 = 0x00F6;
/// Match one character that is not in the preceding string.
pub const OP_NANYOF: u16 = 0x00F7;
/// Match any one character the engine considers matchable.
pub const OP_ANY: u16 = 0x00F8;
/// A switch (state) reference: a 1-based index into the string table.
pub const OP_SWITCH: u16 = 0x00F9;
