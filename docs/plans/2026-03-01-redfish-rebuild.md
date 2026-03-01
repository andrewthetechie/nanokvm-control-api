# NanoKVM Redfish Rebuild Implementation Plan

> **For Antigravity:** REQUIRED WORKFLOW: Use `.agent/workflows/execute-plan.md` to execute this plan in single-flow mode.

**Goal:** Rebuild the NanoKVM Control API as a single-machine Redfish BMC emulator with power control via GPIO relays, virtual media via the NanoKVM API, and boot source override via ISO swapping.

**Architecture:** axum HTTP server with trait-abstracted hardware layers (`PowerController`, `NanoKvmClient`, `VirtualMediaManager`). Mock implementations enable full testing on Mac. CLI subcommands via `clap` for `serve` and `cleanup`. Single-threaded tokio runtime for memory efficiency.

**Tech Stack:** Rust, axum, tokio, clap, serde/serde_json, toml, gpiocdev, reqwest (rustls), tracing, base64

**Design Doc:** `docs/plans/2026-02-28-redfish-rebuild-design.md`

---

### Task 1: Project Scaffold — Replace Dependencies and Set Up Module Structure

**Files:**
- Modify: `Cargo.toml`
- Create: `src/cli.rs`
- Create: `src/config.rs` (overwrite existing)
- Create: `src/error.rs`
- Create: `src/state.rs`
- Create: `src/auth.rs`
- Create: `src/power/mod.rs`
- Create: `src/power/gpio.rs`
- Create: `src/power/mock.rs`
- Create: `src/nanokvm/mod.rs`
- Create: `src/nanokvm/client.rs`
- Create: `src/nanokvm/mock.rs`
- Create: `src/virtual_media/mod.rs`
- Create: `src/virtual_media/manager.rs`
- Create: `src/virtual_media/cleanup.rs`
- Create: `src/virtual_media/mock.rs`
- Create: `src/redfish/mod.rs`
- Create: `src/redfish/models.rs`
- Create: `src/redfish/service_root.rs`
- Create: `src/redfish/systems.rs`
- Create: `src/redfish/managers.rs`
- Create: `src/redfish/virtual_media.rs`
- Modify: `src/main.rs` (overwrite)
- Delete: `src/control.rs`

**Step 1: Update `Cargo.toml` with new dependencies**

Replace all dependencies with:

```toml
[package]
name = "nanokvm-control-api"
version = "0.2.0"
edition = "2021"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["rt", "macros", "time", "fs", "io-util", "net"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
base64 = "0.22"
async-trait = "0.1"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
futures-util = "0.3"
tokio-util = { version = "0.7", features = ["io"] }

[target.'cfg(target_os = "linux")'.dependencies]
gpiocdev = "0.7"
```

Note: `edition = "2021"` not `"2024"` for broad compatibility. `gpiocdev` only compiles on Linux, so it's target-gated.

**Step 2: Create all module directories and stub files**

Create every file listed above with minimal content — just `//! Module description` doc comments and empty trait/struct definitions. Every module should compile.

The key stubs:

`src/main.rs`:
```rust
use clap::Parser;

mod auth;
mod cli;
mod config;
mod error;
mod nanokvm;
mod power;
mod redfish;
mod state;
mod virtual_media;

use cli::{Cli, Commands};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config: config_path } => {
            eprintln!("serve not yet implemented");
            Ok(())
        }
        Commands::Cleanup { config: config_path, dry_run } => {
            eprintln!("cleanup not yet implemented");
            Ok(())
        }
    }
}
```

