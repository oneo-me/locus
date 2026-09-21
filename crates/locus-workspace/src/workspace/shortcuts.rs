use super::Locus;
use gpui_kit::component::{Theme, WindowExt as _, notification::Notification};
use gpui_kit::{Action, AnyElement, App, Context, Window, prelude::*};
use locus_library::{LibraryNode, ShortcutKind};
use locus_preferences::EditorPreference;
use locus_ui::{Icon, toolbar_icon_button};
use rust_i18n::t;

pub(super) struct ShortcutCommand {
    kind: ShortcutKind,
    pub(super) label: String,
    tooltip: String,
}

impl ShortcutCommand {
    pub(super) fn icon(&self) -> gpui_kit::assets::IconName {
        match self.kind {
            ShortcutKind::Website => gpui_kit::assets::IconName::Globe,
            ShortcutKind::Github => gpui_kit::assets::IconName::Github,
            ShortcutKind::Directory => gpui_kit::assets::IconName::FolderPen,
        }
    }

    pub(super) fn action(&self) -> Box<dyn Action> {
        match self.kind {
            ShortcutKind::Website => Box::new(crate::OpenNoteWebsite),
            ShortcutKind::Github => Box::new(crate::OpenNoteGithub),
            ShortcutKind::Directory => Box::new(crate::OpenNoteProjectDirectory),
        }
    }
}

impl Locus {
    pub(super) fn note_shortcut_commands(&self, cx: &App) -> Vec<ShortcutCommand> {
        let Some(node) = self.current_note_node() else {
            return Vec::new();
        };
        let Some(shortcuts) = self.library_tree.resolved_shortcuts(&node.path) else {
            return Vec::new();
        };
        let preferences = self.settings.read(cx).preferences();
        let locale = preferences.appearance.language.locale();
        ShortcutKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let target = shortcuts.target(kind)?;
                let label = match kind {
                    ShortcutKind::Website => {
                        t!("workspace.shortcuts.website", locale = locale).into_owned()
                    }
                    ShortcutKind::Github => {
                        t!("workspace.shortcuts.github", locale = locale).into_owned()
                    }
                    ShortcutKind::Directory => match preferences.editor {
                        EditorPreference::System => {
                            t!("workspace.shortcuts.directory_system", locale = locale).into_owned()
                        }
                        EditorPreference::VisualStudioCode => {
                            t!("workspace.shortcuts.directory_editor", locale = locale).into_owned()
                        }
                    },
                };
                let target = if kind == ShortcutKind::Directory {
                    shortcuts
                        .directory_path()
                        .ok()
                        .flatten()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| target.into())
                } else {
                    target.into()
                };
                let mut tooltip = format!("{label}\n{target}");
                if shortcuts.source_path() != node.path {
                    tooltip.push_str(&format!(
                        "\n{}",
                        t!(
                            "workspace.shortcuts.inherited",
                            locale = locale,
                            title = shortcuts.source_title()
                        )
                    ));
                }
                Some(ShortcutCommand {
                    kind,
                    label,
                    tooltip,
                })
            })
            .collect()
    }

    pub(super) fn render_note_shortcuts(
        &self,
        node: &LibraryNode,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> (Vec<AnyElement>, Option<AnyElement>) {
        let mut buttons = Vec::new();
        let mut directory_button = None;
        for shortcut in self.note_shortcut_commands(cx) {
            let action = shortcut.action();
            let button = toolbar_icon_button(
                format!("note-shortcut-{}", shortcut.kind.key()),
                shortcut.tooltip.clone(),
                shortcut.icon(),
                theme,
            )
            .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
            .into_any_element();
            if shortcut.kind == ShortcutKind::Directory {
                directory_button = Some(button);
            } else {
                buttons.push(button);
            }
        }
        if let Some(shortcuts) = self.library_tree.resolved_shortcuts(&node.path)
            && !shortcuts.errors().is_empty()
        {
            let label = t!(
                "workspace.shortcuts.invalid",
                locale = self.language(cx).locale(),
                title = shortcuts.source_title()
            );
            let detail = format!("{label}\n{}", shortcuts.errors().join("\n"));
            buttons.push(
                toolbar_icon_button(
                    "invalid-note-shortcuts",
                    detail.clone(),
                    Icon::TriangleAlert,
                    theme,
                )
                .on_click(move |_, window, cx| {
                    window.push_notification(Notification::error(detail.clone()), cx);
                })
                .into_any_element(),
            );
        }
        (buttons, directory_button)
    }

    pub(super) fn open_note_shortcut(
        &mut self,
        kind: ShortcutKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(shortcuts) = self
            .current_note_node()
            .and_then(|node| self.library_tree.resolved_shortcuts(&node.path))
        else {
            return;
        };
        let Some(target) = shortcuts.target(kind) else {
            return;
        };
        if kind != ShortcutKind::Directory {
            cx.open_url(target);
            return;
        }
        let editor = self.settings.read(cx).preferences().editor;
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let path = shortcuts
                        .directory_path()?
                        .ok_or("Directory shortcut is missing")?;
                    if !path.is_dir() {
                        return Err(format!("目录不存在或无法访问：{}", path.display()));
                    }
                    locus_ui::platform::open_in_editor(&path, editor)
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Err(error) = result {
                    let label = t!(
                        "workspace.shortcuts.open_error",
                        locale = this.language(cx).locale()
                    );
                    window.push_notification(Notification::error(format!("{label}\n{error}")), cx);
                }
            });
        })
        .detach();
    }
}
