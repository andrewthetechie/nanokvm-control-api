# NanoKVM Control API — Redfish Rebuild Design

## Problem

Rebuild the NanoKVM Control API from a multi-machine I2C-based KVM controller into a single-machine BMC emulator with Redfish-compatible endpoints. The goal is to integrate these NanoKVM-powered systems with automated provisioning tools (MaaS, Metal3, Sidero Omni) that expect a standard Redfish BMC interface.

## Constraints

- **Target hardware:** Sipeed NanoKVM (RISC-V, Debian firmware, minimal RAM)
- **Must be memory/CPU efficient** — single-threaded tokio runtime, no unnecessary allocations
- **Statically compiled** — musl target, no external runtime dependencies
- **Cross-compiled from Mac** — via `cargo zigbuild` for `riscv64gc-unknown-linux-musl`
- **Testable on Mac** — hardware interactions abstracted behind traits with mock implementations

## Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| HTTP framework | axum | Rich routing/middleware, we already need tokio |
| Virtual media | NanoKVM API | Already running, avoids reinventing USB gadget management |
| Redfish scope | MVP surface | Service Root, Systems, Managers, VirtualMedia, Power, Boot |
| Auth | Basic HTTP (toggleable) | What MaaS/Metal3 expect; disable for testing |
| GPIO | `gpiocdev` crate | Modern chardev API, pure Rust, works on any Linux |
| Power relays | 2x GPIO-controlled 3.3V relays | One for power button (short/long press), one for hard power cut |
| Special boot ISOs | Configurable paths | User-supplied PXE and disk-boot ISOs |
| ISO cleanup | TTL-based | Configurable idle period, exempt for special boot ISOs |

---

## Approach Comparison

### Approach A: Trait-Abstracted Modules (Recommended)

Clean separation between hardware and business logic via Rust traits:

- `PowerController` trait → `GpioPowerController` (real) / `MockPowerController` (test)
- `NanoKvmClient` trait → `HttpNanoKvmClient` (real) / `MockNanoKvmClient` (test)
- `VirtualMediaManager` orchestrates ISO download, NanoKVM mount/unmount, cleanup

**Pros:** Testable on Mac without hardware, clean module boundaries, idiomatic Rust.
**Cons:** Slightly more upfront boilerplate.

### Approach B: Direct Implementation, Feature-Gated

Use `#[cfg(feature = "hardware")]` to gate GPIO/NanoKVM code, with stub implementations when the feature is off.

**Pros:** Less code, simpler.
**Cons:** Feature flags are harder to test exhaustively, less flexible for integration testing, can't easily test real logic paths with mock hardware.

### Recommendation: Approach A

Trait-based abstraction is the Rust-idiomatic way and makes the code genuinely testable. The "boilerplate" is minimal — a few trait definitions and mock structs.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  axum HTTP Server                │
│  ┌───────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Redfish   │ │ Auth     │ │ Error          │  │
│  │ Router    │ │ Middleware│ │ Handling       │  │
│  └─────┬─────┘ └──────────┘ └────────────────┘  │
│        │                                         │
│  ┌─────▼──────────────────────────────────────┐  │
│  │            Redfish Handlers                 │  │
│  │  ServiceRoot │ Systems │ Managers │ VMdia   │  │
│  └─────┬────────┴────┬────┴────┬─────┴────────┘  │
│        │             │         │                  │
├────────┼─────────────┼─────────┼──────────────────┤
│  ┌─────▼─────┐ ┌─────▼─────┐ ┌▼───────────────┐  │
│  │ State     │ │ Power     │ │ VirtualMedia   │  │
│  │ Manager   │ │ Controller│ │ Manager        │  │
│  │ (in-mem)  │ │ (trait)   │ │ (trait-based)  │  │
│  └───────────┘ └─────┬─────┘ └───┬─────┬──────┘  │
│                      │           │     │          │
├──────────────────────┼───────────┼─────┼──────────┤
│  Hardware Layer      │           │     │          │
│  ┌───────────────────▼──┐  ┌────▼─────▼───────┐  │
│  │ GPIO (gpiocdev)      │  │ NanoKVM HTTP API  │  │
│  │ /dev/gpiochipN       │  │ ISO Download      │  │
│  └──────────────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Module Breakdown

### `config` — Configuration
- TOML config file (upgrade from env-only) with env var overrides
- Sections: server, auth, gpio, nanokvm, virtual_media, power
- Config struct with serde `Deserialize`

