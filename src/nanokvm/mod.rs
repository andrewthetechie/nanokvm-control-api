use crate::error::AppError;

pub mod client;
pub mod mock;

#[async_trait::async_trait]
pub trait NanoKvmClient: Send + Sync {
    /// Mount an ISO file to the virtual media CD-ROM drive
    async fn mount_iso(&self, path: &std::path::Path) -> Result<(), AppError>;

    /// Unmount the currently mounted ISO
    async fn unmount_iso(&self) -> Result<(), AppError>;
}
