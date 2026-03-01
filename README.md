# nanokvm-control-api

A Rust-based API to run on a NanoKVM Lite (or similar device) to emulate a minimal Redfish BMC. It allows controlling power state via GPIO and managing virtual media for OS installation and recovery.

## Features

- **Redfish BMC Emulation**: Minimal implementation of Redfish API endpoints (`/redfish/v1/Systems`, `/redfish/v1/Managers`) for power and virtual media control.
- **Power Control**: Control physical machine power via GPIO relays (supporting ATX power switches).
- **Virtual Media**: Mount and unmount ISO files using the NanoKVM API for OS installation.
- **Security**: Basic Authentication with constant-time comparison.
- **Automated ISO Cleanup**: Background task to expire old ISOs.

## Endpoints

* `GET /redfish/v1/` - Service Root
* `GET /redfish/v1/Systems` - Computer Systems Collection
* `GET /redfish/v1/Systems/1` - Computer System details (Power State, Boot Target)
* `PATCH /redfish/v1/Systems/1` - Set Boot Source Override (e.g. `Pxe`, `Cd`)
* `POST /redfish/v1/Systems/1/Actions/ComputerSystem.Reset` - Reset actions (`On`, `ForceOff`, `ForceRestart`, `GracefulShutdown`)
* `GET /redfish/v1/Managers` - Managers Collection
* `GET /redfish/v1/Managers/1` - Manager Details (Virtual Media)
* `POST /redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia` - Mount an ISO image
* `POST /redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.EjectMedia` - Unmount an ISO image
* `GET /api/v1/power-state` - Current power state helper (non-Redfish)
* `PUT /api/v1/power-state` - Set power state helper (non-Redfish)

## Building

The API is built using Rust 2024 edition and optimized for the NanoKVM (RISC-V) environment.

To build locally for your host machine:

```bash
make build
```

To cross-compile for the NanoKVM (RISC-V with musl) using `cargo-zigbuild`:

```bash
cargo +nightly zigbuild --release -Z build-std=std,panic_abort --target riscv64gc-unknown-linux-musl
```

## Running

The API requires a proper `config.toml` file (see `tests/integration/config.test.toml` for an example). It supports TOML configuration with environment variable overrides (e.g., `NANOKVM_SERVER_PORT=8080`).

```bash
nanokvm-control-api serve --config /etc/nanokvm/config.toml
```

To run the standalone ISO cleanup task (typically managed via a systemd timer):

```bash
nanokvm-control-api cleanup --config /etc/nanokvm/config.toml
```

## Testing

Run unit tests:
```bash
make test
```

Run integration tests using Docker:
```bash
make test-integration
```

## Setup on NanoKVM

Systemd unit files for both the primary API service and the ISO cleanup timer are provided in `systemd/`.

1. Copy the built binary to `/usr/local/bin/nanokvm-control-api`.
2. Place your `config.toml` in `/etc/nanokvm/`.
3. Copy the systemd files to `/etc/systemd/system/`.
4. Run `systemctl daemon-reload && systemctl enable --now nanokvm-control-api.service nanokvm-cleanup.timer`.

## LLM Usage

I used LLMs while working on this project. I wouldn't call it "vibe-coded" but if you have issues with code that has LLM generated components, do not use this project.
