use rust_i18n::t;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use gpui_kit::{
    AnyElement, Div, DragMoveEvent, FontWeight, ObjectFit, StatefulInteractiveElement, StyledImage,
    Transformation, div, ease_out_quint, prelude::*, px, radians,
};

use super::Locus;
use crate::sidebar_order::reorder_siblings;
use gpui_kit::base::{Tree, TreeEntry, TreeEvent, TreeItem, TreeState};
use gpui_kit::component::Theme;
use gpui_kit::component::{menu::ContextMenuExt as _, scroll::Scrollbar};
use locus_library::{ChangeStatus, LibraryNode};
use locus_ui::{Icon, document_icon_image, icon};
use oneo_gpui::Language;
use oneo_gpui::sidebar_selection_background;

const TREE_ROW_HEIGHT: f32 = 30.0;
const TREE_LEVEL_INDENT: f32 = 16.0;

const REORDER_DURATION: Duration = Duration::from_millis(150);

pub(super) struct LibraryReorderAnimation {
    offsets: HashMap<String, f32>,
    started: Instant,
}

impl LibraryReorderAnimation {
    pub(super) fn is_finished(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= REORDER_DURATION
    }

    fn offset(&self, path: &str, now: Instant) -> Option<f32> {
        if self.is_finished(now) {
            return None;
        }
        let progress =
            now.duration_since(self.started).as_secs_f32() / REORDER_DURATION.as_secs_f32();
        let mut path = path;
        while !path.is_empty() {
            if let Some(offset) = self.offsets.get(path) {
                return Some(offset * (1.0 - ease_out_quint()(progress)));
            }
            path = parent_path(path);
        }
        None
    }
}

#[derive(Clone)]
struct DraggedLibraryNode {
    path: String,
    parent: String,
}

pub(super) fn library_tree_items(
    nodes: &[LibraryNode],
    expanded: &HashSet<String>,
) -> Vec<TreeItem> {
    nodes
        .iter()
        .map(|node| {
            TreeItem::new(node.path.clone(), node.title.clone())
                .expanded(expanded.contains(&node.path))
                .children(library_tree_items(&node.children, expanded))
        })
        .collect()
}

fn selected_tree_node(state: &TreeState) -> Option<(String, bool)> {
    state
        .selected_entry()
        .map(|entry| (entry.item().id.to_string(), entry.is_expanded()))
}

