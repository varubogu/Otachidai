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
