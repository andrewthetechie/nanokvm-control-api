use crate::error::AppError;
use std::path::Path;
use tokio::fs;
use tracing::info;

#[async_trait::async_trait]
pub trait MediaController: Send + Sync {
    /// Mount an ISO by forcefully ejecting the current one and setting the new backing file
    async fn mount_iso(&self, path: &Path) -> Result<(), AppError>;

    /// Unmount the currently mounted ISO by forcefully ejecting and clearing the backing file
    async fn unmount_iso(&self) -> Result<(), AppError>;
}

#[allow(dead_code)]
pub struct LinuxConfigFsController {
    lun_path: String,
}

#[allow(dead_code)]
impl LinuxConfigFsController {
    pub fn new(lun_path: String) -> Self {
        Self { lun_path }
    }
}

#[async_trait::async_trait]
impl MediaController for LinuxConfigFsController {
    async fn mount_iso(&self, path: &Path) -> Result<(), AppError> {
        let forced_eject_path = format!("{}/forced_eject", self.lun_path);
        let file_path = format!("{}/file", self.lun_path);

        info!("Force ejecting existing media via configfs");
        fs::write(&forced_eject_path, "1")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write forced_eject: {}", e)))?;

        info!("Mounting ISO {:?} via configfs", path);
        fs::write(&file_path, path.to_string_lossy().as_ref())
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to write to configfs lun file: {}", e))
            })?;

        Ok(())
    }

    async fn unmount_iso(&self) -> Result<(), AppError> {
        let forced_eject_path = format!("{}/forced_eject", self.lun_path);
        let file_path = format!("{}/file", self.lun_path);

        info!("Force ejecting existing media via configfs to unmount");
        fs::write(&forced_eject_path, "1")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write forced_eject: {}", e)))?;

        info!("Clearing configfs lun file");
        fs::write(&file_path, "")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear configfs lun file: {}", e)))?;

        Ok(())
    }
}
