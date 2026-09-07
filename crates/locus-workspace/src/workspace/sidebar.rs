use rust_i18n::t;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui_kit::{
    Animation, AnimationExt, AnyElement, Div, DragMoveEvent, FontWeight, ObjectFit, Role,
    StatefulInteractiveElement, StyledImage, Transformation, div, ease_out_quint, prelude::*, px,
    radians,
};

use super::Locus;
use crate::sidebar_order::reorder_siblings;
use gpui_kit::component::Theme;
use gpui_kit::component::{menu::ContextMenuExt as _, scroll::Scrollbar};
use locus_library::{ChangeStatus, LibraryNode};
use locus_ui::{Icon, document_icon_image, icon};
use oneo_gpui::Language;
use oneo_gpui::sidebar_selection_background;

const TREE_ROW_HEIGHT: f32 = 30.0;
const TREE_LEVEL_INDENT: f32 = 16.0;

#[derive(Clone)]
struct DraggedLibraryNode {
    path: String,
    parent: String,
}

impl Locus {
    pub(super) fn render_library_sidebar(
        &self,
        language: Language,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Div {
        let theme = Theme::global(cx).clone();
        if self.settings.read(cx).preferences().library_path.is_none() {
            return sidebar_message(
                t!("workspace.library.no_selection", locale = language.locale()).into_owned(),
                theme.muted_foreground,
            );
        }
        if self.library_tree.error.is_some() {
            return sidebar_message(
                t!("workspace.library.load_error", locale = language.locale()).into_owned(),
                theme.danger,
            );
        }

        let mut body = div()
            .id("library-tree-scroll")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.library_scroll_handle)
            .px(px(8.0))
            .pb_3();
        if self.library_tree.repository_error.is_some() {
            body = body.child(
                div()
                    .mx(px(4.0))
                    .mb(px(8.0))
                    .px(px(9.0))
                    .py(px(7.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.danger.opacity(0.35))
                    .text_xs()
                    .text_color(theme.danger)
                    .child(
                        t!(
                            "workspace.library.git_unavailable",
                            locale = language.locale()
                        )
                        .into_owned(),
                    ),
            );
        }
        if self.library_tree.nodes.is_empty() {
            let message = if self.library_tree.archived_notes.is_empty() {
                t!("workspace.library.empty", locale = language.locale()).into_owned()
            } else {
                t!("workspace.library.all_archived", locale = language.locale()).into_owned()
            };
            return div().size_full().child(
                body.child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(message),
                ),
            );
        }
        if self.library_watcher_error.is_some() {
            body = body.child(
                div()
                    .mx(px(4.0))
                    .mb(px(8.0))
                    .px(px(9.0))
                    .py(px(7.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.danger.opacity(0.35))
                    .text_xs()
                    .text_color(theme.danger)
                    .child(
                        t!(
                            "workspace.library.watching_unavailable",
                            locale = language.locale()
                        )
                        .into_owned(),
                    ),
            );
        }

        for node in &self.library_tree.nodes {
            body = body.child(self.render_library_node(node, 0, cx));
        }
        div()
            .relative()
            .size_full()
            .child(body)
            .child(Scrollbar::vertical(&self.library_scroll_handle))
    }

    fn render_library_node(
        &self,
        node: &LibraryNode,
        depth: usize,
        cx: &mut gpui_kit::Context<Self>,
    ) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let has_children = !node.children.is_empty();
        let expanded = self.expanded_library_nodes.contains(&node.path);
        let selected = self.selected_library_node.as_deref() == Some(node.path.as_str());
        let path = node.path.clone();
        let parent = parent_path(&path).to_owned();
        let dragged = self.dragged_library_node.as_deref() == Some(path.as_str());
        let drag = DraggedLibraryNode {
            path: path.clone(),
            parent: parent.clone(),
        };
        let select_path = path.clone();
        let content = div()
            .id(format!("library-node-action-{}", node.path))
            .role(Role::Button)
            .aria_label(node.title.clone())
            .h_full()
            .min_w_0()
            .flex_1()
            .child(library_node_content(node, expanded, theme.clone()))
            .on_drag(drag, |_, _, _, cx| cx.new(|_| gpui_kit::Empty))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_library_node(select_path.clone(), has_children, cx);
                cx.stop_propagation();
                cx.notify();
            }));
        let content = if node.document_path.is_some() {
            content
                .context_menu(self.note_popup_menu(node, cx))
                .into_any_element()
        } else {
            content.into_any_element()
        };

        let row = div()
            .id(format!("library-node-{}", node.path))
            .h(px(TREE_ROW_HEIGHT))
            .w_full()
            .min_w_0()
            .pl(px(depth as f32 * TREE_LEVEL_INDENT))
            .rounded(px(7.0))
            .flex()
            .items_center()
            .cursor_default()
            .when(selected, |row| {
                row.bg(sidebar_selection_background(&theme))
                    .text_color(theme.foreground)
            })
            .when(selected, |row| {
                row.anchor_scroll(Some(self.selected_library_scroll_anchor.clone()))
            })
            .when(!selected, |row| {
                let selection_background = sidebar_selection_background(&theme);
                row.hover(move |style| style.bg(selection_background))
                    .active(move |style| style.bg(selection_background))
            })
            .when(dragged, |row| row.bg(theme.secondary))
            .child(content)
            .on_drag_move::<DraggedLibraryNode>(cx.listener({
                let target_path = path.clone();
                let target_parent = parent.clone();
                move |this, event: &DragMoveEvent<DraggedLibraryNode>, _, cx| {
                    if !event.bounds.contains(&event.event.position) {
                        return;
                    }
                    let dragged = event.drag(cx).clone();
                    if this.dragged_library_node.as_deref() != Some(dragged.path.as_str()) {
                        this.dragged_library_node = Some(dragged.path.clone());
                        cx.notify();
                    }
                    if dragged.parent != target_parent || dragged.path == target_path {
                        return;
                    }
                    let Some(relation) =
                        sibling_relation(&this.library_tree.nodes, &dragged.path, &target_path)
                    else {
                        return;
                    };
                    if crossed_reorder_threshold(
                        relation,
                        f32::from(event.event.position.y),
                        f32::from(event.bounds.origin.y),
                        f32::from(event.bounds.size.height),
                    ) {
                        this.reorder_library_node(&dragged.path, &target_path, &target_parent, cx);
                    }
                }
            }));

        let mut branch = div().w_full().min_w_0().flex().flex_col().child(row);
        if has_children && expanded {
            for child in &node.children {
                branch = branch.child(self.render_library_node(child, depth + 1, cx));
            }
        }

        if let Some(offset) = self.library_reorder_offsets.get(&node.path).copied()
            && offset != 0.0
        {
            let animation_id = format!(
                "library-reorder-{}-{}",
                self.library_reorder_generation, node.path
            );
            branch
                .relative()
                .with_animation(
                    animation_id,
                    Animation::new(Duration::from_millis(150)).with_easing(ease_out_quint()),
                    move |branch, progress| branch.top(px(offset * (1.0 - progress))),
                )
                .into_any_element()
        } else {
            branch.into_any_element()
        }
    }

    fn reorder_library_node(
        &mut self,
        dragged_path: &str,
        target_path: &str,
        parent: &str,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        let Some(previous_layout) = sibling_layout(
            &self.library_tree.nodes,
            dragged_path,
            &self.expanded_library_nodes,
        ) else {
            return;
        };
        let Some(paths) = reorder_siblings(&mut self.library_tree.nodes, dragged_path, target_path)
        else {
            return;
        };
        self.library_reorder_offsets = reorder_animation_offsets(&previous_layout, &paths);
        self.library_reorder_generation = self.library_reorder_generation.wrapping_add(1);
        let Some(library_path) = self
            .current_library_path(cx)
            .map(std::path::Path::to_path_buf)
        else {
            return;
        };
        self.sidebar_order.set_group(&library_path, parent, paths);
        if let Err(error) = self.sidebar_order.save() {
            eprintln!("无法保存侧边栏顺序：{error}");
        }
        cx.notify();
    }

    pub(super) fn select_library_node(
        &mut self,
        path: String,
        has_children: bool,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        self.library_navigation
            .visit(self.selected_library_node.as_deref(), &path);
        self.apply_library_selection(path, has_children, cx);
    }

    pub(super) fn restore_library_selection(
        &mut self,
        path: String,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        let Some(has_children) = self
            .library_tree
            .node(&path)
            .map(|node| !node.children.is_empty())
        else {
            return;
        };
        self.apply_library_selection(path, has_children, cx);
        self.scroll_to_selected_library_node = true;
        cx.notify();
    }

    fn apply_library_selection(
        &mut self,
        path: String,
        has_children: bool,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        let could_reload_current_note = self.can_reload_current_note();
        let selection_changed = self.selected_library_node.as_deref() != Some(path.as_str());
        if selection_changed {
            self.copied_library_node = None;
            self.note_text_selection.reset();
            self.note_code_blocks.reset();
            self.note_copy_generation = self.note_copy_generation.wrapping_add(1);
            self.note_scroll_handle
                .set_offset(gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(0.0)));
        }
        self.selected_library_node = Some(path.clone());
        self.expanded_library_nodes = expanded_paths_for_selection(&path, has_children);
        self.save_interface_state(cx);
        if selection_changed {
            self.begin_selected_note_load(cx);
            self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
        }
    }
}

