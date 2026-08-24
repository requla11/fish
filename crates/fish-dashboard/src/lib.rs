#![forbid(unsafe_code)]

pub mod api;
pub mod dashboard;
pub mod flamegraph;
pub mod metrics;
pub mod persistence;

pub use dashboard::DashboardServer;
pub use flamegraph::FlamegraphGenerator;
pub use metrics::BuildMetrics;
pub use persistence::{PersistentMetricsStore, TeamStats};
