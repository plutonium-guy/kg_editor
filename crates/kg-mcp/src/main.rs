mod client;
mod server;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Write tracing to stderr so stdout stays clean for the MCP JSON-RPC
    // transport.  The log level is controlled by the RUST_LOG env var
    // (default: "info").
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    server::run().await
}
