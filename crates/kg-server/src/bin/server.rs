use kg_neo4j::{auth::basic, ClientBuilder};
use kg_schema::SchemaFile;
use std::sync::Arc;

use kg_server::config::Config;
use kg_server::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let cfg = Config::from_env();
    let schema = SchemaFile::from_yaml(std::path::Path::new(&cfg.schema_path))
        .expect("failed to load KG_SCHEMA yaml");
    let client = ClientBuilder::new(&cfg.neo4j_uri)
        .auth(basic(&cfg.neo4j_user, &cfg.neo4j_password))
        .build()
        .await
        .expect("failed to connect to Neo4j");
    let state = AppState { client: Arc::new(client), schema: Arc::new(schema) };

    let app = kg_server::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind).await.expect("bind");
    tracing::info!("kg-server listening on {}", cfg.bind);
    axum::serve(listener, app).await.expect("serve");
}
