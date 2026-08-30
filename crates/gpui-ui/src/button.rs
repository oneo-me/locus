use gpui::{Div, FocusHandle, Role, Stateful, div, prelude::*, px};

use crate::{Icon, Theme, icon};

pub fn toolbar_icon_button(
    id: &'static str,
    label: &'static str,
    focus: &FocusHandle,
    icon_kind: Icon,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .tab_index(0)
        .tab_stop(true)
        .size(px(26.0))
        .flex_none()
        .rounded(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_default()
        .focus_visible(|style| style.border_1().border_color(theme.focus))
        .hover(|style| style.bg(theme.control_hover))
        .active(|style| style.bg(theme.control_active))
        .child(icon(icon_kind, 14.0, theme.text_secondary))
}
