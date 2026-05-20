use std::sync::Arc;
use kg_neo4j::Client;

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<Client>,
}
