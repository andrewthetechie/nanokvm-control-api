//! Mock implementation
use super::NanoKvmClient;
use crate::error::AppError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone, Default)]
pub struct MockNanoKvmClient {
    pub mounted_iso: Arc<Mutex<Option<PathBuf>>>,
}

impl MockNanoKvmClient {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub async fn get_mounted_iso(&self) -> Option<PathBuf> {
        self.mounted_iso.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl NanoKvmClient for MockNanoKvmClient {
    async fn mount_iso(&self, path: &PathBuf) -> Result<(), AppError> {
        info!("MOCK: Mounting ISO: {:?}", path);
        *self.mounted_iso.lock().await = Some(path.clone());
        Ok(())
    }

    async fn unmount_iso(&self) -> Result<(), AppError> {
        info!("MOCK: Unmounting ISO");
        *self.mounted_iso.lock().await = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_mount_unmount() {
        let mock = MockNanoKvmClient::new();
        let path = PathBuf::from("/tmp/test.iso");

        assert_eq!(mock.get_mounted_iso().await, None);

        mock.mount_iso(&path).await.unwrap();
        assert_eq!(mock.get_mounted_iso().await, Some(path));

        mock.unmount_iso().await.unwrap();
        assert_eq!(mock.get_mounted_iso().await, None);
    }
}
