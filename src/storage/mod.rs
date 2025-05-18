mod in_memory;
mod local_fs;

pub use in_memory::InMemoryStorage;
pub use local_fs::LocalFileStorage;

use crate::errors::ApiError;
use async_trait::async_trait;
use mime::Mime;
use uuid::Uuid;

#[derive(Debug)]
pub struct FileData {
    pub bytes: Vec<u8>,
    pub content_type: Option<Mime>,
    pub filename: Option<String>,
}

#[async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn save(&self, file_data: FileData) -> Result<String, ApiError>;
    async fn get(&self, file_id: &str) -> Result<(Vec<u8>, Option<Mime>), ApiError>;
    async fn list(&self) -> Result<Vec<(String, Option<String>)>, ApiError>;
    async fn delete(&self, file_id: &str) -> Result<(), ApiError>;
}
