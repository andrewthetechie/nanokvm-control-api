//! HTTP client implementation

use super::NanoKvmClient;
use crate::config::NanoKvmConfig;
use crate::error::AppError;
use reqwest::{Client, StatusCode};
use tracing::{debug, error};

#[allow(dead_code)]
pub struct HttpNanoKvmClient {
    client: Client,
    base_url: String,
    auth_token: String,
}

#[allow(dead_code)]
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
impl NanoKvmClient for HttpNanoKvmClient {}
