# Disk Boot ISO Build Script Implementation Plan

> **For Antigravity:** REQUIRED WORKFLOW: Use `.agent/workflows/execute-plan.md` to execute this plan in single-flow mode.

**Goal:** Create a Docker-based build script that produces a minimal UEFI-bootable ISO which chainloads to the first hard disk.

**Architecture:** A shell script invokes Docker to build a Debian container with GRUB2 EFI tools. The container runs `grub-mkrescue` with a minimal config (`exit` immediately) to produce the ISO. Output is copied to `images/disk-boot.iso`.

**Tech Stack:** Docker, GRUB2 (`grub-efi-amd64-bin`), `xorriso`, `mtools`, shell script

**Design Doc:** `docs/plans/2026-03-01-disk-boot-iso-design.md`

---

### Task 1: Create GRUB Config

**Files:**
- Create: `tools/disk-boot-iso/grub.cfg`

**Step 1: Write the GRUB config**

```cfg
set timeout=0
set default=0

menuentry "Boot from disk" {
    exit
}
```

This is the entire bootloader config. GRUB's `exit` in UEFI mode returns control to the firmware, which tries the next boot device (the hard disk).

**Step 2: Commit**

```bash
git add tools/disk-boot-iso/grub.cfg
git commit -m "feat: add GRUB config for disk boot ISO"
```

---

### Task 2: Create Dockerfile

**Files:**
- Create: `tools/disk-boot-iso/Dockerfile`

**Step 1: Write the Dockerfile**

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    grub-efi-amd64-bin \
    grub-common \
    xorriso \
    mtools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Create the directory structure grub-mkrescue expects
RUN mkdir -p iso/boot/grub

COPY grub.cfg iso/boot/grub/grub.cfg

# Build the ISO
# grub-mkrescue creates a bootable ISO with an EFI System Partition
RUN grub-mkrescue \
    --output=/build/disk-boot.iso \
    --modules="normal exit" \
    iso
```

**Step 2: Verify the Dockerfile builds**

Run: `docker build -t disk-boot-iso-builder tools/disk-boot-iso/`
Expected: Image builds successfully, no errors

**Step 3: Commit**

```bash
git add tools/disk-boot-iso/Dockerfile
git commit -m "feat: add Dockerfile for disk boot ISO build"
```

---

### Task 3: Create Build Script

**Files:**
- Create: `tools/disk-boot-iso/build.sh`

**Step 1: Write the build script**

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/images"
OUTPUT_FILE="$OUTPUT_DIR/disk-boot.iso"
IMAGE_NAME="disk-boot-iso-builder"
CONTAINER_NAME="disk-boot-iso-build-$$"

echo "==> Building disk boot ISO..."

# Build the Docker image
echo "  Building Docker image..."
docker build -t "$IMAGE_NAME" "$SCRIPT_DIR"

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Create a container and copy the ISO out
echo "  Extracting ISO..."
docker create --name "$CONTAINER_NAME" "$IMAGE_NAME" /bin/true
docker cp "$CONTAINER_NAME:/build/disk-boot.iso" "$OUTPUT_FILE"
docker rm "$CONTAINER_NAME"

echo "==> Done! ISO written to: $OUTPUT_FILE"
ls -lh "$OUTPUT_FILE"
```

**Step 2: Make the script executable**

Run: `chmod +x tools/disk-boot-iso/build.sh`

**Step 3: Commit**

```bash
git add tools/disk-boot-iso/build.sh
git commit -m "feat: add build script for disk boot ISO"
```

---

### Task 4: Add Makefile Target

**Files:**
- Modify: `Makefile`

**Step 1: Add `build-disk-iso` target**

Add to the Makefile:

```makefile
build-disk-iso:
	./tools/disk-boot-iso/build.sh
```

**Step 2: Commit**

```bash
git add Makefile
git commit -m "build: add build-disk-iso Makefile target"
```

---

### Task 5: Build and Verify the ISO

**Step 1: Run the build**

Run: `make build-disk-iso`
Expected: Docker builds, ISO is extracted to `images/disk-boot.iso`, output shows file size (~2-5MB)

**Step 2: Verify the ISO structure**

Run: `file images/disk-boot.iso`
Expected: Output contains "ISO 9660" and "UEFI" or "EFI" in the description

**Step 3: Verify the ISO contains GRUB EFI files**

Run: `docker run --rm -v "$(pwd)/images/disk-boot.iso:/tmp/disk-boot.iso:ro" debian:bookworm-slim bash -c "apt-get update && apt-get install -y p7zip-full && 7z l /tmp/disk-boot.iso" 2>/dev/null | grep -i efi`
Expected: Shows EFI directory structure with `BOOTX64.EFI` or similar GRUB EFI binary

**Step 4: Final commit**

```bash
git add -A && git commit -m "feat: disk boot ISO build system complete"
```
