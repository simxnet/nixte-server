use super::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tokio::fs;

pub struct LocalFileStorage {
    storage_path: PathBuf,
}

impl LocalFileStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self, ApiError> {
        if !storage_path.exists() {
            std::fs::create_dir_all(&storage_path).map_err(|_| ApiError::Internal)?;
        }
        Ok(Self { storage_path })
    }
}

#[async_trait]
impl Storage for LocalFileStorage {
    async fn save(&self, file_data: FileData) -> Result<String, ApiError> {
        let file_id = Uuid::new_v4().to_string();
        let file_path = self.storage_path.join(&file_id);

        // Save metadata
        let meta_path = self.storage_path.join(format!("{}.meta", file_id));
        let mut meta_file = File::create(meta_path).map_err(|_| ApiError::Internal)?;

        let meta = serde_json::json!({
            "content_type": file_data.content_type.map(|m| m.to_string()),
            "filename": file_data.filename
        });

        meta_file
            .write_all(meta.to_string().as_bytes())
            .map_err(|_| ApiError::Internal)?;

        // Save file data
        tokio::fs::write(&file_path, file_data.bytes)
            .await
            .map_err(|_| ApiError::Internal)?;

        Ok(file_id)
    }

    async fn get(&self, file_id: &str) -> Result<(Vec<u8>, Option<Mime>), ApiError> {
        let file_path = self.storage_path.join(file_id);
        let meta_path = self.storage_path.join(format!("{}.meta", file_id));

        // Read metadata
        let meta = tokio::fs::read_to_string(meta_path)
            .await
            .map_err(|_| ApiError::NotFound)?;

        let meta: serde_json::Value =
            serde_json::from_str(&meta).map_err(|_| ApiError::Internal)?;

        let content_type = meta["content_type"].as_str().and_then(|s| s.parse().ok());

        // Read file data
        let bytes = fs::read(&file_path).await.map_err(|_| ApiError::NotFound)?;

        Ok((bytes, content_type))
    }

    async fn list(&self) -> Result<Vec<(String, Option<String>)>, ApiError> {
        let mut entries = fs::read_dir(&self.storage_path)
            .await
            .map_err(|_| ApiError::Internal)?;

        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|_| ApiError::Internal)? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if !file_name.ends_with(".meta") {
                let meta_path = self.storage_path.join(format!("{}.meta", file_name));
                if let Ok(meta) = tokio::fs::read_to_string(meta_path).await {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta) {
                        let filename = meta["filename"].as_str().map(|s| s.to_string());
                        files.push((file_name.to_string(), filename));
                    }
                }
            }
        }

        Ok(files)
    }

    async fn delete(&self, file_id: &str) -> Result<(), ApiError> {
        let file_path = self.storage_path.join(file_id);
        let meta_path = self.storage_path.join(format!("{}.meta", file_id));

        fs::remove_file(file_path)
            .await
            .map_err(|_| ApiError::NotFound)?;

        let _ = fs::remove_file(meta_path).await;
        Ok(())
    }
}
