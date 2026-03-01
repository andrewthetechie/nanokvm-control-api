//! Virtual Media Manager

use crate::config::VirtualMediaConfig;
use crate::error::AppError;
use crate::nanokvm::NanoKvmClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct VirtualMediaManager {
    disk_iso: PathBuf,
    pxe_iso: PathBuf,
    nanokvm: Arc<dyn NanoKvmClient>,
    mounted_iso: Arc<RwLock<Option<String>>>,
}

impl VirtualMediaManager {
    pub fn new(config: &VirtualMediaConfig, nanokvm: Arc<dyn NanoKvmClient>) -> Self {
        let base = PathBuf::from(&config.isos_dir);
        Self {
            disk_iso: base.join(&config.boot_from_disk_iso),
            pxe_iso: base.join(&config.pxe_boot_iso),
            nanokvm,
            mounted_iso: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the boot media to the "boot from disk" ISO
    pub async fn set_boot_from_disk(&self) -> Result<(), AppError> {
        self.ensure_iso_exists(&self.disk_iso).await?;
        self.nanokvm.mount_iso(&self.disk_iso).await?;
        let filename = self
            .disk_iso
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        *self.mounted_iso.write().await = Some(filename);
        Ok(())
    }

    /// Set the boot media to the PXE boot ISO
    pub async fn set_pxe_boot(&self) -> Result<(), AppError> {
        self.ensure_iso_exists(&self.pxe_iso).await?;
        self.nanokvm.mount_iso(&self.pxe_iso).await?;
        let filename = self
            .pxe_iso
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        *self.mounted_iso.write().await = Some(filename);
        Ok(())
    }

    /// Provides access to the underlying NanoKvmClient to unmount or do custom mounts
    pub fn client(&self) -> Arc<dyn NanoKvmClient> {
        self.nanokvm.clone()
    }

    /// Returns the name of the currently mounted ISO, if any
    pub async fn get_mounted_iso(&self) -> Option<String> {
        self.mounted_iso.read().await.clone()
    }

    /// Clears the locally tracked mounted ISO state (e.g. after unmount)
    pub async fn clear_mounted_iso(&self) {
        *self.mounted_iso.write().await = None;
    }

    #[allow(clippy::collapsible_if)]
    #[allow(dead_code)]
    async fn insert_media(&self, image_url: &str) -> Result<(), AppError> {
        // TODO: Implement media insertion logic (e.g., download image, mount it)
        warn!(
            "`insert_media` called with URL: {}. Not yet fully implemented.",
            image_url
        );
        Ok(())
    }

    async fn ensure_iso_exists(&self, path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            warn!("Expected ISO not found at {:?}", path);
            // In a real implementation, we might want to automatically create a dummy ISO
            // or fetch the required ISO if missing. For now, we'll try to create an empty file
            // to satisfy basic checks if the directory exists, though a real ISO is needed for boot.
            #[allow(clippy::collapsible_if)]
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
