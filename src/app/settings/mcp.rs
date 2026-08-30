use super::*;

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

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::OpenCode => "OpenCode",
            Self::Generic => language.text("通用", "Generic"),
        }
    }

    fn description(self, language: Language) -> &'static str {
        match self {
            Self::Codex => language.text(
                "CLI 与 IDE 扩展共享这项配置；进入 Codex 后运行 /mcp 检查连接。",
                "The CLI and IDE extension share this configuration; run /mcp in Codex to verify it.",
            ),
            Self::ClaudeCode => language.text(
                "添加 HTTP 类型服务；进入 Claude Code 后运行 /mcp 检查连接。",
                "Adds an HTTP server; run /mcp in Claude Code to verify it.",
            ),
            Self::Cursor => language.text(
                "写入全局 MCP 配置，然后在 Cursor 中刷新 MCP 服务。",
                "Add this to the global MCP configuration, then refresh MCP servers in Cursor.",
            ),
            Self::OpenCode => language.text(
                "添加远程 MCP 服务，再运行 opencode mcp list 检查连接。",
                "Add the remote MCP server, then run opencode mcp list to verify it.",
            ),
            Self::Generic => language.text(
                "客户端字段名称可能不同；选择 Streamable HTTP，不要选择旧式 SSE。",
                "Field names vary by client; choose Streamable HTTP, not legacy SSE.",
            ),
        }
    }

    fn code_label(self, language: Language) -> &'static str {
        match self {
            Self::Codex | Self::ClaudeCode | Self::OpenCode => language.text("终端", "Terminal"),
            Self::Cursor => "~/.cursor/mcp.json",
            Self::Generic => language.text("连接参数", "Connection parameters"),
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

    fn tab_id(self) -> &'static str {
        match self {
            Self::Codex => "mcp-codex-tab",
            Self::ClaudeCode => "mcp-claude-tab",
            Self::Cursor => "mcp-cursor-tab",
            Self::OpenCode => "mcp-opencode-tab",
            Self::Generic => "mcp-generic-tab",
        }
    }

    fn tab_focus(self) -> usize {
        match self {
            Self::Codex => FOCUS_MCP_CODEX_TAB,
            Self::ClaudeCode => FOCUS_MCP_CLAUDE_TAB,
            Self::Cursor => FOCUS_MCP_CURSOR_TAB,
            Self::OpenCode => FOCUS_MCP_OPENCODE_TAB,
            Self::Generic => FOCUS_MCP_GENERIC_TAB,
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
                    language
                        .text(
                            "端口必须是 1 到 65535 之间的整数。",
                            "Port must be an integer from 1 to 65535.",
                        )
                        .to_owned(),
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
        label: &'static str,
        checked: bool,
        local_network: bool,
        focus_index: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        div()
            .id(id)
            .role(Role::Switch)
            .aria_label(label)
            .aria_toggled(Toggled::from(checked))
            .track_focus(&self.controls[focus_index])
            .tab_index(0)
            .tab_stop(true)
            .w(px(34.0))
            .h(px(20.0))
            .p(px(3.0))
            .rounded(px(10.0))
            .bg(if checked {
                theme.focus
            } else {
                theme.border_strong
            })
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.text))
            .child(
                div()
                    .size(px(14.0))
                    .rounded(px(7.0))
                    .bg(theme.surface)
                    .when(checked, |knob| knob.ml(px(14.0))),
            )
            .on_activation(cx, move |this, _, cx| this.toggle_mcp(local_network, cx))
    }

    fn render_mcp_button(
        &self,
        id: &'static str,
        label: &'static str,
        focus_index: usize,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.controls[focus_index])
            .tab_index(0)
            .tab_stop(true)
            .h(px(30.0))
            .px(px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.raised)
            .flex()
            .items_center()
            .text_sm()
            .text_color(theme.text_secondary)
            .cursor_default()
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .focus_visible(|style| style.border_color(theme.focus))
            .child(label)
            .on_activation(cx, move |this, _, cx| action(this, cx))
    }

    fn render_mcp_client_tab(
        &self,
        client: McpClient,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let selected = self.mcp_client == client;
        let label = client.label(language);
        div()
            .id(client.tab_id())
            .role(Role::Tab)
            .aria_label(label)
            .aria_selected(selected)
            .track_focus(&self.controls[client.tab_focus()])
            .tab_index(0)
            .tab_stop(true)
            .h(px(34.0))
            .px(px(2.0))
            .flex()
            .items_center()
            .border_b_2()
            .border_color(if selected { theme.focus } else { theme.border })
            .cursor_default()
            .text_xs()
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected {
                theme.text
            } else {
                theme.text_secondary
            })
            .hover(|style| style.text_color(theme.text))
            .focus_visible(|style| style.border_color(theme.focus))
            .child(label)
            .on_activation(cx, move |this, _, cx| this.select_mcp_client(client, cx))
    }

    fn render_mcp_code(
        &self,
        client: McpClient,
        code: String,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let key = client.copy_key();
        let code_font_size = self.preferences.code_font_size;
        let copied = self.mcp_copied == Some(key);
        let copy_label = if copied {
            language.text("配置已复制", "Configuration copied")
        } else {
            language.text("复制配置", "Copy configuration")
        };
        let copy_value = code.clone();
        let copy_button = div()
            .id(key)
            .role(Role::Button)
            .aria_label(copy_label)
            .track_focus(&self.controls[FOCUS_MCP_GUIDE_COPY])
            .tab_index(0)
            .tab_stop(true)
            .size(px(24.0))
            .flex_none()
            .rounded(px(5.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(
                if copied { Icon::Check } else { Icon::Copy },
                13.0,
                theme.text_tertiary,
            ))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, _, cx| {
                this.copy_mcp_value(key, copy_value.clone(), cx)
            });

        div()
            .mt(px(12.0))
            .w_full()
            .min_w_0()
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h(px(32.0))
                    .pl(px(10.0))
                    .pr(px(4.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme.text_tertiary)
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
                    .px(px(10.0))
                    .py(px(8.0))
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .whitespace_nowrap()
                    .child(selectable_code_text(
                        "mcp-client-code-text",
                        code.into(),
                        self.preferences
                            .resolved_code_font_family()
                            .to_owned()
                            .into(),
                        code_font_size,
                        (code_font_size * 1.5).round(),
                        theme.text_secondary,
                        theme.focus.opacity(if theme.is_dark { 0.34 } else { 0.24 }),
                        self.mcp_code_selection.clone(),
                        self.controls[FOCUS_MCP_GUIDE_CODE].clone(),
                        cx.entity_id(),
                    )),
            )
    }

    fn render_mcp_guide(&self, endpoint: &str, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let client = self.mcp_client;
        let tabs = McpClient::ALL.into_iter().fold(
            div()
                .id("mcp-client-tabs")
                .role(Role::TabList)
                .flex()
                .items_center()
                .gap(px(14.0))
                .border_b_1()
                .border_color(theme.border),
            |tabs, client| tabs.child(self.render_mcp_client_tab(client, cx)),
        );

        div()
            .px(px(20.0))
            .py(px(15.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(language.text("配置到 Agent", "Connect an Agent")),
            )
            .child(tabs)
            .child(
                div()
                    .mt(px(12.0))
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(client.description(language)),
            )
            .child(self.render_mcp_code(client, client.code(endpoint), cx))
    }

    pub(super) fn render_mcp(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let endpoint = self.mcp_server_state.endpoint().to_owned();
        let endpoint_copy = endpoint.clone();
        let status = match &self.mcp_server_state {
            McpServerState::Stopped { .. } => language.text("已停止", "Stopped").to_owned(),
            McpServerState::Running { endpoint } => {
                format!("{} · {endpoint}", language.text("运行中", "Running"))
            }
            McpServerState::Failed { message, .. } => format!(
                "{}：{message}",
                language.text("启动失败", "Failed to start")
            ),
        };
        let port_button = language.text("应用", "Apply");

        div()
            .flex()
            .flex_col()
            .child(self.render_setting_row(
                language.text("启动服务", "Start service"),
                language.text(
                    "通过 Streamable HTTP 提供三个只读文档发现工具",
                    "Provides three read-only document discovery tools over Streamable HTTP",
                ),
                self.render_mcp_switch(
                    "mcp-enabled",
                    language.text("启动 MCP 服务", "Start MCP service"),
                    self.preferences.mcp_enabled,
                    false,
                    FOCUS_MCP_ENABLED,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("公开到局域网", "Share on local network"),
                language.text(
                    "无身份验证；同一网络的设备可检索笔记，并看到本机文件绝对路径",
                    "No authentication; devices on the same network can search notes and see absolute local file paths",
                ),
                self.render_mcp_switch(
                    "mcp-local-network",
                    language.text("公开 MCP 到局域网", "Share MCP on local network"),
                    self.preferences.mcp_local_network,
                    true,
                    FOCUS_MCP_LOCAL_NETWORK,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("端口", "Port"),
                language.text(
                    "修改后应用会重启 MCP 服务",
                    "Applying a change restarts the MCP service",
                ),
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(self.mcp_port_input.clone())
                    .child(self.render_mcp_button(
                        "mcp-port-apply",
                        port_button,
                        FOCUS_MCP_PORT_APPLY,
                        cx,
                        |this, cx| this.apply_mcp_port(cx),
                    )),
                cx,
            ))
            .when_some(self.mcp_port_error.as_ref(), |view, error| {
                view.child(
                    div()
                        .px(px(20.0))
                        .pb(px(12.0))
                        .text_xs()
                        .text_color(theme.error)
                        .child(error.clone()),
                )
            })
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("服务地址", "Service URL"),
                status,
                self.render_mcp_button(
                    "mcp-endpoint-copy",
                    if self.mcp_copied == Some("mcp-endpoint-copy") {
                        language.text("已复制", "Copied")
                    } else {
                        language.text("复制地址", "Copy URL")
                    },
                    FOCUS_MCP_ENDPOINT_COPY,
                    cx,
                    move |this, cx| {
                        this.copy_mcp_value(
                            "mcp-endpoint-copy",
                            endpoint_copy.clone(),
                            cx,
                        )
                    },
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_mcp_guide(&endpoint, cx))
            .child(
                div()
                    .px(px(20.0))
                    .pb(px(18.0))
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(language.text(
                        "服务仅提供 list_articles、search_articles、resolve_article；正文与附件由 Agent 使用本地文件工具维护。",
                        "The server only provides list_articles, search_articles, and resolve_article; Agents maintain content and attachments with local file tools.",
                    )),
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
        let mut tab_focus = std::collections::HashSet::new();

        for client in McpClient::ALL {
            assert!(client.code(endpoint).contains(endpoint));
            assert!(copy_keys.insert(client.copy_key()));
            assert!(tab_focus.insert(client.tab_focus()));
        }
    }
}
