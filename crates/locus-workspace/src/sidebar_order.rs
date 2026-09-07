use oneo_gpui::SettingsStore;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};

use locus_library::LibraryNode;
use locus_preferences::config::SIDEBAR_ORDER_FILE;
use locus_preferences::storage::data_directory;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SidebarOrderState {
    libraries: BTreeMap<String, LibraryOrder>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct LibraryOrder {
    groups: BTreeMap<String, Vec<String>>,
}

impl SidebarOrderState {
    pub fn load() -> (Self, Option<String>) {
        match state_store()
            .and_then(|store| store.load::<Self>().map_err(|error| format!("{error:#}")))
        {
            Ok(state) => (state, None),
            Err(error) => (
                Self::default(),
                Some(format!("无法读取侧边栏顺序：{error}")),
            ),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        state_store()?
            .save(self)
            .map_err(|error| format!("{error:#}"))
    }

    pub fn apply(&self, library_path: &Path, nodes: &mut [LibraryNode]) {
        let Some(order) = self.libraries.get(&library_key(library_path)) else {
            return;
        };
        apply_groups(nodes, "", &order.groups);
    }

    pub fn set_group(&mut self, library_path: &Path, parent: &str, paths: Vec<String>) {
        self.libraries
            .entry(library_key(library_path))
            .or_default()
            .groups
            .insert(parent.to_owned(), paths);
    }
}

pub fn reorder_siblings(
    nodes: &mut [LibraryNode],
    dragged_path: &str,
    target_path: &str,
) -> Option<Vec<String>> {
    let dragged_index = nodes.iter().position(|node| node.path == dragged_path);
    let target_index = nodes.iter().position(|node| node.path == target_path);
    if let (Some(dragged_index), Some(target_index)) = (dragged_index, target_index) {
        if dragged_index == target_index {
            return None;
        }
        if dragged_index < target_index {
            nodes[dragged_index..=target_index].rotate_left(1);
        } else {
            nodes[target_index..=dragged_index].rotate_right(1);
        }
        return Some(nodes.iter().map(|node| node.path.clone()).collect());
    }

    nodes
        .iter_mut()
        .find_map(|node| reorder_siblings(&mut node.children, dragged_path, target_path))
}

fn apply_groups(nodes: &mut [LibraryNode], parent: &str, groups: &BTreeMap<String, Vec<String>>) {
    if let Some(paths) = groups.get(parent) {
        let ranks = paths
            .iter()
            .enumerate()
            .map(|(index, path)| (path.as_str(), index))
            .collect::<HashMap<_, _>>();
        nodes.sort_by(|left, right| {
            match (
                ranks.get(left.path.as_str()),
                ranks.get(right.path.as_str()),
            ) {
                (Some(left), Some(right)) => left.cmp(right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => default_node_order(left, right),
            }
        });
    }

    for node in nodes {
        let path = node.path.clone();
        apply_groups(&mut node.children, &path, groups);
    }
}

fn default_node_order(left: &LibraryNode, right: &LibraryNode) -> std::cmp::Ordering {
    left.children
        .is_empty()
        .cmp(&right.children.is_empty())
        .then_with(|| left.path.cmp(&right.path))
}

fn library_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn state_store() -> Result<SettingsStore, String> {
    let path = data_directory()?.join(SIDEBAR_ORDER_FILE);
    SettingsStore::at_path(path).map_err(|error| format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn node(path: &str, children: Vec<LibraryNode>) -> LibraryNode {
        LibraryNode {
            path: path.to_owned(),
            directory: PathBuf::from(path),
            document_path: None,
            title: path.to_owned(),
            icon: None,
            icon_image: None,
            summary: None,
            included_in_graph: true,
            additional_metadata: Vec::new(),
            attachments: Vec::new(),
            change: None,
            descendant_change: None,
            children,
        }
    }

    fn paths(nodes: &[LibraryNode]) -> Vec<&str> {
        nodes.iter().map(|node| node.path.as_str()).collect()
    }

    #[test]
    fn applies_saved_order_within_each_group() {
        let library = Path::new("/notes");
        let mut state = SidebarOrderState::default();
        state.set_group(library, "", vec!["beta".to_owned(), "alpha".to_owned()]);
        state.set_group(
            library,
            "alpha",
            vec!["alpha/two".to_owned(), "alpha/one".to_owned()],
        );
        let mut nodes = vec![
            node(
                "alpha",
                vec![node("alpha/one", vec![]), node("alpha/two", vec![])],
            ),
            node("beta", vec![]),
            node("new", vec![]),
        ];

        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("sidebar-order.toml")).unwrap();
        store.save(&state).unwrap();
        let restored: SidebarOrderState = store.load().unwrap();
        assert_eq!(restored, state);
        restored.apply(library, &mut nodes);

        assert_eq!(paths(&nodes), vec!["beta", "alpha", "new"]);
        assert_eq!(paths(&nodes[1].children), vec!["alpha/two", "alpha/one"]);
    }

    #[test]
    fn reorders_only_nodes_in_the_same_group() {
        let mut nodes = vec![
            node(
                "alpha",
                vec![node("alpha/one", vec![]), node("alpha/two", vec![])],
            ),
            node("beta", vec![node("beta/one", vec![])]),
        ];

        assert_eq!(
            reorder_siblings(&mut nodes, "alpha/two", "alpha/one"),
            Some(vec!["alpha/two".to_owned(), "alpha/one".to_owned()])
        );
        assert_eq!(reorder_siblings(&mut nodes, "alpha/one", "beta/one"), None);
    }
}
