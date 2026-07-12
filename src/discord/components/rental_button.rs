use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};

pub fn build_rental_button(label: impl Into<String>) -> Vec<Component> {
    build_rental_button_with_custom_id(label, "rental_start")
}

pub fn build_rental_button_with_custom_id(
    label: impl Into<String>,
    custom_id: impl Into<String>,
) -> Vec<Component> {
    let button = Component::Button(Button {
        id: None,
        custom_id: Some(custom_id.into()),
        disabled: false,
        emoji: None,
        label: Some(label.into()),
        style: ButtonStyle::Primary,
        url: None,
        sku_id: None,
    });
    vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![button],
    })]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unwrap_button(components: &[Component]) -> &Button {
        let Component::ActionRow(row) = &components[0] else {
            panic!("最初の要素は ActionRow であること");
        };
        let Component::Button(button) = &row.components[0] else {
            panic!("ActionRow 内に Button があること");
        };
        button
    }

    #[test]
    fn default_button_uses_rental_start_custom_id() {
        let components = build_rental_button("レンタル");
        let button = unwrap_button(&components);
        assert_eq!(button.custom_id.as_deref(), Some("rental_start"));
        assert_eq!(button.label.as_deref(), Some("レンタル"));
        assert_eq!(button.style, ButtonStyle::Primary);
        assert!(!button.disabled);
    }

    #[test]
    fn custom_id_can_be_overridden() {
        let components = build_rental_button_with_custom_id("X", "my_custom_id");
        let button = unwrap_button(&components);
        assert_eq!(button.custom_id.as_deref(), Some("my_custom_id"));
        assert_eq!(button.label.as_deref(), Some("X"));
    }
}
