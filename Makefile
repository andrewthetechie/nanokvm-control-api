default: build-m1

setup-m1:
	brew install zig
	cargo install cargo-zigbuild
	rustup target add riscv64gc-unknown-linux-musl --toolchain nightly

build-m1:
	cargo +nightly zigbuild -Z build-std=std,panic_abort --target riscv64gc-unknown-linux-musl
