#![forbid(unsafe_code)]

pub mod critical_path;
pub mod dynamic;
pub mod error;
pub mod graph;
pub mod lockless;
pub mod pash;
pub mod query;
pub mod state;

pub use critical_path::{CriticalPathAnalyzer, CriticalPathReport};
pub use dynamic::{DynamicGraphExpander, DynamicTaskSpec};
pub use error::GraphError;
pub use graph::{BuildGraph, Node, NodeId};
pub use lockless::{LocklessDependencyGraph, LocklessError, LocklessGraphNode};
pub use pash::{
    BoundarySymbol, InvalidationDecision, LanguageKind, PashExtractor, PolyAbiHyperGraph,
    SymbolKind, SymbolVisibility, SymbolicBoundary,
};
pub use query::{GraphQueryEngine, QueryExpr, parse_query};
pub use state::TaskState;
