# Disk Boot ISO — Design

## Problem

The NanoKVM Redfish BMC emulator needs a "disk boot ISO" — a minimal bootable ISO that, when mounted as virtual media and booted by the target machine, immediately chainloads to the machine's first hard disk. This is the default state: the BIOS is permanently configured to boot from USB (the NanoKVM's virtual media), and this ISO makes the machine boot normally as if USB wasn't involved.

## Constraints

- **UEFI x86_64 only** for now. Architecture should allow adding Legacy BIOS and ARM later.
- **Build script runs on Mac or Linux.** Docker is the build environment — no host dependencies beyond Docker.
- **Local-only workflow.** The script is run manually to generate the ISO, which is then uploaded to the device. Not a CI artifact.
- **Output:** A small (~4MB) ISO file.

## Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| Bootloader | GRUB2 | Battle-tested, trivial hybrid BIOS/UEFI support for future, ARM support available |
| Boot mechanism | GRUB `exit` command | Returns control to UEFI firmware, which falls through to next boot device (hard disk) |
| Build environment | Docker (Debian-based) | Reproducible, works on Mac and Linux, no host deps |
| ISO tool | `grub-mkrescue` + `xorriso` | Standard GRUB tooling, produces proper El Torito UEFI ISOs |

## Approach: GRUB-based EFI ISO

A Docker container installs GRUB's EFI binaries and `xorriso`, creates a minimal GRUB config that immediately exits, and runs `grub-mkrescue` to produce the ISO.

### How It Works

1. UEFI firmware boots from virtual USB (the ISO mounted by NanoKVM)
2. GRUB loads from the ISO's EFI System Partition
3. GRUB config sets `timeout=0` and executes `exit`
4. GRUB `exit` returns to UEFI firmware
5. Firmware falls through to the next boot option — the hard disk (NVMe, SATA, etc.)
6. Machine boots its OS normally

### GRUB Config

```
set timeout=0
set default=0

menuentry "Boot from disk" {
    exit
}
```

### Files

```
tools/disk-boot-iso/
├── Dockerfile       # Debian container with grub-efi-amd64-bin, xorriso, mtools
├── grub.cfg         # Minimal GRUB config (exit immediately)
└── build.sh         # Entry point: docker build + docker cp to extract ISO
```

**Output:** `images/disk-boot.iso` (gitignored, generated locally)

### Future Extensibility

- **Legacy BIOS:** Add `grub-pc-bin` to the Dockerfile. `grub-mkrescue` automatically produces a hybrid BIOS/UEFI ISO when both packages are present.
- **ARM (aarch64):** Add `grub-efi-arm64-bin` and build a second ISO or a multi-arch ISO via a build argument.
