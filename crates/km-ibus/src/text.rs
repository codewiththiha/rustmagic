//! IBus's value types, built the way libibus builds them.
//!
//! IBus does not put a string on the wire. It puts a variant holding a tuple of
//! `(type name, attachments, text, attributes)`, where the attributes are
//! themselves a variant of a similar tuple. The bytes come from
//! `ibus_serializable_serialize`; this module reproduces them exactly, so a real
//! panel deserialises the same struct the C library would have sent.

use std::collections::HashMap;

use zbus::zvariant::{Structure, StructureBuilder, Value};

/// The text an engine commits, as a `v`-typed signal argument.
pub fn text(value: &str) -> Value<'static> {
    let attributes = StructureBuilder::new()
        .add_field("IBusAttrList")
        .add_field(HashMap::<String, Value<'static>>::new())
        .add_field(Vec::<Value<'static>>::new())
        .build()
        .unwrap_or_else(|_| Structure::from(("IBusAttrList",)));
    let structure = StructureBuilder::new()
        .add_field("IBusText")
        .add_field(HashMap::<String, Value<'static>>::new())
        .add_field(value.to_owned())
        .add_field(Value::from(attributes))
        .build()
        .unwrap_or_else(|_| Structure::from(("IBusText",)));
    Value::from(structure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::{serialized::Context, to_bytes};

    /// The signature libibus gives an `IBusText` when it serialises one.
    const TEXT_SIGNATURE: &str = "(sa{sv}sv)";

    #[test]
    fn the_payload_has_the_signature_libibus_expects() {
        let value = text("αβ");
        assert_eq!(value.value_signature().to_string(), TEXT_SIGNATURE);
    }

    #[test]
    fn the_payload_carries_the_text_in_its_bytes() {
        let value = text("αβ");
        let inner = value.downcast::<Structure>().expect("the value is a tuple");
        let encoded =
            to_bytes(Context::new_dbus(zbus::zvariant::LE, 0), &inner).expect("the value encodes");
        // The type name comes first, the text after it, and the attribute list
        // after that. Both strings are somewhere in the body.
        let raw = encoded.bytes();
        for needle in ["IBusText".as_bytes(), "αβ".as_bytes()] {
            assert!(
                raw.windows(needle.len()).any(|window| window == needle),
                "the payload contains {:?}",
                String::from_utf8_lossy(needle)
            );
        }
        assert_eq!(text("αβ").value_signature().to_string(), TEXT_SIGNATURE);
    }

    #[test]
    fn an_empty_string_is_still_a_text() {
        let value = text("");
        assert_eq!(value.value_signature().to_string(), TEXT_SIGNATURE);
    }
}
