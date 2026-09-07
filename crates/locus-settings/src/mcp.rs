use super::*;
use gpui_kit::component::{
    Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    switch::Switch,
    tab::{Tab, TabBar},
};
use rust_i18n::t;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum McpClient {
    #[default]
    Codex,
    ClaudeCode,
    Cursor,
    OpenCode,
    Generic,
}

impl McpClient {
    const ALL: [Self; 5] = [
        Self::Codex,
        Self::ClaudeCode,
        Self::Cursor,
        Self::OpenCode,
        Self::Generic,
    ];

    fn label(self, language: Language) -> String {
        match self {
            Self::Codex => "Codex".to_owned(),
            Self::ClaudeCode => "Claude Code".to_owned(),
            Self::Cursor => "Cursor".to_owned(),
            Self::OpenCode => "OpenCode".to_owned(),
            Self::Generic => {
                t!("settings.mcp.client.generic", locale = language.locale()).into_owned()
            }
        }
    }

    fn description(self, language: Language) -> String {
        match self {
            Self::Codex => t!(
                "settings.mcp.client.codex.description",
                locale = language.locale()
            )
            .into_owned(),
            Self::ClaudeCode => t!(
                "settings.mcp.client.claude.description",
                locale = language.locale()
            )
            .into_owned(),
            Self::Cursor => t!(
                "settings.mcp.client.cursor.description",
                locale = language.locale()
            )
            .into_owned(),
            Self::OpenCode => t!(
                "settings.mcp.client.opencode.description",
                locale = language.locale()
            )
            .into_owned(),
            Self::Generic => t!(
                "settings.mcp.client.generic.description",
                locale = language.locale()
            )
            .into_owned(),
        }
    }

    fn code_label(self, language: Language) -> String {
        match self {
            Self::Codex | Self::ClaudeCode | Self::OpenCode => {
                t!("settings.mcp.client.terminal", locale = language.locale()).into_owned()
            }
            Self::Cursor => "~/.cursor/mcp.json".to_owned(),
            Self::Generic => t!(
                "settings.mcp.client.connection_parameters",
                locale = language.locale()
            )
            .into_owned(),
        }
    }

    fn code(self, endpoint: &str) -> String {
        match self {
            Self::Codex => format!("codex mcp add locus --url {endpoint}"),
            Self::ClaudeCode => {
                format!("claude mcp add --transport http locus {endpoint}")
            }
            Self::Cursor => format!(
                "{{\n  \"mcpServers\": {{\n    \"locus\": {{\n      \"url\": \"{endpoint}\"\n    }}\n  }}\n}}"
            ),
            Self::OpenCode => format!("opencode mcp add locus --url {endpoint}"),
            Self::Generic => format!(
                "Name: locus\nTransport: Streamable HTTP\nURL: {endpoint}\nAuthentication: none"
            ),
        }
    }

