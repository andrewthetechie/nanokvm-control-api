pub mod client;
pub mod mock;

#[async_trait::async_trait]
pub trait NanoKvmClient: Send + Sync {
    // Left empty for future NanoKVM API features (e.g. video streams, etc).
    // ISO mounting has been moved to MediaController.
}
