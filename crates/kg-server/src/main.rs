mod config;
mod error;
mod routes;
mod state;

use axum::{routing::{get, post}, Router};
use kg_neo4j::{auth::basic, ClientBuilder};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let cfg = Config::from_env();
    let client = ClientBuilder::new(&cfg.neo4j_uri)
        .auth(basic(&cfg.neo4j_user, &cfg.neo4j_password))
        .build()
        .await
        .expect("failed to connect to Neo4j");
    let state = AppState { client: Arc::new(client) };

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/query",  post(routes::query::run))
        .route("/commit", post(routes::commit::run))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await.expect("bind");
    tracing::info!("kg-server listening on {}", cfg.bind);
    axum::serve(listener, app).await.expect("serve");
}
