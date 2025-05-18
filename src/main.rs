use axum::{
    Json, Router,
    body::Bytes,
    extract::{Extension, Multipart, Path, Request},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
};
use dotenvy::dotenv;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use storage::{FileData, InMemoryStorage, LocalFileStorage, Storage};

mod errors;
mod storage;

use errors::ApiError;

#[derive(Debug, Clone)]
struct AppConfig {
    auth_token: String,
    storage_type: String,
    storage_path: String,
    host: String,
    port: u16,
}

// todo: mmm so basically make this config more useful anddd also separate this file cuz im getting
// a seizure every time i open ts
impl AppConfig {
    fn from_env() -> Self {
        dotenv().ok();

        Self {
            auth_token: env::var("AUTH_TOKEN").expect("AUTH_TOKEN must be set"),
            storage_type: env::var("STORAGE_TYPE").unwrap_or_else(|_| "memory".to_string()),
            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "./uploads".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
        }
    }

    fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid host or port")
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let config = AppConfig::from_env();
    let auth_token = config.auth_token.clone(); // Clone this before using in middleware

    let storage: Arc<dyn Storage> = match config.storage_type.as_str() {
        "memory" => Arc::new(InMemoryStorage::new()),
        "local" => Arc::new(
            LocalFileStorage::new(PathBuf::from(&config.storage_path))
                .expect("Failed to initialize local storage"),
        ),
        _ => panic!("Invalid storage type"),
    };

    let app = Router::new()
        .route("/file/{file_id}", get(get_file))
        .merge(
            Router::new()
                .route("/", get(health))
                .route("/upload", post(upload))
                .route("/files", get(list_files))
                .layer(middleware::from_fn(move |req: Request, next: Next| {
                    let token = auth_token.clone();
                    async move {
                        let auth_header = req
                            .headers()
                            .get("Authorization")
                            .and_then(|h| h.to_str().ok());

                        match auth_header {
                            Some(header) if header == format!("Bearer {token}") => {
                                next.run(req).await
                            }
                            _ => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
                        }
                    }
                })),
        )
        .layer(Extension(storage));

    let addr = config.socket_addr();
    tracing::info!("Server running on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK\nAPI Version: 1.0"
}

async fn upload(
    Extension(storage): Extension<Arc<dyn Storage>>,
    mut multipart: Multipart,
) -> Result<Json<Vec<(String, Option<String>)>>, ApiError> {
    let mut files_info = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        let content_type = field.content_type().map(|m| m.parse().unwrap());
        let filename = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|_| ApiError::Internal)?;

        let file_data = FileData {
            bytes: data.to_vec(),
            content_type,
            filename: filename.clone(),
        };

        let file_id = storage.save(file_data).await?;
        files_info.push((format!("/file/{file_id}"), filename));
    }

    Ok(Json(files_info))
}

async fn get_file(
    Path(file_id): Path<String>,
    Extension(storage): Extension<Arc<dyn Storage>>,
) -> Result<impl IntoResponse, ApiError> {
    let (bytes, content_type) = storage.get(&file_id).await?;

    let mut response = Bytes::from(bytes).into_response();

    if let Some(content_type) = content_type {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            content_type.to_string().parse().unwrap(),
        );
    }

    Ok(response)
}

async fn list_files(
    Extension(storage): Extension<Arc<dyn Storage>>,
) -> Result<Json<Vec<(String, Option<String>)>>, ApiError> {
    let files = storage.list().await?;
    let links = files
        .into_iter()
        .map(|(id, filename)| (format!("/file/{id}"), filename))
        .collect();
    Ok(Json(links))
}
