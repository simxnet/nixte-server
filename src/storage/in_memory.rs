use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct InMemoryStorage {
    files: Arc<RwLock<HashMap<String, (Vec<u8>, Option<Mime>, Option<String>)>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn save(&self, file_data: FileData) -> Result<String, ApiError> {
        let file_id = Uuid::new_v4().to_string();
        self.files.write().await.insert(
            file_id.clone(),
            (file_data.bytes, file_data.content_type, file_data.filename),
        );
        Ok(file_id)
    }

    async fn get(&self, file_id: &str) -> Result<(Vec<u8>, Option<Mime>), ApiError> {
        self.files
            .read()
            .await
            .get(file_id)
            .map(|(bytes, content_type, _)| (bytes.clone(), content_type.clone()))
            .ok_or(ApiError::NotFound)
    }

    async fn list(&self) -> Result<Vec<(String, Option<String>)>, ApiError> {
        Ok(self
            .files
            .read()
            .await
            .iter()
            .map(|(id, (_, _, filename))| (id.clone(), filename.clone()))
            .collect())
    }

    async fn delete(&self, file_id: &str) -> Result<(), ApiError> {
        self.files
            .write()
            .await
            .remove(file_id)
            .map(|_| ())
            .ok_or(ApiError::NotFound)
    }
}
