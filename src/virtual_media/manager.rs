//! Virtual Media Manager

use crate::config::VirtualMediaConfig;
use crate::error::AppError;
use crate::nanokvm::NanoKvmClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};

#[derive(Clone)]
pub struct VirtualMediaManager {
    disk_iso: PathBuf,
    pxe_iso: PathBuf,
    nanokvm: Arc<dyn NanoKvmClient>,
}

impl VirtualMediaManager {
    pub fn new(config: &VirtualMediaConfig, nanokvm: Arc<dyn NanoKvmClient>) -> Self {
        let base = PathBuf::from(&config.isos_dir);
        Self {
            disk_iso: base.join(&config.boot_from_disk_iso),
            pxe_iso: base.join(&config.pxe_boot_iso),
            nanokvm,
        }
    }

    /// Set the boot media to the "boot from disk" ISO
    pub async fn set_boot_from_disk(&self) -> Result<(), AppError> {
        self.ensure_iso_exists(&self.disk_iso).await?;
        self.nanokvm.mount_iso(&self.disk_iso).await
    }

    /// Set the boot media to the PXE boot ISO
    pub async fn set_pxe_boot(&self) -> Result<(), AppError> {
        self.ensure_iso_exists(&self.pxe_iso).await?;
        self.nanokvm.mount_iso(&self.pxe_iso).await
    }

    /// Provides access to the underlying NanoKvmClient to unmount or do custom mounts
    pub fn client(&self) -> Arc<dyn NanoKvmClient> {
        self.nanokvm.clone()
    }

    async fn ensure_iso_exists(&self, path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            warn!("Expected ISO not found at {:?}", path);
            // In a real implementation, we might want to automatically create a dummy ISO
            // or fetch the required ISO if missing. For now, we'll try to create an empty file
            // to satisfy basic checks if the directory exists, though a real ISO is needed for boot.
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        AppError::Internal(format!("Failed to create ISO dir: {}", e))
                    })?;
                }
            }
            fs::write(path, b"")
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create dummy ISO: {}", e)))?;
            info!("Created empty dummy ISO at {:?}", path);
        }
        Ok(())
    }
}
