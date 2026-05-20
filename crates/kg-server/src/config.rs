//! Server config loaded from environment.

pub struct Config {
    pub bind: String,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub cors_origins: Vec<String>,
    pub serve_ui_from: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let cors = std::env::var("KG_CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173".into());
        Config {
            bind: std::env::var("KG_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:9000".into()),
            neo4j_uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into()),
            neo4j_user: std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into()),
            neo4j_password: std::env::var("NEO4J_PASSWORD").expect("NEO4J_PASSWORD env var required"),
            cors_origins: cors.split(',').map(|s| s.trim().to_string()).collect(),
            serve_ui_from: std::env::var("KG_SERVE_UI").ok(),
        }
    }
}
