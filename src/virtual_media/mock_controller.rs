use super::controller::MediaController;
use crate::error::AppError;
use std::path::Path;
use tokio::sync::RwLock;
use tracing::info;

pub struct MockMediaController {
    mounted_iso: RwLock<Option<String>>,
}

impl MockMediaController {
    pub fn new() -> Self {
        Self {
            mounted_iso: RwLock::new(None),
        }
    }

    #[allow(dead_code)]
    pub async fn get_mounted_iso(&self) -> Option<String> {
        self.mounted_iso.read().await.clone()
    }
}

#[async_trait::async_trait]
impl MediaController for MockMediaController {
    async fn mount_iso(&self, path: &Path) -> Result<(), AppError> {
        info!("[MOCK] Force ejecting and mounting ISO: {:?}", path);
        *self.mounted_iso.write().await = Some(path.to_string_lossy().into_owned());
        Ok(())
    }

    async fn unmount_iso(&self) -> Result<(), AppError> {
        info!("[MOCK] Force ejecting and unmounting ISO");
        *self.mounted_iso.write().await = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_mock_media_controller() {
        let controller = MockMediaController::new();
        assert_eq!(controller.get_mounted_iso().await, None);

        let iso_path = PathBuf::from("/test/iso.iso");
        controller.mount_iso(&iso_path).await.unwrap();
        assert_eq!(
            controller.get_mounted_iso().await,
            Some("/test/iso.iso".to_string())
        );

        controller.unmount_iso().await.unwrap();
        assert_eq!(controller.get_mounted_iso().await, None);
    }
}
