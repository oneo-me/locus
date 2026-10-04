use super::*;
use gpui_kit::component::{Disableable as _, Sizable as _, button::Button, switch::Switch};
use rust_i18n::t;

impl Settings {
    fn toggle_mcp(&mut self, local_network: bool, cx: &mut Context<Self>) {
        if local_network {
            self.preferences.mcp_local_network = !self.preferences.mcp_local_network;
        } else {
            self.preferences.mcp_enabled = !self.preferences.mcp_enabled;
        }
        self.persist();
        cx.emit(SettingsEvent::McpConfigurationChanged);
        cx.notify();
    }

    fn apply_mcp_port(&mut self, cx: &mut Context<Self>) {
        let language = self.preferences.appearance.language;
        let port = match self.mcp_port_draft.trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.mcp_port_error = Some(
                    t!(
                        "settings.mcp.port.validation_error",
                        locale = language.locale()
                    )
                    .into_owned(),
                );
                cx.notify();
                return;
            }
        };
        self.mcp_port_error = None;
        if port == self.preferences.mcp_port {
            cx.notify();
            return;
        }
        self.preferences.mcp_port = port;
        self.mcp_port_draft = port.to_string();
        self.persist();
        cx.emit(SettingsEvent::McpConfigurationChanged);
        cx.notify();
    }

    fn render_mcp_switch(
        &self,
        id: &'static str,
        label: String,
        checked: bool,
        local_network: bool,
        cx: &mut Context<Self>,
    ) -> Switch {
        Switch::new(id)
            .small()
            .checked(checked)
            .disabled(local_network && !self.preferences.mcp_enabled)
            .accessibility_label(label)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_mcp(local_network, cx)))
    }

    fn render_mcp_button(
        &self,
        id: &'static str,
        label: String,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Button {
        Button::new(id)
            .small()
            .label(label)
            .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
    }

    pub(super) fn render_mcp(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.appearance.language;
        let status = match &self.mcp_server_state {
            McpServerState::Stopped => {
                t!("mcp.status.stopped", locale = language.locale()).into_owned()
            }
            McpServerState::Running => {
                t!("mcp.status.running", locale = language.locale()).into_owned()
            }
            McpServerState::Failed { .. } => {
                t!("mcp.status.failed", locale = language.locale()).into_owned()
            }
        };
        let port_button = t!("settings.mcp.port.apply", locale = language.locale()).into_owned();

        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(SettingsItem::new(
                t!("settings.mcp.service.label", locale = language.locale()).into_owned(),
                t!(
                    "settings.mcp.service.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_mcp_switch(
                    "mcp-enabled",
                    t!("settings.mcp.service.toggle", locale = language.locale()).into_owned(),
                    self.preferences.mcp_enabled,
                    false,
                    cx,
                ),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.mcp.local_network.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.mcp.local_network.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_mcp_switch(
                    "mcp-local-network",
                    t!(
                        "settings.mcp.local_network.toggle",
                        locale = language.locale()
                    )
                    .into_owned(),
                    self.preferences.mcp_local_network,
                    true,
                    cx,
                ),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.mcp.port.title", locale = language.locale()).into_owned(),
                t!("settings.mcp.port.description", locale = language.locale()).into_owned(),
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(ui_rem(8.0))
                    .child(
                        Input::new(&self.mcp_port_input)
                            .w(ui_rem(164.0))
                            .aria_label(
                                t!("settings.mcp.port.label", locale = language.locale())
                                    .into_owned(),
                            ),
                    )
                    .child(self.render_mcp_button(
                        "mcp-port-apply",
                        port_button,
                        cx,
                        |this, cx| this.apply_mcp_port(cx),
                    )),
            ))
            .when_some(self.mcp_port_error.as_ref(), |view, error| {
                view.child(
                    div()
                        .px(ui_rem(20.0))
                        .pb(ui_rem(12.0))
                        .text_xs()
                        .text_color(theme.danger)
                        .child(error.clone()),
                )
            })
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.mcp.status.label", locale = language.locale()).into_owned(),
                status,
                div(),
            ))
            .when_some(self.mcp_server_error(), |view, error| {
                view.child(
                    div()
                        .px(ui_rem(20.0))
                        .pb(ui_rem(12.0))
                        .text_xs()
                        .text_color(theme.danger)
                        .child(error.to_owned()),
                )
            })
    }

    fn mcp_server_error(&self) -> Option<&str> {
        match &self.mcp_server_state {
            McpServerState::Failed { message } => Some(message),
            _ => None,
        }
    }
}
