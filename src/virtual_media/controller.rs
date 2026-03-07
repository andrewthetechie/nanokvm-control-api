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
        let file_path = format!("{}/file", self.lun_path);
        let ro_path = format!("{}/ro", self.lun_path);
        let cdrom_path = format!("{}/cdrom", self.lun_path);
        let inquiry_path = format!("{}/inquiry_string", self.lun_path);

        info!("Mounting ISO {:?} via configfs", path);

        // First unmount any existing file
        fs::write(&file_path, "\n")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear configfs lun file: {}", e)))?;

        // Set to CD-ROM and read-only
        fs::write(&ro_path, "1")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set ro flag: {}", e)))?;
        fs::write(&cdrom_path, "1")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set cdrom flag: {}", e)))?;

        // Set inquiry string for CD-ROM
        let inquiry_data = format!("{:-8}{:-16}{:04x}", "NanoKVM", "USB CD/DVD-ROM", 0x0520);
        fs::write(&inquiry_path, inquiry_data.as_bytes())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set inquiry string: {}", e)))?;

        // Mount the new file
        fs::write(&file_path, path.to_string_lossy().as_ref())
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to write to configfs lun file: {}", e))
            })?;

        // Reset USB Gadget to force host to re-enumerate
        let commands = [
            "echo > /sys/kernel/config/usb_gadget/g0/UDC",
            "ls /sys/class/udc/ | cat > /sys/kernel/config/usb_gadget/g0/UDC",
        ];

        for cmd in commands {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to execute UDC reset: {}", e)))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    async fn unmount_iso(&self) -> Result<(), AppError> {
        let file_path = format!("{}/file", self.lun_path);
        let ro_path = format!("{}/ro", self.lun_path);
        let cdrom_path = format!("{}/cdrom", self.lun_path);

        info!("Clearing configfs lun file to unmount media");

        // Write exactly what the Official NanoKVM logic does for unmount
        fs::write(&file_path, "\n")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear configfs lun file: {}", e)))?;

        fs::write(&ro_path, "0")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear ro flag: {}", e)))?;

        fs::write(&cdrom_path, "0")
            .await
            .map_err(|e| AppError::Internal(format!("Failed to clear cdrom flag: {}", e)))?;

        // Reset USB Gadget to force host to re-enumerate
        let commands = [
            "echo > /sys/kernel/config/usb_gadget/g0/UDC",
            "ls /sys/class/udc/ | cat > /sys/kernel/config/usb_gadget/g0/UDC",
        ];

        for cmd in commands {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to execute UDC reset: {}", e)))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }
}