`src/cli.rs`:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nanokvm-control-api", version, about = "NanoKVM Redfish BMC Emulator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the Redfish API server
    Serve {
        /// Path to config file
        #[arg(short, long, default_value = "/etc/nanokvm/config.toml")]
        config: String,
    },
    /// Run ISO cleanup once and exit
    Cleanup {
        /// Path to config file
        #[arg(short, long, default_value = "/etc/nanokvm/config.toml")]
        config: String,
        /// Show what would be cleaned up without deleting
        #[arg(long)]
        dry_run: bool,
    },
}
```

`src/error.rs`:
```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Internal(String),
    NotFound(String),
    BadRequest(String),
    Unauthorized,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            Self::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
        };
        (status, body).into_response()
    }
}
```

All other module files start as empty stubs with just the module-level doc comment and `pub` re-exports.

**Step 3: Delete old `src/control.rs`**

Remove the old monolithic control file.

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors (warnings are OK at this stage)

**Step 5: Commit**

```bash
git add -A && git commit -m "refactor: scaffold new module structure with axum/clap/trait-based architecture"
```

---

### Task 2: Configuration System

**Files:**
- Create: `src/config.rs` (full implementation)
- Create: `deploy/config.example.toml`

**Step 1: Write the config test**

In `src/config.rs`, add a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_config() {
        let toml_str = include_str!("../deploy/config.example.toml");
        let config: Config = toml::from_str(toml_str).expect("Failed to parse example config");
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.auth.enabled, true);
        assert_eq!(config.gpio.power_button_line, 4);
        assert_eq!(config.virtual_media.cleanup_ttl_hours, 24);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.server.host, "0.0.0.0");
        assert!(config.auth.enabled);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test config::tests -- --nocapture`
Expected: FAIL — `Config` struct doesn't exist yet

**Step 3: Implement the config module**

`src/config.rs`:
```rust
//! Configuration loading from TOML file with defaults.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub gpio: GpioConfig,
    pub nanokvm: NanoKvmConfig,
    pub virtual_media: VirtualMediaConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub state_file: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct GpioConfig {
    pub chip: String,
    pub power_button_line: u32,
    pub power_button_short_press_ms: u64,
    pub power_button_long_press_ms: u64,
    pub hard_power_line: u32,
    pub hard_power_restart_delay_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct NanoKvmConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct VirtualMediaConfig {
    pub download_dir: String,
    pub cleanup_ttl_hours: u64,
    pub pxe_boot_iso: String,
    pub disk_boot_iso: String,
}

// Defaults
impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            gpio: GpioConfig::default(),
            nanokvm: NanoKvmConfig::default(),
            virtual_media: VirtualMediaConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8000,
            log_level: "info".to_string(),
            state_file: "./state.json".to_string(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            username: "admin".to_string(),
            password: "password".to_string(),
        }
    }
}

impl Default for GpioConfig {
    fn default() -> Self {
        Self {
            chip: "/dev/gpiochip0".to_string(),
            power_button_line: 4,
            power_button_short_press_ms: 200,
            power_button_long_press_ms: 6000,
            hard_power_line: 5,
            hard_power_restart_delay_ms: 3000,
        }
    }
}

impl Default for NanoKvmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:80".to_string(),
            username: "admin".to_string(),
            password: "admin".to_string(),
        }
    }
}

impl Default for VirtualMediaConfig {
    fn default() -> Self {
        Self {
            download_dir: "/data/isos".to_string(),
            cleanup_ttl_hours: 24,
            pxe_boot_iso: "/data/boot/pxe.iso".to_string(),
            disk_boot_iso: "/data/boot/disk.iso".to_string(),
        }
    }
}

/// Load config from a TOML file, falling back to defaults for missing fields.
pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    if Path::new(path).exists() {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        tracing::warn!("Config file not found at {path}, using defaults");
        Ok(Config::default())
    }
}
```

`deploy/config.example.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8000
log_level = "info"
state_file = "./state.json"

[auth]
enabled = true
username = "admin"
password = "password"

[gpio]
chip = "/dev/gpiochip0"
power_button_line = 4
power_button_short_press_ms = 200
power_button_long_press_ms = 6000
hard_power_line = 5
hard_power_restart_delay_ms = 3000

[nanokvm]
base_url = "http://127.0.0.1:80"
username = "admin"
password = "admin"

[virtual_media]
download_dir = "/data/isos"
cleanup_ttl_hours = 24
pxe_boot_iso = "/data/boot/pxe.iso"
disk_boot_iso = "/data/boot/disk.iso"
```

**Step 4: Run tests to verify they pass**

Run: `cargo test config::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add TOML config system with defaults and example config"
```

---

### Task 3: System State Manager

