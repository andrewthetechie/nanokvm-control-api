#![allow(dead_code)]
//! Mock implementation
use super::NanoKvmClient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
#[allow(dead_code)]
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
impl NanoKvmClient for MockNanoKvmClient {}

#[cfg(test)]
mod tests {
    // Tests removed as NanoKvmClient ISO methods were removed
}
