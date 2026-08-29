use gpui::{
    AnyElement, Div, FontWeight, ObjectFit, Role, StatefulInteractiveElement, StyledImage,
    Transformation, div, img, prelude::*, px, radians,
};

use super::Locus;
use crate::document_library::{ChangeStatus, LibraryNode};
use crate::localization::Language;
use crate::theme::Theme;
use crate::ui::{Icon, icon, vertical_scrollbar};

const TREE_ROW_HEIGHT: f32 = 30.0;
const TREE_LEVEL_INDENT: f32 = 16.0;

impl Locus {
    pub(super) fn render_library_sidebar(
        &self,
        language: Language,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);
        if self.settings.read(cx).preferences().library_path.is_none() {
            return sidebar_message(
                language.text("尚未打开文档库", "No library selected"),
                theme.text_secondary,
            );
        }
        if let Some(error) = &self.library_tree.error {
            return sidebar_message(error.clone(), theme.error);
        }

        let mut body = div()
            .id("library-tree-scroll")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.library_scroll_handle)
            .px(px(8.0))
            .py(px(12.0));
        if let Some(error) = &self.library_tree.repository_error {
            body = body.child(
                div()
                    .mx(px(4.0))
                    .mb(px(8.0))
                    .px(px(9.0))
                    .py(px(7.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.error.opacity(0.35))
                    .text_xs()
                    .text_color(theme.error)
                    .child(format!(
                        "{}：{error}",
                        language.text("Git 状态不可用", "Git status unavailable")
                    )),
            );
        }
        if self.library_tree.nodes.is_empty() {
            return div().size_full().child(
                body.child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child(language.text("文档库还是空的。", "The library is empty.")),
                ),
            );
        }
        if let Some(error) = &self.library_watcher_error {
            body = body.child(
                div()
                    .mx(px(4.0))
                    .mb(px(8.0))
                    .px(px(9.0))
                    .py(px(7.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.error.opacity(0.35))
                    .text_xs()
                    .text_color(theme.error)
                    .child(format!(
                        "{}：{error}",
                        language.text("文件观察不可用", "File watching unavailable")
                    )),
            );
        }

        for node in &self.library_tree.nodes {
            body = body.child(self.render_library_node(node, 0, cx));
        }
        div()
            .relative()
            .size_full()
            .child(body)
            .child(vertical_scrollbar(
                &self.library_scroll_handle,
                &self.library_scrollbar_state,
                0.0,
            ))
    }

    fn render_library_node(
        &self,
        node: &LibraryNode,
        depth: usize,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);
        let has_children = !node.children.is_empty();
        let expanded = self.expanded_library_nodes.contains(&node.path);
        let selected = self.selected_library_node.as_deref() == Some(node.path.as_str());
        let path = node.path.clone();

        let arrow = if has_children {
            div()
                .h(px(26.0))
                .w(px(26.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    icon(Icon::ChevronDown, 13.0, theme.text_tertiary).with_transformation(
                        Transformation::rotate(radians(if expanded {
                            0.0
                        } else {
                            -std::f32::consts::FRAC_PI_2
                        })),
                    ),
                )
                .into_any_element()
        } else {
            div().h(px(26.0)).w(px(26.0)).flex_none().into_any_element()
        };

        let row = div()
            .id(format!("library-node-{}", node.path))
            .role(Role::Button)
            .aria_label(node.title.clone())
            .h(px(TREE_ROW_HEIGHT))
            .w_full()
            .min_w_0()
            .pl(px(depth as f32 * TREE_LEVEL_INDENT))
            .rounded(px(7.0))
            .flex()
            .items_center()
            .cursor_default()
            .when(selected, |row| row.bg(theme.control_selected))
            .when(selected, |row| {
                row.anchor_scroll(Some(self.selected_library_scroll_anchor.clone()))
            })
            .when(!selected, |row| {
                row.hover(|style| style.bg(theme.control_hover))
                    .active(|style| style.bg(theme.control_active))
            })
            .child(arrow)
            .child(
                div()
                    .h(px(TREE_ROW_HEIGHT))
                    .min_w_0()
                    .flex_1()
                    .pl(px(4.0))
                    .pr(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .when_some(document_icon(node), |row, icon| row.child(icon))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_sm()
                            .font_weight(FontWeight::NORMAL)
                            .child(node.title.clone()),
                    )
                    .when(!node.attachments.is_empty(), |row| {
                        row.child(
                            div()
                                .flex_none()
                                .text_xs()
                                .text_color(theme.text_tertiary)
                                .child(node.attachments.len().to_string()),
                        )
                    })
                    .when_some(node.change.or(node.descendant_change), |row, status| {
                        row.child(change_indicator(status, node.change.is_none(), theme))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_library_node(path.clone(), has_children, cx);
                cx.stop_propagation();
                cx.notify();
            }));

        let mut branch = div().w_full().min_w_0().flex().flex_col().child(row);
        if has_children && expanded {
            for child in &node.children {
                branch = branch.child(self.render_library_node(child, depth + 1, cx));
            }
        }

        branch
    }

    fn select_library_node(
        &mut self,
        path: String,
        has_children: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        let selection_changed = self.selected_library_node.as_deref() != Some(path.as_str());
        if selection_changed {
            self.copied_library_node = None;
            self.note_copy_generation = self.note_copy_generation.wrapping_add(1);
            self.note_scroll_handle
                .set_offset(gpui::point(gpui::px(0.0), gpui::px(0.0)));
            self.note_scrollbar_state.reset_visibility();
        }
        self.selected_library_node = Some(path.clone());
        self.expanded_library_nodes = expanded_paths_for_selection(&path, has_children);
        self.save_interface_state();
        if selection_changed {
            self.begin_selected_note_load(cx);
            self.update_app_menus(cx);
        }
    }
}