fn parent_path(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn sibling_layout(
    nodes: &[LibraryNode],
    path: &str,
    expanded: &HashSet<String>,
) -> Option<Vec<(String, f32)>> {
    if nodes.iter().any(|node| node.path == path) {
        return Some(
            nodes
                .iter()
                .map(|node| (node.path.clone(), visible_branch_height(node, expanded)))
                .collect(),
        );
    }
    nodes
        .iter()
        .find_map(|node| sibling_layout(&node.children, path, expanded))
}

fn sibling_relation(
    nodes: &[LibraryNode],
    dragged_path: &str,
    target_path: &str,
) -> Option<Ordering> {
    let dragged_index = nodes.iter().position(|node| node.path == dragged_path);
    let target_index = nodes.iter().position(|node| node.path == target_path);
    if let (Some(dragged_index), Some(target_index)) = (dragged_index, target_index) {
        return Some(dragged_index.cmp(&target_index));
    }
    nodes
        .iter()
        .find_map(|node| sibling_relation(&node.children, dragged_path, target_path))
}

fn crossed_reorder_threshold(
    relation: Ordering,
    pointer_y: f32,
    target_y: f32,
    target_height: f32,
) -> bool {
    let target_middle = target_y + target_height / 2.0;
    match relation {
        Ordering::Less => pointer_y >= target_middle,
        Ordering::Greater => pointer_y <= target_middle,
        Ordering::Equal => false,
    }
}

fn visible_branch_height(node: &LibraryNode, expanded: &HashSet<String>) -> f32 {
    TREE_ROW_HEIGHT
        + if expanded.contains(&node.path) {
            node.children
                .iter()
                .map(|child| visible_branch_height(child, expanded))
                .sum()
        } else {
            0.0
        }
}

fn reorder_animation_offsets(
    previous_layout: &[(String, f32)],
    next_paths: &[String],
) -> HashMap<String, f32> {
    let heights = previous_layout
        .iter()
        .map(|(path, height)| (path.as_str(), *height))
        .collect::<HashMap<_, _>>();
    let previous_tops = layout_tops(
        previous_layout.iter().map(|(path, _)| path.as_str()),
        &heights,
    );
    let next_tops = layout_tops(next_paths.iter().map(String::as_str), &heights);
    next_tops
        .into_iter()
        .filter_map(|(path, next_top)| {
            let offset = previous_tops.get(path).copied()? - next_top;
            (offset != 0.0).then(|| (path.to_owned(), offset))
        })
        .collect()
}

fn layout_tops<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    heights: &HashMap<&str, f32>,
) -> HashMap<&'a str, f32> {
    let mut top = 0.0;
    paths
        .into_iter()
        .map(|path| {
            let current = top;
            top += heights.get(path).copied().unwrap_or(TREE_ROW_HEIGHT);
            (path, current)
        })
        .collect()
}

