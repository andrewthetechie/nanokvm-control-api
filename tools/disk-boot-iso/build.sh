#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/images"
OUTPUT_FILE="$OUTPUT_DIR/disk-boot.iso"
IMAGE_NAME="disk-boot-iso-builder"
CONTAINER_NAME="disk-boot-iso-build-$$"

# Ensure container is cleaned up even if script is interrupted
trap "docker rm -f $CONTAINER_NAME >/dev/null 2>&1 || true" EXIT

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
