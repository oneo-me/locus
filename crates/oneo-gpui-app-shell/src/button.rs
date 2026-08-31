use gpui::{ElementId, SharedString, Styled as _, px};
use gpui_component::{
    IconName, Sizable as _, Theme,
    button::{Button, ButtonVariants as _},
};

use crate::TITLEBAR_CONTROL_SIZE;

pub fn toolbar_icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: IconName,
    theme: &Theme,
) -> Button {
    let label = label.into();
    Button::new(id)
        .ghost()
        .small()
        .size(px(TITLEBAR_CONTROL_SIZE))
        .icon(icon)
        .text_color(theme.muted_foreground)
        .accessibility_label(label.clone())
        .tooltip(label)
}
