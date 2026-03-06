//! Virtual Media Manager

use crate::config::VirtualMediaConfig;
use crate::error::AppError;
use crate::nanokvm::NanoKvmClient;
use futures_util::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct VirtualMediaManager {
    isos_dir: PathBuf,
    disk_iso: PathBuf,
    pxe_iso: PathBuf,
    nanokvm: Arc<dyn NanoKvmClient>,
    mounted_iso: Arc<RwLock<Option<String>>>,
    http_client: Client,
    download_timeout: Duration,
}

impl VirtualMediaManager {
    pub fn new(config: &VirtualMediaConfig, nanokvm: Arc<dyn NanoKvmClient>) -> Self {
        let base = PathBuf::from(&config.isos_dir);
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.download_timeout_secs))
            .build()
            .expect("Failed to build HTTP client for VirtualMediaManager");
        Self {
            isos_dir: base.clone(),
            disk_iso: base.join(&config.boot_from_disk_iso),
            pxe_iso: base.join(&config.pxe_boot_iso),
            nanokvm,
            mounted_iso: Arc::new(RwLock::new(None)),
            http_client,
            download_timeout: Duration::from_secs(config.download_timeout_secs),
        }
    }

    /// Download an ISO from a URL to isos_dir, then mount it.
    /// Blocks until the download is complete before mounting.
    pub async fn insert_media(&self, image_url: &str) -> Result<(), AppError> {
        // Extract filename from URL, fallback to a sanitized default
        let filename = image_url
            .split('/')
            .last()
            .filter(|s| !s.is_empty())
            .unwrap_or("inserted.iso")
            .split('?') // strip query params
            .next()
            .unwrap_or("inserted.iso");

        let dest = self.isos_dir.join(filename);
        info!("Downloading ISO from {} to {:?}", image_url, dest);

        self.download_iso(image_url, &dest).await?;

        info!("Download complete, mounting {:?}", dest);
        self.nanokvm.mount_iso(&dest).await?;

        *self.mounted_iso.write().await = Some(filename.to_string());
        Ok(())
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

    /// Stream-download a URL to a local path, with timeout applied per the config.
    async fn download_iso(&self, url: &str, dest: &Path) -> Result<(), AppError> {
        // Ensure the target directory exists
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create ISO dir: {}", e)))?;
        }

        let response =
            tokio::time::timeout(self.download_timeout, self.http_client.get(url).send())
                .await
                .map_err(|_| AppError::Internal(format!("Download timed out: {}", url)))?
                .map_err(|e| AppError::Internal(format!("Failed to start download: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "Download failed with HTTP {}: {}",
                response.status(),
                url
            )));
        }

        // Write stream to file
        let mut file = fs::File::create(dest)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create file {:?}: {}", dest, e)))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| AppError::Internal(format!("Error reading download stream: {}", e)))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to write to {:?}: {}", dest, e)))?;
        }

        file.flush()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to flush {:?}: {}", dest, e)))?;

        info!("ISO written to {:?}", dest);
        Ok(())
    }

    async fn ensure_iso_exists(&self, path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            warn!("Expected ISO not found at {:?}", path);
            return Err(AppError::Internal(format!(
                "ISO not found at {:?}. Ensure the file exists before booting.",
                path
            )));
        }
        Ok(())
    }
}
