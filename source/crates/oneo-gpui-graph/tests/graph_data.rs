use oneo_gpui_graph::{GraphData, GraphEdge, GraphEdgeStyle, GraphError, GraphNode};

#[test]
fn arbitrary_ids_and_disconnected_nodes_need_no_domain_model() {
    let graph = GraphData::new(
        vec![
            GraphNode::new("service:api", "API"),
            GraphNode::new("数据库", "Database"),
            GraphNode::new("isolated", "Worker"),
        ],
        vec![
            GraphEdge::new("service:api", "数据库")
                .with_distance(80.0)
                .with_strength(0.5),
        ],
    )
    .unwrap();
    assert_eq!(graph.nodes().len(), 3);
    assert_eq!(graph.edges()[0].target(), "数据库");
    assert_eq!(graph.edges()[0].distance(), 80.0);
    assert_eq!(graph.edges()[0].strength(), 0.5);
    assert!(GraphData::default().nodes().is_empty());
}

#[test]
fn invalid_graphs_are_rejected_before_reaching_the_view() {
    let node = || GraphNode::new("a", "A");
    assert_eq!(
        GraphData::new(vec![node(), node()], vec![]),
        Err(GraphError::DuplicateNode("a".into()))
    );
    assert_eq!(
        GraphData::new(vec![node()], vec![GraphEdge::new("a", "missing")]),
        Err(GraphError::MissingNode("missing".into()))
    );
    assert_eq!(
        GraphData::new(vec![node()], vec![GraphEdge::new("a", "a")]),
        Err(GraphError::SelfEdge("a".into()))
    );
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(matches!(
            GraphData::new(vec![node().with_radius(radius)], vec![]),
            Err(GraphError::InvalidNode(_))
        ));
    }
    for edge in [
        GraphEdge::new("a", "b").with_distance(f32::NAN),
        GraphEdge::new("a", "b").with_strength(-0.1),
        GraphEdge::new("a", "b").with_style(
            GraphEdgeStyle::default().with_width(0.0),
            GraphEdgeStyle::default(),
        ),
        GraphEdge::new("a", "b").with_style(
            GraphEdgeStyle::default(),
            GraphEdgeStyle::default().with_opacity(2.0),
        ),
    ] {
        assert!(matches!(
            GraphData::new(vec![node(), GraphNode::new("b", "B")], vec![edge]),
            Err(GraphError::InvalidEdge { .. })
        ));
    }
}
