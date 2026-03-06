# NanoKVM Virtual Media Direct ConfigFS Design

## Problem

The NanoKVM HTTP API refuses to unmount virtual media ISOs if the host OS has not ejected the virtual CD-ROM first. This causes failures in automated provisioning workflows (like Redfish) where the BMC needs to forcefully eject and swap ISOs regardless of the host state.

## Approach: Direct ConfigFS Control (Bypass NanoKVM API)

We will bypass the NanoKVM "Web UI" HTTP API entirely for Virtual Media operations, and instead interact directly with the Linux kernel's USB gadget `mass_storage` driver via `configfs`.

Since the `nanokvm-control-api` runs as root on the NanoKVM itself, it has full access to the `/sys/kernel/config/usb_gadget` filesystem.

### The ConfigFS Mechanism

The Linux USB gadget mass storage driver exposes a `forced_eject` attribute that resets the `prevent_medium_removal` flag at the kernel level, forcefully detaching the backing file.

The relevant path on the NanoKVM is typically:
`/sys/kernel/config/usb_gadget/kvm/functions/mass_storage.0/lun.0`

### Changes Required

1. **`VirtualMediaManager` Refactor**
   - Remove the dependency on `NanoKvmClient` for mounting/unmounting.
   - Add a configuration field for the configfs `lun.0` path (defaulting to `/sys/kernel/config/usb_gadget/kvm/functions/mass_storage.0/lun.0`).
   - Implement `mount_iso` and `unmount_iso` directly in the manager using standard file I/O operations (`tokio::fs`).

2. **Mounting an ISO**
   To mount an ISO:
   1. Write `1` to `.../lun.0/forced_eject` to ensure any existing media is forcefully removed.
   2. Write the absolute path of the new ISO (e.g., `/data/isos/boot.iso`) to `.../lun.0/file`.

3. **Unmounting an ISO**
   To unmount an ISO:
   1. Write `1` to `.../lun.0/forced_eject`.
   2. Write an empty string (`""`) to `.../lun.0/file`.

4. **Configuration Updates**
   - Update `config.rs` to include the `configfs_lun_path` under `[virtual_media]`.
   - Update `config.example.toml` and documentation.

### Trade-offs

- **Pros:** 100% reliable, synchronous, and fully controlled by our application. Solves the unmount error perfectly.
- **Cons:** The official NanoKVM Web UI will not reflect the virtual media mounted by our API (since we bypass their middleware). This is acceptable as our long-term goal is for our API to be the single source of truth for automation.

### Testing Plan

- Ensure `VirtualMediaManager` accepts the configfs path parameter.
- Implement a mock test configfs directory in `tests/integration/` (or via a mock struct if we want to abstract file I/O) to verify file writes during unit testing without requiring real root access to `/sys`.
- (Recommended) Create a `VirtualMediaController` trait similar to `PowerController`, with a `ConfigFsMediaController` and a `MockMediaController`. This keeps hardware interactions testable on macOS.

## Terminal State

This concludes the brainstorming phase. I will now invoke the `writing-plans` workflow to create a detailed implementation plan.