### `state` — System State
- In-memory state: power state, current boot source override, mounted virtual media
- Thread-safe via `Arc<RwLock<_>>`
- Optional persistence to JSON file for crash recovery
- **Power state tracking:**
  - Power state can be set externally via a dedicated API endpoint (e.g., `PUT /api/v1/power-state` with `{"state": "On"}`) — this doesn't control power, it tells the API what the actual state is
  - Once set, power state is updated optimistically based on power control actions (all actions assumed successful)
  - State transitions: `On` after power-on actions, `Off` after power-off actions
  - Designed to be eventually replaced/supplemented by an external power state monitoring system

### `power` — Power Control
```rust
#[async_trait]
pub trait PowerController: Send + Sync {
    async fn power_on(&self) -> Result<()>;           // Short press power button relay
    async fn power_off_graceful(&self) -> Result<()>;  // Short press power button relay
    async fn power_off_hard(&self) -> Result<()>;      // Toggle hard power relay
    async fn force_restart(&self) -> Result<()>;       // Hard off, delay, hard on
}
```
- `GpioPowerController`: configurable chip/line numbers, press durations
- `MockPowerController`: logs actions, returns Ok

### `virtual_media` — Virtual Media Management
```rust
#[async_trait]
pub trait VirtualMediaManager: Send + Sync {
    async fn insert_media(&self, image_url: &str) -> Result<()>;
    async fn eject_media(&self) -> Result<()>;
    async fn get_status(&self) -> Result<VirtualMediaStatus>;
}
```
- Downloads ISO from HTTP/HTTPS URL to configurable local path
- Streams download to avoid memory spikes
- Calls NanoKVM API to mount/unmount
- Tracks last-used timestamps (written to metadata file alongside each ISO)
- Special boot ISOs (PXE, disk) are permanent, not subject to cleanup

### `nanokvm` — NanoKVM API Client
```rust
#[async_trait]
pub trait NanoKvmClient: Send + Sync {
    async fn mount_image(&self, path: &str) -> Result<()>;
    async fn unmount_image(&self) -> Result<()>;
    async fn get_mounted_image(&self) -> Result<Option<String>>;
}
```
- HTTP client wrapping the NanoKVM's `/api/storage/image/mount` etc.
- Configurable base URL and auth

### `redfish` — Redfish API Handlers

MVP endpoints:

| Method | Path | Purpose |
|---|---|---|
| GET | `/redfish/v1/` | Service Root |
| GET | `/redfish/v1/Systems` | Systems collection |
| GET | `/redfish/v1/Systems/1` | System resource (PowerState, Boot) |
| PATCH | `/redfish/v1/Systems/1` | Set BootSourceOverrideTarget |
| POST | `/redfish/v1/Systems/1/Actions/ComputerSystem.Reset` | Power control |
| GET | `/redfish/v1/Managers` | Managers collection |
| GET | `/redfish/v1/Managers/1` | Manager resource |
| GET | `/redfish/v1/Managers/1/VirtualMedia` | Virtual media collection |
| GET | `/redfish/v1/Managers/1/VirtualMedia/1` | Virtual media resource |
| POST | `/redfish/v1/Managers/1/VirtualMedia/1/Actions/VirtualMedia.InsertMedia` | Mount ISO |
| POST | `/redfish/v1/Managers/1/VirtualMedia/1/Actions/VirtualMedia.EjectMedia` | Unmount ISO |

**Non-Redfish management endpoints:**

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/power-state` | Get current tracked power state |
| PUT | `/api/v1/power-state` | Externally set power state (`{"state": "On"}` or `{"state": "Off"}`) |

All responses use Redfish JSON schema with `@odata.id`, `@odata.type`, etc.

### `auth` — Authentication Middleware
- axum middleware layer
- Checks `Authorization: Basic <base64>` header
- Username/password from config
- Configurable enable/disable flag
- Returns 401 with `WWW-Authenticate: Basic` when auth fails

---

## Config File Format (TOML)

```toml
[server]
host = "0.0.0.0"
port = 8000
log_level = "info"

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
# NanoKVM auth cookie/credentials if needed
username = "admin"
password = "admin"