**Files:**
- Create: `src/state.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_power_state_is_unknown() {
        let state = SystemState::default();
        assert_eq!(state.power_state, PowerState::Unknown);
    }

    #[test]
    fn test_set_power_state() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::On);
        assert_eq!(manager.get_power_state(), PowerState::On);
    }

    #[test]
    fn test_power_on_transitions_to_on() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::Off);
        manager.apply_power_action(PowerAction::On);
        assert_eq!(manager.get_power_state(), PowerState::On);
    }

    #[test]
    fn test_force_off_transitions_to_off() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::On);
        manager.apply_power_action(PowerAction::ForceOff);
        assert_eq!(manager.get_power_state(), PowerState::Off);
    }

    #[test]
    fn test_graceful_shutdown_transitions_to_off() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::On);
        manager.apply_power_action(PowerAction::GracefulShutdown);
        assert_eq!(manager.get_power_state(), PowerState::Off);
    }

    #[test]
    fn test_force_restart_transitions_to_on() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::On);
        manager.apply_power_action(PowerAction::ForceRestart);
        assert_eq!(manager.get_power_state(), PowerState::On);
    }

    #[test]
    fn test_boot_source_default_is_hdd() {
        let state = SystemState::default();
        assert_eq!(state.boot_source_override_target, BootSourceTarget::Hdd);
        assert_eq!(state.boot_source_override_enabled, BootSourceEnabled::Disabled);
    }

    #[test]
    fn test_set_boot_override_once() {
        let manager = StateManager::new();
        manager.set_boot_override(BootSourceTarget::Pxe, BootSourceEnabled::Once);
        let state = manager.get_state();
        assert_eq!(state.boot_source_override_target, BootSourceTarget::Pxe);
        assert_eq!(state.boot_source_override_enabled, BootSourceEnabled::Once);
    }

    #[test]
    fn test_clear_boot_override_once_after_reset() {
        let manager = StateManager::new();
        manager.set_boot_override(BootSourceTarget::Pxe, BootSourceEnabled::Once);
        manager.consume_boot_override(); // Called after a reset
        let state = manager.get_state();
        assert_eq!(state.boot_source_override_target, BootSourceTarget::Hdd);
        assert_eq!(state.boot_source_override_enabled, BootSourceEnabled::Disabled);
    }

    #[test]
    fn test_continuous_boot_override_not_cleared() {
        let manager = StateManager::new();
        manager.set_boot_override(BootSourceTarget::Pxe, BootSourceEnabled::Continuous);
        manager.consume_boot_override();
        let state = manager.get_state();
        assert_eq!(state.boot_source_override_target, BootSourceTarget::Pxe);
        assert_eq!(state.boot_source_override_enabled, BootSourceEnabled::Continuous);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test state::tests -- --nocapture`
Expected: FAIL

**Step 3: Implement the state module**

Implement `SystemState`, `StateManager`, `PowerState`, `PowerAction`, `BootSourceTarget`, `BootSourceEnabled`, `VirtualMediaState` types. `StateManager` wraps `Arc<RwLock<SystemState>>` for thread-safe access. Include `consume_boot_override()` which clears "Once" overrides after a reset.

**Step 4: Run tests to verify pass**

Run: `cargo test state::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add system state manager with power state tracking and boot overrides"
```

---

### Task 4: Auth Middleware

**Files:**
- Create: `src/auth.rs`

