//! kg-server library: route module tree + Router factory.

pub mod config;
pub mod error;
pub mod routes;
pub mod state;

use axum::{routing::{get, post}, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn router(state: state::AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/query",  post(routes::query::run))
        .route("/commit", post(routes::commit::run))
        .route("/schema", get(routes::schema::run))
        .route("/entities", axum::routing::get(routes::entities::list).post(routes::entities::create))
        .route("/entities/:id",
            axum::routing::get(routes::entities::get_one)
                .put(routes::entities::update)
                .delete(routes::entities::delete))
        .route("/links", axum::routing::post(routes::links::create))
        .route("/links/:id", axum::routing::delete(routes::links::delete))
        .route("/search", axum::routing::get(routes::search::run))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
