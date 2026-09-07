use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div};
use oneo_gpui::{AppShell, SettingsStore, ShellWindow, titlebar_drag_region};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    greeting: Option<String>,
}

struct Welcome {
    preferences: Preferences,
}

impl Render for Welcome {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(div().h_12().child(titlebar_drag_region("titlebar")))
            .child(
                div().p_4().child(
                    self.preferences
                        .greeting
                        .clone()
                        .unwrap_or_else(|| "Welcome".into()),
                ),
            )
    }
}

fn main() {
    AppShell::new().run(|cx| {
        let store = SettingsStore::for_project("oneo-example")?;
        let preferences = store.load()?;
        ShellWindow::new("Oneo example").open(cx, |_, cx| cx.new(|_| Welcome { preferences }))
    });
}
