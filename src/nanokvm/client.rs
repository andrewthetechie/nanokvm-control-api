//! HTTP client implementation

use super::NanoKvmClient;
use crate::config::NanoKvmConfig;
use crate::error::AppError;
use reqwest::{Client, StatusCode};
use serde_json::json;
use tracing::{debug, error, info};

pub struct HttpNanoKvmClient {
    client: Client,
    base_url: String,
    auth_token: String,
}

impl HttpNanoKvmClient {
    pub fn new(config: &NanoKvmConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.base_url.clone(),
            auth_token: config
                .auth_token
                .clone()
                .expect("auth_token must be set when use_mock is false (call validate() first)"),
        }
    }

    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }

    async fn send_request(
        &self,
        endpoint: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError> {
        let url = self.build_url(endpoint);
        let mut req = self.client.post(&url);
        req = req.header("Cookie", format!("nano-kvm-token={}", self.auth_token));

        let res = req
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("NanoKVM client error: {}", e)))?;

        match res.status() {
            StatusCode::OK => {
                debug!("Request to {} succeeded", url);
                Ok(())
            }
            status => {
                let err_msg = res.text().await.unwrap_or_default();
                error!(
                    "Request to {} failed with status {}: {}",
                    url, status, err_msg
                );
                Err(AppError::Internal(format!(
                    "NanoKVM API returned {}: {}",
                    status, err_msg
                )))
            }
        }
    }
}

#[async_trait::async_trait]
impl NanoKvmClient for HttpNanoKvmClient {
    async fn mount_iso(&self, path: &std::path::Path) -> Result<(), AppError> {
        info!("Mounting ISO: {:?}", path);
        // Assuming the nanokvm API expects the path as a string in a JSON payload.
        // We will need to adjust this depending on the exact nanokvm API contract.
        let payload = json!({
            "file": path.to_string_lossy().to_string(),
            "cdrom": true
        });

        self.send_request("/api/storage/image/mount", payload).await
    }

    async fn unmount_iso(&self) -> Result<(), AppError> {
        info!("Unmounting ISO");
        let payload = json!({
            "file": "",
            "cdrom": true
        });
        self.send_request("/api/storage/image/mount", payload).await
    }
}
