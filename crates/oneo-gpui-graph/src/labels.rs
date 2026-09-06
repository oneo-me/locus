use std::rc::Rc;

use gpui::SharedString;

#[derive(Clone)]
pub struct GraphLabels {
    pub(crate) loading: SharedString,
    pub(crate) empty: SharedString,
    pub(crate) accessibility: Rc<dyn Fn(usize, usize) -> SharedString>,
}

impl GraphLabels {
    /// The accessibility callback receives the currently displayed node and edge counts.
    pub fn new(
        loading: impl Into<SharedString>,
        empty: impl Into<SharedString>,
        accessibility: impl Fn(usize, usize) -> SharedString + 'static,
    ) -> Self {
        Self {
            loading: loading.into(),
            empty: empty.into(),
            accessibility: Rc::new(accessibility),
        }
    }
}

impl Default for GraphLabels {
    fn default() -> Self {
        Self::new("Arranging graph…", "No nodes", |nodes, edges| {
            format!("Relationship graph with {nodes} nodes and {edges} edges").into()
        })
    }
}
