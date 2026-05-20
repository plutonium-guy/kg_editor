use std::sync::Arc;
use kg_neo4j::Client;
use kg_schema::SchemaFile;

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<Client>,
    pub schema: Arc<SchemaFile>,
}