fn library_node_content(node: &LibraryNode, expanded: bool, theme: Theme) -> Div {
    let arrow = if node.children.is_empty() {
        div().h(px(26.0)).w(px(26.0)).flex_none().into_any_element()
    } else {
        div()
            .h(px(26.0))
            .w(px(26.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                icon(Icon::ChevronDown, 13.0, theme.muted_foreground).transform(
                    Transformation::rotate(radians(if expanded {
                        0.0
                    } else {
                        -std::f32::consts::FRAC_PI_2
                    })),
                ),
            )
            .into_any_element()
    };

    div()
        .h(px(TREE_ROW_HEIGHT))
        .min_w_0()
        .flex_1()
        .flex()
        .items_center()
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
                            .text_color(theme.muted_foreground)
                            .child(node.attachments.len().to_string()),
                    )
                })
                .when_some(node.change.or(node.descendant_change), |row, status| {
                    row.child(change_indicator(status, node.change.is_none(), theme))
                }),
        )
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
    if let Some(image) = &node.icon_image {
        let fallback = node.icon.clone();
        return Some(
            document_icon_image(image)
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

fn sidebar_message(message: impl Into<gpui_kit::SharedString>, color: gpui_kit::Hsla) -> Div {
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
    use super::{
        crossed_reorder_threshold, expanded_paths_for_selection, reorder_animation_offsets,
    };
    use std::cmp::Ordering;
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

    #[test]
    fn reorder_animation_starts_at_the_previous_layout_position() {
        let previous = vec![
            ("alpha".to_owned(), 90.0),
            ("beta".to_owned(), 30.0),
            ("gamma".to_owned(), 30.0),
        ];
        let next = vec!["beta".to_owned(), "alpha".to_owned(), "gamma".to_owned()];

        let offsets = reorder_animation_offsets(&previous, &next);

        assert_eq!(offsets.get("beta"), Some(&90.0));
        assert_eq!(offsets.get("alpha"), Some(&-30.0));
        assert_eq!(offsets.get("gamma"), None);
    }

    #[test]
    fn reorder_waits_until_the_pointer_crosses_the_target_middle() {
        assert!(!crossed_reorder_threshold(Ordering::Less, 44.0, 30.0, 30.0));
        assert!(crossed_reorder_threshold(Ordering::Less, 45.0, 30.0, 30.0));
        assert!(!crossed_reorder_threshold(
            Ordering::Greater,
            46.0,
            30.0,
            30.0
        ));
        assert!(crossed_reorder_threshold(
            Ordering::Greater,
            45.0,
            30.0,
            30.0
        ));
    }
}