**Step 1: Write the tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_auth_passes_with_valid_credentials() {
        let creds = AuthCredentials {
            username: "admin".to_string(),
            password: "pass".to_string(),
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:pass");
        let result = validate_basic_auth(
            &format!("Basic {encoded}"),
            &creds,
        );
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_fails_with_wrong_password() {
        let creds = AuthCredentials {
            username: "admin".to_string(),
            password: "pass".to_string(),
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:wrong");
        let result = validate_basic_auth(
            &format!("Basic {encoded}"),
            &creds,
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_auth_fails_with_missing_header() {
        let result = validate_basic_auth("", &AuthCredentials {
            username: "admin".to_string(),
            password: "pass".to_string(),
        });
        assert!(result.is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test auth::tests -- --nocapture`
Expected: FAIL

**Step 3: Implement auth middleware**

Implement `AuthCredentials` struct, `validate_basic_auth` function, and an axum middleware layer that checks the `Authorization` header. The middleware should be configurable: when `auth.enabled = false`, it passes all requests through.

**Step 4: Run tests to verify pass**

Run: `cargo test auth::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add basic auth middleware with configurable enable/disable"
```

---

### Task 5: Power Controller Trait and Mock

**Files:**
- Create: `src/power/mod.rs`
- Create: `src/power/mock.rs`

**Step 1: Write the trait and mock with tests**

```rust
// src/power/mod.rs
#[cfg(target_os = "linux")]
pub mod gpio;
pub mod mock;

use async_trait::async_trait;
use crate::error::AppError;

#[async_trait]
pub trait PowerController: Send + Sync {
    async fn power_on(&self) -> Result<(), AppError>;
    async fn power_off_graceful(&self) -> Result<(), AppError>;
    async fn power_off_hard(&self) -> Result<(), AppError>;
    async fn force_restart(&self) -> Result<(), AppError>;
    async fn graceful_restart(&self) -> Result<(), AppError>;
}
```

Test the mock:
```rust
// In src/power/mock.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_power_on() {
        let mock = MockPowerController::new();
        assert!(mock.power_on().await.is_ok());
        assert_eq!(mock.last_action(), Some("power_on".to_string()));
    }

    #[tokio::test]
    async fn test_mock_records_all_actions() {
        let mock = MockPowerController::new();
        mock.power_on().await.unwrap();
        mock.power_off_hard().await.unwrap();
        mock.force_restart().await.unwrap();
        assert_eq!(mock.action_count(), 3);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test power:: -- --nocapture`
Expected: FAIL

**Step 3: Implement MockPowerController**

`MockPowerController` stores actions in `Arc<Mutex<Vec<String>>>`. Each method pushes its name to the vec and returns `Ok(())`.

**Step 4: Run tests to verify pass**

Run: `cargo test power:: -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add PowerController trait and MockPowerController"
```

---

### Task 6: GPIO Power Controller (Linux only)

**Files:**
- Create: `src/power/gpio.rs`

**Step 1: Implement GpioPowerController**

This file is `#[cfg(target_os = "linux")]` gated. It uses `gpiocdev` to control GPIO lines:

- `power_on()`: set power button line high for `short_press_ms`, then low
- `power_off_graceful()`: same as power_on (short press simulates button press)
- `power_off_hard()`: set hard power line to cut power
- `force_restart()`: hard off → sleep `restart_delay_ms` → hard on
- `graceful_restart()`: short press → sleep `restart_delay_ms` → short press

**Step 2: Verify it compiles on Mac (cfg-gated)**

Run: `cargo check`
Expected: Compiles — gpio.rs is skipped on non-Linux targets

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add GpioPowerController for Linux GPIO relay control"
```

Note: No unit test for real GPIO — this is tested via the Docker integration test and on-device smoke tests.

---

### Task 7: NanoKVM Client Trait and Mock

**Files:**
- Create: `src/nanokvm/mod.rs`
- Create: `src/nanokvm/mock.rs`

**Step 1: Write trait and mock with tests**

Define `NanoKvmClient` trait with `mount_image`, `unmount_image`, `get_mounted_image`. `MockNanoKvmClient` stores state in-memory.

Tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_mount_and_get() {
        let mock = MockNanoKvmClient::new();
        mock.mount_image("/data/test.iso").await.unwrap();
        let mounted = mock.get_mounted_image().await.unwrap();
        assert_eq!(mounted, Some("/data/test.iso".to_string()));
    }

    #[tokio::test]
    async fn test_mock_unmount() {
        let mock = MockNanoKvmClient::new();
        mock.mount_image("/data/test.iso").await.unwrap();
        mock.unmount_image().await.unwrap();
        let mounted = mock.get_mounted_image().await.unwrap();
        assert_eq!(mounted, None);
    }
}
```

**Step 2: Run test to verify fail, implement, verify pass**

Run: `cargo test nanokvm:: -- --nocapture`

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add NanoKvmClient trait and MockNanoKvmClient"
```

---

### Task 8: NanoKVM HTTP Client Implementation

**Files:**
- Create: `src/nanokvm/client.rs`

**Step 1: Implement HttpNanoKvmClient**

Uses `reqwest` to call the NanoKVM API:
- `POST /api/storage/image/mount` with body `{"file": "<path>"}`
- Login via `/api/auth/login` to get a session cookie
- `get_mounted_image` via `/api/storage/image/status` (or equivalent)

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add HttpNanoKvmClient for NanoKVM API integration"
```

Note: Integration tested in the Docker test suite, not unit tested (requires real NanoKVM API).

---

### Task 9: Virtual Media Manager

**Files:**
- Create: `src/virtual_media/mod.rs`
- Create: `src/virtual_media/manager.rs`
- Create: `src/virtual_media/mock.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_insert_and_status() {
        let mock = MockVirtualMediaManager::new();
        mock.insert_media("http://example.com/test.iso").await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(status.inserted);
        assert_eq!(status.image_name, Some("test.iso".to_string()));
    }

    #[tokio::test]
    async fn test_mock_eject_restores_default() {
        let mock = MockVirtualMediaManager::new();
        mock.insert_media("http://example.com/test.iso").await.unwrap();
        mock.eject_media().await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(status.inserted); // disk-boot ISO should be mounted
    }

    #[tokio::test]
    async fn test_mount_boot_iso_pxe() {
        let mock = MockVirtualMediaManager::new();
        mock.mount_boot_iso(BootSourceTarget::Pxe).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert_eq!(status.image_name, Some("pxe.iso".to_string()));
    }

    #[tokio::test]
    async fn test_mount_boot_iso_hdd() {
        let mock = MockVirtualMediaManager::new();
        mock.mount_boot_iso(BootSourceTarget::Hdd).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert_eq!(status.image_name, Some("disk.iso".to_string()));
    }
}
```

**Step 2: Implement trait and mock**

`VirtualMediaManager` trait adds `mount_boot_iso(target: BootSourceTarget)` alongside `insert_media`/`eject_media`/`get_status`.

`RealVirtualMediaManager`:
- `insert_media`: Download ISO from URL via streaming reqwest → file, write `.meta.json` with timestamp, call `nanokvm.mount_image(path)`
- `eject_media`: Call `nanokvm.unmount_image()`, then mount disk-boot ISO
- `mount_boot_iso(Pxe)`: Mount configured PXE ISO path
- `mount_boot_iso(Hdd)`: Mount configured disk-boot ISO path

**Step 3: Run tests, verify pass**

Run: `cargo test virtual_media:: -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add VirtualMediaManager trait with download, mount, and boot ISO support"
```

---

### Task 10: ISO Cleanup Subcommand

**Files:**
- Create: `src/virtual_media/cleanup.rs`
- Modify: `src/main.rs` (wire up cleanup command)

**Step 1: Write cleanup tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_identifies_expired_isos() {
        let dir = tempfile::tempdir().unwrap();
        let iso_path = dir.path().join("old.iso");
        let meta_path = dir.path().join("old.iso.meta.json");
        fs::write(&iso_path, b"fake iso").unwrap();
        // Write metadata with timestamp 48 hours ago
        let old_ts = chrono::Utc::now() - chrono::Duration::hours(48);
        let meta = serde_json::json!({"last_used": old_ts.to_rfc3339()});
        fs::write(&meta_path, meta.to_string()).unwrap();

        let expired = find_expired_isos(dir.path(), 24, &[]);
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_skips_permanent_isos() {
        let dir = tempfile::tempdir().unwrap();
        let pxe_path = dir.path().join("pxe.iso");
        fs::write(&pxe_path, b"pxe").unwrap();

        let expired = find_expired_isos(
            dir.path(), 0, // TTL of 0 = everything expired
            &[pxe_path.to_str().unwrap()],
        );
        assert!(expired.is_empty()); // pxe.iso is permanent
    }
}
```

Note: Add `tempfile` and `chrono` as dev-dependencies in `Cargo.toml`.

**Step 2: Implement cleanup logic**

`find_expired_isos(dir, ttl_hours, permanent_paths)` scans the directory for `.iso` files, reads their `.meta.json` for `last_used` timestamp, and returns those older than TTL. `run_cleanup(dir, ttl_hours, permanent_paths, dry_run)` deletes (or prints) the expired ISOs.

**Step 3: Wire up the cleanup subcommand in `main.rs`**

**Step 4: Run tests, verify pass**

Run: `cargo test virtual_media::cleanup::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add ISO cleanup subcommand with TTL-based expiration"
```

---

### Task 11: Redfish Models — JSON Schema Types

**Files:**
- Create: `src/redfish/models.rs`

**Step 1: Write tests for serialization**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_root_serializes_correctly() {
        let root = ServiceRoot::default();
        let json = serde_json::to_value(&root).unwrap();
        assert_eq!(json["@odata.id"], "/redfish/v1/");
        assert_eq!(json["@odata.type"], "#ServiceRoot.v1_0_0.ServiceRoot");
        assert!(json["Systems"].is_object());
        assert!(json["Managers"].is_object());
    }

    #[test]
    fn test_computer_system_includes_power_state() {
        let sys = ComputerSystem::new(PowerState::On, BootOverride::default());
        let json = serde_json::to_value(&sys).unwrap();
        assert_eq!(json["PowerState"], "On");
        assert_eq!(json["@odata.type"], "#ComputerSystem.v1_0_0.ComputerSystem");
    }

    #[test]
    fn test_reset_type_deserializes() {
        let input = r#"{"ResetType": "ForceOff"}"#;
        let req: ResetRequest = serde_json::from_str(input).unwrap();
        assert_eq!(req.reset_type, ResetType::ForceOff);
    }
}
```

**Step 2: Implement all Redfish model structs**

`ServiceRoot`, `ComputerSystem`, `ComputerSystemCollection`, `Manager`, `ManagerCollection`, `VirtualMediaResource`, `VirtualMediaCollection`, `ResetRequest`, `ResetType`, `InsertMediaRequest`, `BootOverride`, `ODataLink`, etc.

All with proper `@odata.id`, `@odata.type`, `Id`, `Name` fields using `#[serde(rename = "@odata.id")]` etc.

**Step 3: Run tests, verify pass**

Run: `cargo test redfish::models::tests -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Redfish JSON schema model types with OData annotations"
```

---

### Task 12: Redfish Handlers — Service Root and Collections

**Files:**
- Create: `src/redfish/mod.rs`
- Create: `src/redfish/service_root.rs`

**Step 1: Write handler tests**

Use axum's test utilities to test the handler directly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_service_root_returns_200() {
        let app = create_test_router();
        let response = app
            .oneshot(Request::get("/redfish/v1/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_service_root_has_odata_fields() {
        let app = create_test_router();
        let response = app
            .oneshot(Request::get("/redfish/v1/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["@odata.id"], "/redfish/v1/");
    }
}
```

**Step 2: Implement service root handler and router**

`GET /redfish/v1/` returns `ServiceRoot` JSON. Set up the axum router with shared `AppState` containing `Arc<dyn PowerController>`, `Arc<dyn VirtualMediaManager>`, `Arc<StateManager>`.

**Step 3: Run tests, verify pass**

Run: `cargo test redfish:: -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Redfish service root and router setup"
```

---

### Task 13: Redfish Handlers — Systems (Power Control + Boot Override)

**Files:**
- Create: `src/redfish/systems.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_system_returns_power_state() {
        let app = create_test_router_with_state(PowerState::On);
        let response = app
            .oneshot(Request::get("/redfish/v1/Systems/1").body(Body::empty()).unwrap())
            .await.unwrap();
        let json = parse_json_body(response).await;
        assert_eq!(json["PowerState"], "On");
    }

    #[tokio::test]
    async fn test_reset_force_off() {
        let app = create_test_router_with_state(PowerState::On);
        let response = app.oneshot(
            Request::post("/redfish/v1/Systems/1/Actions/ComputerSystem.Reset")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"ResetType": "ForceOff"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_patch_boot_override() {
        let app = create_test_router();
        let response = app.oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/redfish/v1/Systems/1")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"Boot":{"BootSourceOverrideTarget":"Pxe","BootSourceOverrideEnabled":"Once"}}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

**Step 2: Implement Systems handlers**

- `GET /redfish/v1/Systems` — returns collection with one system
- `GET /redfish/v1/Systems/1` — returns `ComputerSystem` with current `PowerState` and `Boot` from state manager
- `PATCH /redfish/v1/Systems/1` — updates boot source override, triggers ISO swap via `VirtualMediaManager::mount_boot_iso`
- `POST .../ComputerSystem.Reset` — dispatches to `PowerController` method based on `ResetType`, updates state via `StateManager::apply_power_action`, handles "Once" boot override consumption

**Step 3: Run tests, verify pass**

Run: `cargo test redfish::systems::tests -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Redfish Systems endpoints with power control and boot override"
```

---

### Task 14: Redfish Handlers — Managers and Virtual Media

**Files:**
- Create: `src/redfish/managers.rs`
- Create: `src/redfish/virtual_media.rs`

**Step 1: Write tests**

```rust
// managers tests
#[tokio::test]
async fn test_get_managers_collection() { /* returns 200 with one manager */ }

#[tokio::test]
async fn test_get_manager() { /* returns 200 with VirtualMedia link */ }

// virtual_media tests
#[tokio::test]
async fn test_insert_media() {
    // POST InsertMedia with {"Image": "http://example.com/test.iso"}
    // Returns 204
}

#[tokio::test]
async fn test_eject_media() {
    // POST EjectMedia returns 204
}

#[tokio::test]
async fn test_get_virtual_media_status() {
    // GET VirtualMedia/1 returns current status
}
```

**Step 2: Implement handlers**

- `GET /redfish/v1/Managers` — collection
- `GET /redfish/v1/Managers/1` — manager with `VirtualMedia` link
- `GET /redfish/v1/Managers/1/VirtualMedia` — collection
- `GET /redfish/v1/Managers/1/VirtualMedia/1` — status
- `POST .../InsertMedia` — calls `virtual_media.insert_media(url)`
- `POST .../EjectMedia` — calls `virtual_media.eject_media()`

**Step 3: Run tests, verify pass**

Run: `cargo test redfish:: -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Redfish Managers and VirtualMedia endpoints"
```

---

### Task 15: Non-Redfish Management Endpoints (Power State)

**Files:**
- Modify: `src/redfish/mod.rs` (add management routes)

**Step 1: Write tests**

```rust
#[tokio::test]
async fn test_get_power_state_returns_unknown_initially() {
    let app = create_test_router();
    let resp = app.oneshot(Request::get("/api/v1/power-state").body(Body::empty()).unwrap()).await.unwrap();
    let json = parse_json_body(resp).await;
    assert_eq!(json["state"], "Unknown");
}

#[tokio::test]
async fn test_set_power_state() {
    let app = create_test_router();
    let resp = app.clone().oneshot(
        Request::put("/api/v1/power-state")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"state": "On"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
```

**Step 2: Implement, run tests, verify pass**

Run: `cargo test -- power_state --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add power-state management endpoints"
```

---

### Task 16: Wire Everything Together — `main.rs` Server Setup

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement the `serve` subcommand**

Wire up:
1. Load config from TOML
2. Initialize tracing subscriber
3. Create `StateManager`
4. Create `PowerController` (GPIO on Linux, Mock otherwise — or configurable)
5. Create `NanoKvmClient`
6. Create `VirtualMediaManager`
7. Mount disk-boot ISO on startup (default state)
8. Build axum router with all Redfish routes + management routes
9. Apply auth middleware if enabled
10. Start server on configured host:port

**Step 2: Verify it compiles and starts**

Run: `cargo run -- serve --config deploy/config.example.toml`
Expected: Server starts, logs "listening on 0.0.0.0:8000"

Test with curl: `curl http://localhost:8000/redfish/v1/`
Expected: Returns ServiceRoot JSON

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: wire up serve command with full router and dependency injection"
```

---

### Task 17: systemd Unit Files

**Files:**
- Create: `deploy/nanokvm-control-api.service`
- Create: `deploy/nanokvm-cleanup.service`
- Create: `deploy/nanokvm-cleanup.timer`

**Step 1: Write systemd units**

`deploy/nanokvm-control-api.service`:
```ini
[Unit]
Description=NanoKVM Redfish Control API
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/nanokvm-control-api serve --config /etc/nanokvm/config.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

`deploy/nanokvm-cleanup.service`:
```ini
[Unit]
Description=NanoKVM ISO Cleanup

[Service]
Type=oneshot
ExecStart=/usr/local/bin/nanokvm-control-api cleanup --config /etc/nanokvm/config.toml
```

`deploy/nanokvm-cleanup.timer`:
```ini
[Unit]
Description=Run NanoKVM ISO cleanup hourly

[Timer]
OnCalendar=hourly
Persistent=true

[Install]
WantedBy=timers.target
```

**Step 2: Commit**

```bash
git add -A && git commit -m "feat: add systemd service and timer units"
```

---

### Task 18: Update Makefile

**Files:**
- Modify: `Makefile`

**Step 1: Update Makefile with new targets**

```makefile
default: build

# Local development
check:
	cargo check

test:
	cargo test

lint:
	cargo fmt --check
	cargo clippy -- -D warnings

fmt:
	cargo fmt

# Cross-compile for NanoKVM
build:
	cargo +nightly zigbuild -Z build-std=std,panic_abort --target riscv64gc-unknown-linux-musl --release

# Docker integration tests
test-integration:
	docker compose -f tests/docker-compose.yml up --build --abort-on-container-exit --exit-code-from test-runner

# Upload to device
upload-to-kvm: build
	@if [ -z "$$KVM_IP" ]; then read -p "KVM IP: " KVM_IP; fi; \
	rsync -avz --progress -e ssh \
		target/riscv64gc-unknown-linux-musl/release/nanokvm-control-api \
		"root@$$KVM_IP:~/"
```

**Step 2: Verify local targets work**

Run: `make check && make test && make lint`
Expected: All pass

**Step 3: Commit**

```bash
git add -A && git commit -m "build: update Makefile with new dev targets"
```

---

### Task 19: GitHub Actions Workflows

**Files:**
- Modify: `.github/workflows/lint.yaml`
- Modify: `.github/workflows/test.yaml`
- Modify: `.github/workflows/build.yaml`
- Create: `.github/workflows/integration.yaml`

**Step 1: Update lint.yaml**

Add `cargo fmt --check` step alongside existing clippy.

**Step 2: Update test.yaml**

Keep as-is (already runs `cargo test`).

**Step 3: Create integration.yaml**

```yaml
name: Integration Tests

on:
  push:
    branches: ['**']

jobs:
  integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        run: docker compose -f tests/docker-compose.yml up --build --abort-on-container-exit --exit-code-from test-runner
```

**Step 4: Commit**

```bash
git add -A && git commit -m "ci: update workflows and add integration test pipeline"
```

---

### Task 20: Docker Integration Test Suite

**Files:**
- Create: `tests/docker-compose.yml`
- Create: `tests/integration/Dockerfile.api`
- Create: `tests/integration/Dockerfile.test`
- Create: `tests/integration/config.test.toml`
- Create: `tests/integration/test_redfish.py`
- Create: `tests/integration/requirements.txt`

**Step 1: Write Docker Compose setup**

`tests/docker-compose.yml`:
```yaml
services:
  api:
    build:
      context: ..
      dockerfile: tests/integration/Dockerfile.api
    ports:
      - "8000:8000"
    volumes:
      - ./integration/config.test.toml:/etc/nanokvm/config.toml

  test-runner:
    build:
      context: .
      dockerfile: integration/Dockerfile.test
    depends_on:
      - api
    environment:
      - API_URL=http://api:8000
```

**Step 2: Write test config (auth disabled, mock GPIO)**

`tests/integration/config.test.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8000

[auth]
enabled = false
```

**Step 3: Write the Python test suite**

`tests/integration/test_redfish.py`:

Test the full provisioning workflow:
1. Service root responds with proper OData
2. Systems collection has one system
3. System resource has PowerState
4. Set power state via management endpoint
5. Execute each ResetType and verify state transitions
6. Insert virtual media — verify 204
7. Get virtual media status — verify inserted
8. Eject virtual media — verify ejected
9. Set boot override to Pxe — verify OK
10. Reset with Once override — verify it clears after
11. Test auth enforcement (separate test with auth enabled config)

Use `requests` library (simpler than redfishtool for programmatic testing), with a few `redfishtool` CLI calls to verify CLI compatibility.

**Step 4: Run locally**

Run: `make test-integration`
Expected: All tests pass

**Step 5: Commit**

```bash
git add -A && git commit -m "test: add Docker-based Redfish integration test suite"
```

---

### Task 21: Final Verification and README

**Files:**
- Modify: `README.md`

**Step 1: Run full test suite**

```bash
make lint
make test
make test-integration
make build
```

Expected: All pass

**Step 2: Update README**

Update with new project description, config example, usage instructions, development setup, and testing instructions.

**Step 3: Commit**

```bash
git add -A && git commit -m "docs: update README for Redfish rebuild"
```

---

## Task Dependency Order

```
Task 1 (scaffold) → Task 2 (config) → Task 3 (state) → Task 4 (auth)
                                                              ↓
Task 5 (power trait) → Task 6 (GPIO) ────────────────→ Task 12 (service root)
Task 7 (nanokvm trait) → Task 8 (HTTP client) ───────→ Task 13 (systems)
Task 9 (vmedia) → Task 10 (cleanup) ─────────────────→ Task 14 (managers/vmedia)
                                                              ↓
                                                     Task 15 (mgmt endpoints)
                                                              ↓
                                                     Task 16 (wire up main.rs)
                                                              ↓
Task 17 (systemd) ──→ Task 18 (Makefile) ──→ Task 19 (CI) ──→ Task 20 (integration)
                                                              ↓
                                                     Task 21 (verify + README)
```