[virtual_media]
download_dir = "/data/isos"
cleanup_ttl_hours = 24
pxe_boot_iso = "/data/boot/pxe.iso"
disk_boot_iso = "/data/boot/disk.iso"
```

---

## Crate Dependencies

| Crate | Purpose | Notes |
|---|---|---|
| `clap` | CLI parsing | Subcommands (serve, cleanup), `derive` feature |
| `axum` | HTTP framework | Routing, middleware, extractors |
| `tokio` | Async runtime | `rt`, `macros`, `time`, `fs` features; single-threaded |
| `serde` / `serde_json` | Serialization | Redfish JSON responses |
| `toml` | Config parsing | TOML config file |
| `gpiocdev` | GPIO control | Linux chardev API, feature-gated |
| `reqwest` | HTTP client | NanoKVM API calls, ISO downloads; rustls for TLS |
| `tracing` / `tracing-subscriber` | Logging | Structured logging, replaces flexi_logger |
| `base64` | Auth decoding | Basic auth header parsing |
| `uuid` | OData IDs | Redfish resource identifiers (optional) |

**Removed from current:** `pcf857x`, `linux-embedded-hal`, `tiny_http`, `flexi_logger`, `log`

---

## Project Structure

```
src/
├── main.rs              # Entry point, runtime setup, server start
├── cli.rs               # CLI subcommands (serve, cleanup)
├── config.rs            # TOML + env config loading
├── state.rs             # In-memory system state
├── auth.rs              # Basic auth middleware
├── error.rs             # Unified error types
├── power/
│   ├── mod.rs           # PowerController trait
│   ├── gpio.rs          # GpioPowerController implementation
│   └── mock.rs          # MockPowerController for testing
├── virtual_media/
│   ├── mod.rs           # VirtualMediaManager trait
│   ├── manager.rs       # Real implementation (download + NanoKVM)
│   ├── cleanup.rs       # Standalone cleanup logic (used by CLI subcommand)
│   └── mock.rs          # Mock for testing
├── nanokvm/
│   ├── mod.rs           # NanoKvmClient trait
│   ├── client.rs        # HTTP client implementation
│   └── mock.rs          # Mock for testing
└── redfish/
    ├── mod.rs           # Router setup
    ├── models.rs        # Redfish JSON schema types
    ├── service_root.rs  # GET /redfish/v1/
    ├── systems.rs       # Systems endpoints
    ├── managers.rs      # Managers endpoints
    └── virtual_media.rs # VirtualMedia endpoints

