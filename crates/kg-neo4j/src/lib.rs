//! kg-neo4j: Neo4j transport + client.

#[cfg(all(feature = "native", feature = "wasm"))]
compile_error!(
    "features `native` and `wasm` are mutually exclusive; enable exactly one"
);

#[cfg(not(any(feature = "native", feature = "wasm")))]
compile_error!(
    "kg-neo4j requires exactly one of the `native` or `wasm` features"
);

pub mod auth;
pub mod error;
mod transport;

pub use auth::{basic, Auth};
pub use error::{Neo4jError, TransportError};
pub use kg_core;
