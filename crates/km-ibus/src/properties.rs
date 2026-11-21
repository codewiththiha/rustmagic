//! The IBus panel's keyboard chooser, serialized as IBus properties.

use std::collections::HashMap;

use zbus::zvariant::{Structure, StructureBuilder, Value};

use crate::text;

const PROP_TYPE_RADIO: u32 = 2;
const PROP_TYPE_MENU: u32 = 3;
const PROP_STATE_UNCHECKED: u32 = 0;
const PROP_STATE_CHECKED: u32 = 1;

/// A keyboard the IBus panel can activate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// The library id, used by `property_activate`.
    pub id: String,
    /// The name shown in the panel.
    pub name: String,
}

/// Builds the IBus `IBusPropList` containing one keyboard submenu.
pub fn keyboard_menu(choices: &[Choice], active: Option<&str>) -> Value<'static> {
    let children: Vec<Value<'static>> = choices
        .iter()
        .map(|choice| {
            property(
                &format!("keyboard.{}", choice.id),
                PROP_TYPE_RADIO,
                &choice.name,
                if active == Some(choice.id.as_str()) {
                    PROP_STATE_CHECKED
                } else {
                    PROP_STATE_UNCHECKED
                },
                empty_list(),
            )
        })
        .collect();
    let submenu = Value::from(prop_list(children));
    let root = property("keyboard", PROP_TYPE_MENU, "Keyboard", 0, submenu);
    Value::from(prop_list(vec![root]))
}

fn property(
    key: &str,
    kind: u32,
    label: &str,
    state: u32,
    sub_props: Value<'static>,
) -> Value<'static> {
    let structure = StructureBuilder::new()
        .add_field("IBusProperty")
        .add_field(HashMap::<String, Value<'static>>::new())
        .add_field(key.to_owned())
        .add_field(kind)
        .add_field(text::text(label))
        .add_field(String::new())
        .add_field(text::text(""))
        .add_field(true)
        .add_field(true)
        .add_field(state)
        .add_field(sub_props)
        .add_field(text::text(""))
        .build()
        .unwrap_or_else(|_| Structure::from(("IBusProperty",)));
    Value::from(structure)
}

fn empty_list() -> Value<'static> {
    Value::from(prop_list(Vec::new()))
}

fn prop_list(properties: Vec<Value<'static>>) -> Structure<'static> {
    StructureBuilder::new()
        .add_field("IBusPropList")
        .add_field(HashMap::<String, Value<'static>>::new())
        .add_field(properties)
        .build()
        .unwrap_or_else(|_| Structure::from(("IBusPropList",)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooser_is_an_ibus_property_list_with_a_radio_entry_for_each_keyboard() {
        let value = keyboard_menu(
            &[
                Choice {
                    id: "pyidaungsu-mm".into(),
                    name: "Pyidaungsu MM".into(),
                },
                Choice {
                    id: "myanmar3".into(),
                    name: "Myanmar3".into(),
                },
            ],
            Some("myanmar3"),
        );
        assert_eq!(value.value_signature().to_string(), "(sa{sv}av)");
        let inner = value
            .downcast::<Structure>()
            .expect("the value wraps a structure");
        let encoded = zbus::zvariant::to_bytes(
            zbus::zvariant::serialized::Context::new_dbus(zbus::zvariant::LE, 0),
            &inner,
        )
        .expect("the property list serializes");
        let bytes = encoded.bytes();
        for needle in [
            "IBusPropList",
            "Pyidaungsu MM",
            "Myanmar3",
            "keyboard.myanmar3",
        ] {
            assert!(
                bytes
                    .windows(needle.len())
                    .any(|window| window == needle.as_bytes()),
                "serialized properties contain {needle}"
            );
        }
    }
}
