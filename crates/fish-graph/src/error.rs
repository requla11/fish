use crate::graph::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    #[error("no node with id `{0}` in the graph")]
    MissingNode(NodeId),

    #[error("self-dependency is not allowed (`{0}` depends on itself)")]
    SelfDependency(NodeId),

    #[error("dependency cycle detected: {}", format_cycle_path(.path))]
    Cycle { path: Vec<NodeId> },
}

/// Render a cycle as a closed walk like `a -> b -> c -> a`. The stored path is
/// open form; the closing edge back to the first node is added here.
pub(crate) fn format_cycle_path<T: std::fmt::Display>(path: &[T]) -> String {
    let joined: Vec<String> = path.iter().map(ToString::to_string).collect();
    let mut rendered = joined.join(" -> ");
    if let Some(first) = joined.first() {
        rendered.push_str(" -> ");
        rendered.push_str(first);
    }
    rendered
}
