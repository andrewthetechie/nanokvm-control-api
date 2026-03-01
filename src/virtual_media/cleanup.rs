//! ISO Cleanup logic
use crate::config::VirtualMediaConfig;
use crate::error::AppError;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{debug, info, warn};

pub async fn cleanup_old_isos(config: &VirtualMediaConfig) -> Result<(), AppError> {
    info!(
        "Starting cleanup of ISOs in {} older than {} seconds",
        config.isos_dir, config.cleanup_ttl_secs
    );

    let mut dir = fs::read_dir(&config.isos_dir).await.map_err(|e| {
        AppError::Internal(format!(
            "Failed to read ISO directory {}: {}",
            config.isos_dir, e
        ))
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut deleted_count = 0;

    while let Some(entry) = dir.next_entry().await.unwrap_or(None) {
        let path = entry.path();

        // Skip directories and the permanent ISOs
        if path.is_dir() {
            continue;
        }

        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if filename == config.boot_from_disk_iso || filename == config.pxe_boot_iso {
            continue;
        }

        #[allow(clippy::collapsible_if)]
        if let Ok(metadata) = entry.metadata().await {
            if let Ok(modified) = metadata.modified() {
                let mod_time = modified.duration_since(UNIX_EPOCH).unwrap().as_secs();
                let age = now.saturating_sub(mod_time);

                if age > config.cleanup_ttl_secs {
                    debug!("Deleting old ISO: {:?}", path);
                    if let Err(e) = fs::remove_file(&path).await {
                        warn!("Failed to delete {}: {}", path.display(), e);
                    } else {
                        deleted_count += 1;
                    }
                }
            }
        }
    }

    info!("Cleanup complete. Deleted {} ISOs.", deleted_count);
    Ok(())
}
