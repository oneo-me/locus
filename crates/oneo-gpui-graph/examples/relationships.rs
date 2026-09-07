use gpui_kit::component::{Root, Theme};
use gpui_kit::{
    AppContext, Context, Entity, Render, SharedString, Subscription, Window, WindowOptions, div,
    prelude::*,
};
use oneo_gpui_graph::{GraphData, GraphEdge, GraphEvent, GraphLabels, GraphNode, GraphView};

struct Relationships {
    graph: Entity<GraphView>,
    status: SharedString,
    _subscription: Subscription,
}

impl Relationships {
    fn new(cx: &mut Context<Self>) -> Self {
        let data = GraphData::new(
            vec![
                GraphNode::new("api", "API").with_cluster("services"),
                GraphNode::new("worker", "Worker").with_cluster("services"),
                GraphNode::new("database", "Database")
                    .with_cluster("storage")
                    .with_radius(10.0),
                GraphNode::new("cache", "Cache").with_cluster("storage"),
            ],
            vec![
                GraphEdge::new("api", "database"),
                GraphEdge::new("api", "cache").with_distance(80.0),
                GraphEdge::new("worker", "database"),
            ],
        )
        .unwrap();
        let graph = cx.new(|cx| GraphView::new(data, GraphLabels::default(), cx));
        let subscription = cx.subscribe(&graph, |this, _, event, cx| {
            let GraphEvent::NodeActivated(id) = event;
            this.status = format!("Activated: {id}").into();
            cx.notify();
        });
        Self {
            graph,
            status: "Double-click a node to activate it".into(),
            _subscription: subscription,
        }
    }
}

impl Render for Relationships {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().flex_1().min_h_0().child(self.graph.clone()))
            .child(div().px_4().py_2().text_sm().child(self.status.clone()))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(Relationships::new);
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap();
        cx.activate(true);
    });
}
