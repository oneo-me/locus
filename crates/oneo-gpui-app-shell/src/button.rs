use gpui::{Div, FocusHandle, Stateful, div, prelude::*};
use gpui_component::{Icon, IconName, Sizable as _, Theme};

pub fn toolbar_icon_button(
    id: &'static str,
    label: &'static str,
    focus: &FocusHandle,
    icon: IconName,
    theme: &Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .tab_index(0)
        .tab_stop(true)
        .size_6()
        .flex_none()
        .rounded(theme.radius)
        .flex()
        .items_center()
        .justify_center()
        .cursor_default()
        .focus_visible(|style| style.border_1().border_color(theme.ring))
        .hover(|style| style.bg(theme.accent))
        .active(|style| style.bg(theme.accent))
        .child(Icon::new(icon).small().text_color(theme.muted_foreground))
}
