// @LITE_DESC Axum web server with routing, extractors, JSON responses, middleware, state sharing, and comprehensive error handling
// @LITE_SCENE Production-ready async web server demonstrating REST API patterns, request validation, shared state, and middleware
// @LITE_TAGS rust, axum, server, web, api, async

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// Application state shared across all request handlers
#[derive(Clone)]
struct AppState {
    startup_time: Instant,
}

// Request tracking middleware
async fn request_tracker(
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed();
    println!(
        "{} {} {} {}",
        method,
        uri,
        response.status(),
        format!("({:?})", duration).dimmed()
    );

    Ok(response)
}

// Error handling type
#[derive(Debug)]
enum AppError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

// Data models
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    limit: Option<usize>,
    offset: Option<usize>,
}

// In-memory user storage (use a real database in production)
type UserStore = Arc<tokio::sync::RwLock<Vec<User>>>;

// Handler functions
async fn health_check() -> &'static str {
    "OK"
}

async fn get_startup_time(State(state): State<AppState>) -> String {
    format!("Server running for {:?}", state.startup_time.elapsed())
}

async fn list_users(
    Query(params): Query<QueryParams>,
    State(users): State<UserStore>,
) -> Json<Vec<User>> {
    let users = users.read().await;
    let limit = params.limit.unwrap_or(10);
    let offset = params.offset.unwrap_or(0);

    let result: Vec<User> = users
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Json(result)
}

async fn get_user(
    Path(id): Path<Uuid>,
    State(users): State<UserStore>,
) -> Result<Json<User>, AppError> {
    let users = users.read().await;
    users
        .iter()
        .find(|u| u.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
        .map(Json)
}

async fn create_user(
    State(users): State<UserStore>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    if payload.name.is_empty() || payload.email.is_empty() {
        return Err(AppError::BadRequest("Name and email are required".into()));
    }

    let user = User {
        id: Uuid::new_v4(),
        name: payload.name,
        email: payload.email,
    };

    let mut users = users.write().await;
    users.push(user.clone());

    Ok((StatusCode::CREATED, Json(user)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize user store with some sample data
    let user_store: UserStore = Arc::new(tokio::sync::RwLock::new(vec![
        User {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        },
        User {
            id: Uuid::new_v4(),
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        },
    ]));

    let app_state = AppState {
        startup_time: Instant::now(),
    };

    // Build router with middleware and state
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/info", get(get_startup_time))
        .route("/users", get(list_users).post(create_user))
        .route("/users/:id", get(get_user))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(middleware::from_fn(request_tracker)),
        )
        .with_state(app_state)
        .with_state(user_store);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\nShutdown signal received, shutting down gracefully...");
}
