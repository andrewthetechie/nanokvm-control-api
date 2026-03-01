use crate::error::AppError;
use std::path::PathBuf;

pub mod client;
pub mod mock;

#[async_trait::async_trait]
pub trait NanoKvmClient: Send + Sync {
    /// Mount an ISO file to the virtual media CD-ROM drive
    async fn mount_iso(&self, path: &PathBuf) -> Result<(), AppError>;

    /// Unmount the currently mounted ISO
    async fn unmount_iso(&self) -> Result<(), AppError>;
}
