mod fps;
mod platform;

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div, px,
    transparent_black,
};
use gpui_component::{
    Icon, IconName, Sizable as _, Theme,
    tab::{Tab, TabBar},
};

pub const HOME_TAB_ID: &str = "home";
pub const TITLEBAR_HEIGHT: f32 = 40.0;

pub use fps::{FpsCounterState, fps_counter};
pub use platform::{
    close_main_window, configure_main_window_close_behavior, configure_window_material,
    init_reduce_motion,
};

pub fn titlebar_drag_region(id: impl Into<ElementId>) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .h_full()
        .window_control_area(gpui::WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |_, window, _| {
            window.start_window_move();
        })
        .on_click(|event, window, _| {
            if event.click_count() == 2 {
                window.titlebar_double_click();
            }
        })
}

#[derive(Clone)]
pub struct AppShellTab {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
}

impl AppShellTab {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn icon(&self) -> Option<&IconName> {
        self.icon.as_ref()
    }
}

impl std::fmt::Debug for AppShellTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppShellTab")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("icon", &self.icon.as_ref().map(|_| "<icon>"))
            .finish()
    }
}

#[derive(Debug)]
pub struct AppShellState {
    tabs: Vec<AppShellTab>,
    selected_index: usize,
}

impl AppShellState {
    pub fn new(home_label: impl Into<SharedString>) -> Self {
        Self {
            tabs: vec![AppShellTab {
                id: HOME_TAB_ID.into(),
                label: home_label.into(),
                icon: None,
            }],
            selected_index: 0,
        }
    }

    pub fn with_home_icon(mut self, icon: IconName) -> Self {
        self.tabs[0].icon = Some(icon);
        self
    }

    pub fn tabs(&self) -> &[AppShellTab] {
        &self.tabs
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_id(&self) -> &str {
        self.tabs[self.selected_index].id()
    }

    pub fn select(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.selected_index = index;
        }
    }

    pub fn open_tab(&mut self, id: impl Into<SharedString>, label: impl Into<SharedString>) {
        let id = id.into();
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.tabs[index].label = label.into();
            self.selected_index = index;
            return;
        }

        self.tabs.push(AppShellTab {
            id,
            label: label.into(),
            icon: None,
        });
        self.selected_index = self.tabs.len() - 1;
    }

    pub fn open_tab_with_icon(
        &mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: IconName,
    ) {
        let id = id.into();
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.tabs[index].label = label.into();
            self.tabs[index].icon = Some(icon);
            self.selected_index = index;
            return;
        }

        self.tabs.push(AppShellTab {
            id,
            label: label.into(),
            icon: Some(icon),
        });
        self.selected_index = self.tabs.len() - 1;
    }

    pub fn close(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.tabs.len() {
            return false;
        }

        self.tabs.remove(index);
        if self.selected_index > index {
            self.selected_index -= 1;
        } else if self.selected_index == index && self.selected_index >= self.tabs.len() {
            self.selected_index = self.tabs.len() - 1;
        }
        true
    }

    pub fn close_selected(&mut self) -> bool {
        self.close(self.selected_index)
    }
}

type TabHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TabbedAppShell {
    tabs: Vec<AppShellTab>,
    selected_index: usize,
    content: AnyElement,
    titlebar_prefix: AnyElement,
    titlebar_drag_region: AnyElement,
    titlebar_status: Option<AnyElement>,
    close_tab_label: SharedString,
    on_select: Option<TabHandler>,
    on_close: Option<TabHandler>,
}

impl TabbedAppShell {
    pub fn new(
        state: &AppShellState,
        content: impl IntoElement,
        titlebar_prefix: impl IntoElement,
        titlebar_drag_region: impl IntoElement,
        close_tab_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            tabs: state.tabs.clone(),
            selected_index: state.selected_index,
            content: content.into_any_element(),
            titlebar_prefix: titlebar_prefix.into_any_element(),
            titlebar_drag_region: titlebar_drag_region.into_any_element(),
            titlebar_status: None,
            close_tab_label: close_tab_label.into(),
            on_select: None,
            on_close: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    pub fn titlebar_status(mut self, status: impl IntoElement) -> Self {
        self.titlebar_status = Some(status.into_any_element());
        self
    }
}

impl RenderOnce for TabbedAppShell {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx);
        let close_tab_label = self.close_tab_label;
        let on_close = self.on_close;
        let rendered_tabs = self
            .tabs
            .into_iter()
            .enumerate()
            .map(|(index, tab)| {
                let tab_id = tab.id.clone();
                let icon = tab.icon;
                let tab = Tab::new().label(tab.label).min_h(px(TITLEBAR_HEIGHT));
                let tab = match icon {
                    Some(icon) => tab.prefix(div().pl(px(12.0)).mr(px(-8.0)).child(Icon::new(icon))),
                    None => tab,
                };
                if index == 0 {
                    return tab;
                }

                let handler = on_close.clone();
                tab.suffix(
                    div().pr(px(6.0)).child(
                        div()
                            .id((ElementId::from("app-shell-close-tab"), tab_id))
                            .role(Role::Button)
                            .aria_label(close_tab_label.clone())
                            .size(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .hover(|close| close.text_color(theme.foreground))
                            .child(Icon::new(IconName::Close).xsmall())
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                if let Some(handler) = &handler {
                                    handler(&index, window, cx);
                                }
                            }),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let mut tabs = TabBar::new("app-shell-tabs")
            .h(px(TITLEBAR_HEIGHT))
            .w_auto()
            .bg(transparent_black())
            .max_width(px(220.0))
            .selected_index(self.selected_index)
            .children(rendered_tabs);
        if let Some(handler) = self.on_select {
            tabs = tabs.on_click(move |index, window, cx| handler(index, window, cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(transparent_black())
            .text_color(theme.foreground)
            .child(
                div()
                    .id("app-shell-titlebar")
                    .w_full()
                    .h(px(TITLEBAR_HEIGHT))
                    .flex_none()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.titlebar_prefix)
                    .child(
                        div()
                            .h_full()
                            .w_auto()
                            .min_w_0()
                            .flex_shrink_1()
                            .child(tabs),
                    )
                    .child(self.titlebar_drag_region)
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .pr(px(10.0))
                            .children(self.titlebar_status),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_always_the_first_tab() {
        let mut state = AppShellState::new("Home");
        state.open_tab("documents", "Documents");

        assert_eq!(state.tabs()[0].id(), HOME_TAB_ID);
        assert_eq!(state.tabs()[0].label(), "Home");
    }

    #[test]
    fn invalid_selection_does_not_change_the_active_tab() {
        let mut state = AppShellState::new("Home");
        state.select(8);

        assert_eq!(state.selected_id(), HOME_TAB_ID);
    }

    #[test]
    fn home_cannot_close_and_closing_selected_uses_the_adjacent_tab() {
        let mut state = AppShellState::new("Home");
        assert!(!state.close_selected());

        state.open_tab("backup:a", "A");
        state.open_tab("backup:b", "B");
        assert!(state.close_selected());
        assert_eq!(state.selected_id(), "backup:a");
    }
}
