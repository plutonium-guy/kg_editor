//! Thin wrapper around kg-server REST API.
//!
//! `KgClient` is constructed once in `main`, shared across tool handlers via
//! `Arc<KgClient>`, and calls the kg-server HTTP endpoints.  Full method
//! implementations land in T12; only the constructor is wired here.

pub struct KgClient {
    pub base: String,
    pub http: reqwest::Client,
}

impl KgClient {
    /// Build a client from the `KG_SERVER_URL` environment variable,
    /// defaulting to `http://localhost:9000`.
    pub fn from_env() -> Self {
        let base = std::env::var("KG_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:9000".into());
        Self {
            base,
            http: reqwest::Client::new(),
        }
    }
}