pub(super) fn expanded_paths_for_selection(
    path: &str,
    has_children: bool,
) -> std::collections::HashSet<String> {
    let segments = path.split('/').collect::<Vec<_>>();
    let mut expanded = (1..segments.len())
        .map(|index| segments[..index].join("/"))
        .collect::<std::collections::HashSet<_>>();
    if has_children {
        expanded.insert(path.to_owned());
    }
    expanded
}

fn document_icon(node: &LibraryNode) -> Option<AnyElement> {
    if let Some(path) = &node.icon_path {
        let fallback = node.icon.clone();
        return Some(
            img(path.clone())
                .size(px(14.0))
                .flex_none()
                .rounded(px(3.0))
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    fallback
                        .clone()
                        .map(emoji_icon)
                        .unwrap_or_else(|| div().size(px(14.0)).into_any_element())
                })
                .into_any_element(),
        );
    }
    node.icon.clone().map(emoji_icon)
}

fn emoji_icon(icon: String) -> AnyElement {
    div()
        .size(px(14.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .line_height(px(14.0))
        .child(icon)
        .into_any_element()
}

fn sidebar_message(message: impl Into<gpui::SharedString>, color: gpui::Hsla) -> Div {
    div()
        .p_4()
        .text_sm()
        .text_color(color)
        .child(message.into())
}

fn change_indicator(status: ChangeStatus, descendant: bool, theme: Theme) -> Div {
    let color = match status {
        ChangeStatus::Added => theme.success,
        ChangeStatus::Modified => theme.warning,
    };
    div()
        .size(px(6.0))
        .flex_none()
        .rounded_full()
        .when(descendant, |dot| dot.border_1().border_color(color))
        .when(!descendant, |dot| dot.bg(color))
}

#[cfg(test)]
mod tests {
    use super::expanded_paths_for_selection;
    use std::collections::HashSet;

    #[test]
    fn selection_keeps_only_its_ancestor_chain_expanded() {
        assert_eq!(
            expanded_paths_for_selection("projects/locus/design", false),
            HashSet::from(["projects".to_owned(), "projects/locus".to_owned()])
        );
        assert_eq!(
            expanded_paths_for_selection("projects/locus", true),
            HashSet::from(["projects".to_owned(), "projects/locus".to_owned()])
        );
    }
}