impl Locus {
    pub(super) fn sync_library_tree(&self, cx: &mut gpui_kit::Context<Self>) {
        let items = library_tree_items(&self.library_tree.nodes, &self.expanded_library_nodes);
        self.library_tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
            let ix = self
                .selected_library_node
                .as_ref()
                .and_then(|path| state.index_of(&path.clone().into()));
            state.set_selected_index(ix, cx);
        });
    }

    pub(super) fn observe_library_tree(
        state: &gpui_kit::Entity<TreeState>,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        cx.subscribe(state, |this, _, event, cx| {
            match event {
                TreeEvent::Expanded(path) => {
                    this.expanded_library_nodes.insert(path.to_string());
                }
                TreeEvent::Collapsed(path) => {
                    this.expanded_library_nodes.remove(path.as_ref());
                }
            }
            cx.notify();
        })
        .detach();
        cx.observe(state, |this, state, cx| {
            let selected = selected_tree_node(state.read(cx));
            if let Some((path, expanded)) = selected
                && this.selected_library_node.as_ref() != Some(&path)
                && this.library_tree.node(&path).is_some()
            {
                this.select_library_node(path, expanded, cx);
                cx.notify();
            }
        })
        .detach();
    }

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

        let mut body = div().size_full().min_h_0().flex().flex_col().pb_3();
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

        let owner = cx.entity().downgrade();
        let focus = self.library_tree_state.clone();
        let tree = Tree::new(&self.library_tree_state)
            .size_full()
            .list_style(gpui_kit::StyleRefinement::default().size_full())
            .item(move |_, entry, _, window, cx| {
                owner
                    .update(cx, |this, cx| this.render_library_node(entry, window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            });
        body.child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .child(
                    div()
                        .id("library-tree-focus")
                        .size_full()
                        .px(px(8.0))
                        .on_mouse_down(gpui_kit::MouseButton::Left, move |_, window, cx| {
                            focus.update(cx, |state, cx| state.focus(window, cx));
                        })
                        .child(tree),
                )
                .child(Scrollbar::vertical(
                    self.library_tree_state.read(cx).scroll_handle(),
                )),
        )
    }

    fn render_library_node(
        &self,
        entry: &TreeEntry,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> AnyElement {
        let Some(node) = self.library_tree.node(entry.item().id.as_ref()) else {
            return div().into_any_element();
        };
        let depth = entry.depth();
        let theme = Theme::global(cx).clone();
        let expanded = entry.is_expanded();
        let selected = self.selected_library_node.as_deref() == Some(node.path.as_str());
        let path = node.path.clone();
        let parent = parent_path(&path).to_owned();
        let dragged = self.dragged_library_node.as_deref() == Some(path.as_str());
        let drag = DraggedLibraryNode {
            path: path.clone(),
            parent: parent.clone(),
        };
        let content = div()
            .id(format!("library-node-action-{}", node.path))
            .h_full()
            .min_w_0()
            .flex_1()
            .child(library_node_content(node, expanded, theme.clone()))
            .on_drag(drag, |_, _, _, cx| cx.new(|_| gpui_kit::Empty));
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

        if !cx.reduce_motion()
            && let Some(offset) = self
                .library_reorder_animation
                .as_ref()
                .and_then(|animation| animation.offset(&node.path, Instant::now()))
            && offset != 0.0
        {
            window.request_animation_frame();
            row.relative().top(px(offset)).into_any_element()
        } else {
            row.into_any_element()
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
        self.library_reorder_animation = (!cx.reduce_motion()).then(|| LibraryReorderAnimation {
            offsets: reorder_animation_offsets(&previous_layout, &paths),
            started: Instant::now(),
        });
        self.sync_library_tree(cx);
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
        expand_selected: bool,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        self.library_navigation
            .visit(self.selected_library_node.as_deref(), &path);
        self.apply_library_selection(path, expand_selected, cx);
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
        expand_selected: bool,
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
        self.expanded_library_nodes = expanded_paths_for_selection(&path, expand_selected);
        self.sync_library_tree(cx);
        self.scroll_to_selected_library_node = true;
        self.save_interface_state();
        if selection_changed {
            self.schedule_note_canvas_selection(cx);
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
    expand_selected: bool,
) -> std::collections::HashSet<String> {
    let segments = path.split('/').collect::<Vec<_>>();
    let mut expanded = (1..segments.len())
        .map(|index| segments[..index].join("/"))
        .collect::<std::collections::HashSet<_>>();
    if expand_selected {
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

    #[gpui_kit::test]
    fn library_projection_preserves_hierarchy_identity_and_selection(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use super::library_tree_items;
        use gpui_kit::AppContext;
        use gpui_kit::base::TreeState;
        use locus_library::LibraryTree;

        let root = tempfile::tempdir().unwrap();
        for path in ["alpha/child", "beta"] {
            std::fs::create_dir_all(root.path().join(path)).unwrap();
            std::fs::write(
                root.path().join(path).join("index.md"),
                "---\ntitle: Same title\n---\n",
            )
            .unwrap();
        }
        let mut library = LibraryTree::load(Some(root.path()));
        let expanded = expanded_paths_for_selection("alpha/child", false);
        let state =
            cx.new(|cx| TreeState::new(cx).items(library_tree_items(&library.nodes, &expanded)));
        state.update(cx, |state, cx| {
            assert_eq!(state.entry(0).unwrap().item().id.as_ref(), "alpha");
            assert_eq!(state.entry(1).unwrap().depth(), 1);
            assert_eq!(state.entry(1).unwrap().item().label.as_ref(), "Same title");
            let ix = state.index_of(&"alpha/child".into());
            state.set_selected_index(ix, cx);
            assert_eq!(state.selected_item().unwrap().id.as_ref(), "alpha/child");

            super::reorder_siblings(&mut library.nodes, "beta", "alpha").unwrap();
            state.set_items(library_tree_items(&library.nodes, &expanded), cx);
            let ix = state.index_of(&"alpha/child".into());
            state.set_selected_index(ix, cx);
            assert_eq!(state.selected_index(), Some(2));
            assert_eq!(state.selected_item().unwrap().id.as_ref(), "alpha/child");

            state.set_items(library_tree_items(&library.nodes, &HashSet::new()), cx);
            assert!(state.index_of(&"alpha/child".into()).is_none());
            assert!(state.selected_item().is_none());
            state.set_items(Vec::new(), cx);
            assert!(state.entry(0).is_none());
        });
    }

    #[gpui_kit::test]
    fn clicking_an_expanded_parent_keeps_it_collapsed_after_selection(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::base::{Tree, TreeItem, TreeState};
        use gpui_kit::test::{TestSupportExt, TestWindowExt};
        use gpui_kit::{AppContext, Context, Entity, Render, Window, div, prelude::*, px, size};

        struct TreeFixture(Entity<TreeState>);
        impl Render for TreeFixture {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Tree::new(&self.0)
                    .size_full()
                    .list_style(gpui_kit::StyleRefinement::default().size_full())
                    .item(|_, entry, _, _, _| {
                        div()
                            .id(entry.item().id.clone())
                            .test_support()
                            .h(px(30.0))
                            .w_full()
                            .child(entry.item().label.clone())
                            .into_any_element()
                    })
            }
        }

        cx.update(gpui_kit::init);
        let state = cx.new(|cx| {
            let mut state = TreeState::new(cx).items(vec![
                TreeItem::new("parent", "Parent")
                    .expanded(true)
                    .child(TreeItem::new("parent/child", "Child")),
            ]);
            state.set_selected_index(Some(1), cx);
            state
        });
        let handle = cx.open_window(size(px(300.0), px(300.0)), |_, _| {
            TreeFixture(state.clone())
        });
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.click("parent", cx);
        })
        .unwrap();
        state.update(cx, |state, cx| {
            let (path, expanded) = super::selected_tree_node(state).unwrap();
            assert_eq!(path, "parent");
            assert!(!expanded);
            let paths = expanded_paths_for_selection(&path, expanded);
            assert!(!paths.contains("parent"));
            state.set_items(
                vec![
                    TreeItem::new("parent", "Parent")
                        .expanded(paths.contains("parent"))
                        .child(TreeItem::new("parent/child", "Child")),
                ],
                cx,
            );
            state.set_selected_index(Some(0), cx);
            assert!(state.index_of(&"parent/child".into()).is_none());
        });
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.click("parent", cx);
        })
        .unwrap();
        state.read_with(cx, |state, _| {
            assert_eq!(
                super::selected_tree_node(state),
                Some(("parent".to_owned(), true))
            );
            assert!(state.index_of(&"parent/child".into()).is_some());
        });
    }

    #[test]
    fn reappearing_rows_share_the_original_animation_deadline() {
        use std::collections::HashMap;
        use std::time::{Duration, Instant};
        let started = Instant::now();
        let animation = super::LibraryReorderAnimation {
            offsets: HashMap::from([("parent/item".to_owned(), 30.0)]),
            started,
        };
        assert_eq!(animation.offset("parent/item", started), Some(30.0));
        let later = started + Duration::from_millis(75);
        let offset = animation.offset("parent/item", later).unwrap();
        assert!(offset > 0.0 && offset < 30.0);
        assert_eq!(animation.offset("parent/item/child", later), Some(offset));
        let finished = started + super::REORDER_DURATION;
        assert!(animation.is_finished(finished));
        assert_eq!(animation.offset("parent/item", finished), None);
        assert_eq!(
            animation.offset("parent/item/child", finished + Duration::from_secs(1)),
            None
        );
    }

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