deploy/
├── nanokvm-control-api.service   # systemd service unit
├── nanokvm-cleanup.service       # systemd oneshot unit for ISO cleanup
├── nanokvm-cleanup.timer         # systemd timer to trigger cleanup
└── config.example.toml           # Example config file
```

## CLI Subcommands

The binary uses subcommands (via `clap`) to serve both the API server and the cleanup task:

```
nanokvm-control-api serve              # Run the Redfish API server
nanokvm-control-api cleanup            # Run ISO cleanup once and exit
nanokvm-control-api cleanup --dry-run  # Show what would be cleaned up
```

The cleanup subcommand:
- Reads the same TOML config to find `download_dir` and `cleanup_ttl_hours`
- Scans download dir for ISOs with last-used timestamps older than the TTL
- Skips the configured PXE and disk-boot ISOs (permanent)
- Deletes expired ISOs and their metadata files
- Exits immediately — no daemon, no background thread

Triggered by a systemd timer (e.g., every hour).

## systemd Integration

**Service unit** (`nanokvm-control-api.service`):
- Runs `nanokvm-control-api serve --config /etc/nanokvm/config.toml`
- `Type=simple`, `Restart=on-failure`
- Runs as a dedicated service user (or root for GPIO access)

**Cleanup timer** (`nanokvm-cleanup.timer`):
- Triggers `nanokvm-cleanup.service` on a configurable interval (default: hourly)
- `nanokvm-cleanup.service` is a oneshot that runs `nanokvm-control-api cleanup`

---

## Testing Strategy

### Unit Tests (run on Mac, `cargo test`)
- All trait implementations get unit tests
- Mock implementations used to test Redfish handlers without hardware
- Config parsing tests
- Auth middleware tests
- Power state tracking / state transition tests
- ISO metadata and cleanup logic tests
- `cargo test` works on Mac with no hardware dependencies

### Docker Integration Tests (`make test-integration`)

A Docker Compose setup runs the API server with mock hardware, then executes a real Redfish client test suite against it:

```
tests/
├── docker-compose.yml       # API server + test runner containers
├── integration/
│   ├── Dockerfile.api       # Builds API for x86_64-linux with mock GPIO
│   ├── Dockerfile.test      # Python + redfishtool test runner
│   ├── test_redfish.py      # Pytest suite using redfishtool/requests
│   └── config.test.toml     # Test config (mock GPIO, no auth)
```

**How it works:**
1. `Dockerfile.api` builds the API binary targeting x86_64-linux-musl (not riscv — this runs on the dev machine) with `MockPowerController` and `MockNanoKvmClient`
2. `docker-compose.yml` starts the API server container
3. `Dockerfile.test` installs `redfishtool` (DMTF's official Python CLI) and `pytest`
4. `test_redfish.py` runs the full provisioning workflow against the live API:
   - `GET /redfish/v1/` — verify service root
   - `GET /redfish/v1/Systems/1` — verify system resource schema
   - `PUT /api/v1/power-state` — seed power state to "On"
   - `POST .../ComputerSystem.Reset` with each ResetType — verify state transitions
   - `POST .../VirtualMedia.InsertMedia` — verify media insertion
   - `POST .../VirtualMedia.EjectMedia` — verify media ejection
   - `PATCH /redfish/v1/Systems/1` — set boot source override
   - Verify correct `@odata.id`, `@odata.type`, response codes
   - Test auth enforcement when enabled

**Run locally:** `make test-integration` (requires Docker)

### On-Device Smoke Tests
- Upload via `make upload-to-kvm` and run on real hardware
- Manual verification with `curl` or `redfishtool` pointed at the device

---

## CI/CD (GitHub Actions)

The existing `.github/workflows/` has `lint.yaml`, `test.yaml`, and `build.yaml`. These will be updated for the new project:

### `lint.yaml` — Formatting & Linting
- `cargo fmt --check` — enforce consistent formatting
- `cargo clippy -- -D warnings` — catch lint issues
- Runs on every push/PR

### `test.yaml` — Unit Tests
- `cargo test` — runs all unit tests with mock implementations
- Runs on every push/PR

### `integration.yaml` — Docker Integration Tests (new)
- Builds the API binary for x86_64-linux
- Runs `docker compose` to start API + redfishtool test runner
- Executes the full `test_redfish.py` pytest suite
- Runs on every push/PR (uses GitHub Actions Docker support)

### `build.yaml` — Cross-Compile Release Binary
- Cross-compiles for `riscv64gc-unknown-linux-musl` using `actions-rust-cross`
- Uploads binary artifact
- Runs on every push/PR

---

## Boot Source Override Flow

**Key concept:** The target machine's BIOS is permanently configured to boot from USB (the NanoKVM's virtual media). The NanoKVM always has an ISO mounted — there is never a "no ISO" state. The API controls which ISO is mounted, effectively acting as an external boot device controller.

**Default state:** The disk-boot ISO is always mounted. This small ISO chainloads to the first hard disk, making the machine boot normally as if USB wasn't involved.

**Flow when a provisioning system requests PXE boot:**

1. `PATCH /redfish/v1/Systems/1` with `{"Boot": {"BootSourceOverrideTarget": "Pxe", "BootSourceOverrideEnabled": "Once"}}`
2. API swaps the mounted ISO from disk-boot to the PXE boot ISO via NanoKVM API
3. On next `ComputerSystem.Reset` (power cycle), system boots from USB → PXE ISO → network boot
4. If `BootSourceOverrideEnabled: "Once"`, after the reset completes, the API swaps back to the disk-boot ISO (default state)
5. If `BootSourceOverrideEnabled: "Continuous"`, the PXE ISO stays mounted until explicitly changed

**Flow when a provisioning system requests Cd boot (custom ISO via VirtualMedia):**

1. `InsertMedia` mounts a custom ISO (downloaded from URL)
2. `PATCH /redfish/v1/Systems/1` with `{"Boot": {"BootSourceOverrideTarget": "Cd"}}`
3. The custom ISO is already mounted, system boots from it on next reset
4. When done, `EjectMedia` removes the custom ISO and the disk-boot ISO is restored

**Boot target → ISO mapping:**

| BootSourceOverrideTarget | Mounted ISO |
|---|---|
| `Hdd` (default) | disk-boot ISO |
| `Pxe` | PXE boot ISO |
| `Cd` | Currently inserted virtual media ISO |

---

## Power Control Flow

| Redfish ResetType | Action | State After |
|---|---|---|
| `On` | Short press power button relay (if currently off) | `On` |
| `GracefulShutdown` | Short press power button relay (if currently on) | `Off` |
| `ForceOff` | Toggle hard power relay off | `Off` |
| `ForceRestart` | Hard power off → delay → hard power on | `On` |
| `GracefulRestart` | Short press (off) → delay → short press (on) | `On` |

GPIO relay behavior:
- **Power button relay:** Close circuit for configurable duration (200ms short, 6s long), then open
- **Hard power relay:** Open circuit to cut power, close to restore

### Power State Tracking

- Power state is reported in `GET /redfish/v1/Systems/1` as `PowerState` ("On" or "Off")
- State starts as `Unknown` until explicitly set via `PUT /api/v1/power-state`
- After initial set, state transitions optimistically based on power actions (see table above)
- All power actions are assumed successful — no hardware feedback loop (yet)
- Future: an external monitoring system will provide real power state updates

---

## Memory/CPU Considerations

- Single-threaded tokio runtime (`#[tokio::main(flavor = "current_thread")]`)
- Stream ISO downloads to disk, never buffer entire ISO in memory
- Minimal cloning — use `Arc` for shared state, `&str` over `String` where possible
- No background tasks in the server process — fully event-driven
- ISO cleanup runs as a separate short-lived process via systemd timer
