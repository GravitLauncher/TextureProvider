mod auth;
mod config;
mod handlers;
mod models;
mod retrieval;
mod storage;

use axum::{
    extract::State,
    middleware,
    routing::{get, post},
    Router,
};
use config::Config;
use handlers::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use storage::create_storage;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::auth::decode_key;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv()?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "texture_provider=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    tracing::info!("Starting texture provider service");
    tracing::info!("Storage type: {:?}", config.storage_type);

    // Connect to database
    let db = sqlx::PgPool::connect(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Run migrations
    sqlx::query!("SELECT 1 as \"id: i32\"")
        .fetch_one(&db)
        .await
        .map_err(|e| anyhow::anyhow!("Database connection test failed: {}", e))?;

    tracing::info!("Database connection verified");

    // Initialize storage
    let storage: Arc<dyn storage::StorageBackend> = create_storage(config.clone());

    // Initialize texture retriever
    let retriever = retrieval::create_retriever(config.clone(), storage.clone(), db.clone());
    tracing::info!("Retrieval type: {:?}", config.retrieval_type);

    // Build application state
    let state = AppState {
        db,
        storage,
        retriever,
        config: config.clone(),
        public_key: Arc::new(decode_key(&config.jwt_public_key)?)
    };

    // Build our application with routes
    let app = Router::new()
        .route("/get/:uuid", get(handlers::get_textures))
        .route("/get/:uuid/:texture_type", get(handlers::get_texture))
        .route("/upload/:texture_type", post(handlers::upload_texture))
        .route("/api/upload/:type", post(handlers::admin_upload_texture))
        .route("/download/:texture_type/:uuid", get(handlers::download_texture))
        .route("/download/:hash", get(handlers::download_by_hash))
        .route("/files/:hash.:ext", get(handlers::serve_texture_file))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            add_public_key_to_state,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Middleware to add JWT public key and admin token to request state
async fn add_public_key_to_state(
    State(state): State<AppState>,
    mut request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    // Add public key to request extensions so it can be accessed by AuthUser extractor
    request
        .extensions_mut()
        .insert(state.public_key.clone());
    
    // Add admin token to request extensions if configured
    if let Some(ref admin_token) = state.config.admin_token {
        request
            .extensions_mut()
            .insert(format!("admin_token:{}", admin_token));
    }
    
    next.run(request).await
}