    fn copy_key(self) -> &'static str {
        match self {
            Self::Codex => "mcp-codex-copy",
            Self::ClaudeCode => "mcp-claude-copy",
            Self::Cursor => "mcp-cursor-copy",
            Self::OpenCode => "mcp-opencode-copy",
            Self::Generic => "mcp-generic-copy",
        }
    }
}

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
        let language = self.preferences.language;
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

    fn copy_mcp_value(&mut self, key: &'static str, value: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.mcp_copied = Some(key);
        cx.notify();
    }

    fn select_mcp_client(&mut self, client: McpClient, cx: &mut Context<Self>) {
        if self.mcp_client != client {
            self.mcp_client = client;
            cx.notify();
        }
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

    fn render_mcp_code(
        &self,
        client: McpClient,
        code: String,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let key = client.copy_key();
        let code_font_size = self.preferences.code_font_size;
        let copied = self.mcp_copied == Some(key);
        let copy_label = if copied {
            t!(
                "settings.mcp.configuration_copied",
                locale = language.locale()
            )
            .into_owned()
        } else {
            t!(
                "settings.mcp.copy_configuration",
                locale = language.locale()
            )
            .into_owned()
        };
        let copy_value = code.clone();
        let copy_button = Button::new(key)
            .ghost()
            .xsmall()
            .icon(if copied { Icon::Check } else { Icon::Copy })
            .accessibility_label(copy_label)
            .on_click(
                cx.listener(move |this, _, _, cx| this.copy_mcp_value(key, copy_value.clone(), cx)),
            );

        div()
            .mt(ui_rem(12.0))
            .w_full()
            .min_w_0()
            .rounded(ui_rem(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h(ui_rem(32.0))
                    .pl(ui_rem(10.0))
                    .pr(ui_rem(4.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .child(client.code_label(language)),
                    )
                    .child(copy_button),
            )
            .child(
                div()
                    .id(format!("{key}-code"))
                    .role(Role::Code)
                    .px(ui_rem(10.0))
                    .py(ui_rem(8.0))
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .whitespace_nowrap()
                    .child(selectable_code_text(
                        "mcp-client-code-text",
                        code.into(),
                        SelectableCodeStyle {
                            font_family: self
                                .preferences
                                .resolved_code_font_family()
                                .to_owned()
                                .into(),
                            font_size: code_font_size,
                            line_height: (code_font_size * 1.5).round(),
                            color: theme.muted_foreground,
                            selection_color: theme.selection,
                        },
                        self.mcp_code_selection.clone(),
                        self.mcp_code_focus.clone(),
                        cx.entity_id(),
                    )),
            )
    }

    fn render_mcp_guide(&self, endpoint: &str, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let client = self.mcp_client;
        let selected_index = McpClient::ALL
            .iter()
            .position(|client| *client == self.mcp_client)
            .unwrap_or_default();
        let tabs = TabBar::new("mcp-client-tabs")
            .underline()
            .selected_index(selected_index)
            .on_click(cx.listener(|this, index: &usize, _, cx| {
                if let Some(client) = McpClient::ALL.get(*index) {
                    this.select_mcp_client(*client, cx);
                }
            }))
            .children(
                McpClient::ALL
                    .into_iter()
                    .map(|client| Tab::new().label(client.label(language))),
            );

        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .px(ui_rem(20.0))
            .py(ui_rem(15.0))
            .child(
                div().text_sm().font_weight(FontWeight::MEDIUM).child(
                    t!("settings.mcp.connect_agent", locale = language.locale()).into_owned(),
                ),
            )
            .child(tabs)
            .child(
                div()
                    .mt(ui_rem(12.0))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(client.description(language)),
            )
            .child(self.render_mcp_code(client, client.code(endpoint), cx))
    }

    pub(super) fn render_mcp(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let endpoint = self.mcp_server_state.endpoint().to_owned();
        let endpoint_copy = endpoint.clone();
        let status = match &self.mcp_server_state {
            McpServerState::Stopped { .. } => {
                t!("mcp.status.stopped", locale = language.locale()).into_owned()
            }
            McpServerState::Running { endpoint } => {
                format!(
                    "{} · {endpoint}",
                    t!("mcp.status.running", locale = language.locale()).into_owned()
                )
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
            .child(
                self.render_setting_row(
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
                    cx,
                ),
            )
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(
                self.render_setting_row(
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
                    cx,
                ),
            )
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(
                self.render_setting_row(
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
                    cx,
                ),
            )
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
            .child(self.render_setting_row(
                t!("settings.mcp.endpoint.label", locale = language.locale()).into_owned(),
                status,
                self.render_mcp_button(
                    "mcp-endpoint-copy",
                    if self.mcp_copied == Some("mcp-endpoint-copy") {
                        t!("settings.mcp.endpoint.copied", locale = language.locale()).into_owned()
                    } else {
                        t!("settings.mcp.endpoint.copy", locale = language.locale()).into_owned()
                    },
                    cx,
                    move |this, cx| {
                        this.copy_mcp_value("mcp-endpoint-copy", endpoint_copy.clone(), cx)
                    },
                ),
                cx,
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_mcp_guide(&endpoint, cx))
            .child(
                div()
                    .px(ui_rem(20.0))
                    .pb(ui_rem(18.0))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t!("settings.mcp.scope_note", locale = language.locale()).into_owned()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mcp_client_uses_the_current_endpoint_and_unique_controls() {
        let endpoint = "http://127.0.0.1:2301/mcp";
        let mut copy_keys = std::collections::HashSet::new();

        for client in McpClient::ALL {
            assert!(client.code(endpoint).contains(endpoint));
            assert!(copy_keys.insert(client.copy_key()));
        }
    }
}
