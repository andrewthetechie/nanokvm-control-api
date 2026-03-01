FROM --platform=linux/amd64 debian:bookworm-slim

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
    iso
