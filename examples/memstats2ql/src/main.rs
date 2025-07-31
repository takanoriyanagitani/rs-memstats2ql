use std::io;

use std::process::ExitCode;

use axum::Router;

use tokio::net::TcpListener;

use async_graphql_axum::GraphQLRequest;
use async_graphql_axum::GraphQLResponse;
use async_graphql_axum::GraphQLSubscription;

use rs_memstats2ql::MemSchema;

fn schema() -> MemSchema {
    rs_memstats2ql::schema_default()
}

async fn req2res(s: &MemSchema, req: GraphQLRequest) -> GraphQLResponse {
    s.execute(req.into_inner()).await.into()
}

fn env2listen_addr_port() -> Result<String, io::Error> {
    std::env::var("LISTEN_ADDR").map_err(io::Error::other)
}

async fn sub() -> Result<(), io::Error> {
    let ladr: String = env2listen_addr_port()?;

    let s: MemSchema = schema();
    let sdl: String = s.sdl();
    std::fs::write("memstats.graphql", sdl.as_bytes())?;

    let listener = TcpListener::bind(ladr).await?;

    let s4q: MemSchema = s.clone();

    let app = Router::new()
        .route(
            "/",
            axum::routing::post(|req: GraphQLRequest| async move { req2res(&s4q, req).await }),
        )
        .route_service("/ws", GraphQLSubscription::new(s));

    axum::serve(listener, app).await
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = sub().await {
        eprintln!("Server error: {e}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
