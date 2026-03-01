default: build-m1

setup-m1:
	brew install zig
	cargo install cargo-zigbuild
	rustup target add riscv64gc-unknown-linux-musl --toolchain nightly

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

build-m1:
	cargo +nightly zigbuild -Z build-std=std,panic_abort --target riscv64gc-unknown-linux-musl --release

# Docker integration tests
test-integration:
	docker compose -f tests/docker-compose.yml up --build --abort-on-container-exit --exit-code-from test-runner

upload-to-kvm: build-m1
	@if [ -z "$$KVM_IP" ]; then \
		read -p "KVM IP: " KVM_IP; \
	else \
		KVM_IP="$$KVM_IP"; \
	fi; \
	if [ -z "$$KVM_USERNAME" ]; then \
		read -p "KVM Username: " KVM_USERNAME; \
	else \
		KVM_USERNAME="$$KVM_USERNAME"; \
	fi; \
	rsync -avz --progress \
		-e ssh \
		target/riscv64gc-unknown-linux-musl/release/nanokvm-control-api \
		"$$KVM_USERNAME@$$KVM_IP:~/"

run-on-kvm: upload-to-kvm
	@if [ -z "$$KVM_IP" ]; then \
		read -p "KVM IP: " KVM_IP; \
	else \
		KVM_IP="$$KVM_IP"; \
	fi; \
	if [ -z "$$KVM_USERNAME" ]; then \
		read -p "KVM Username: " KVM_USERNAME; \
	else \
		KVM_USERNAME="$$KVM_USERNAME"; \
	fi; \
	ssh -tt $$KVM_USERNAME@$$KVM_IP "exec ~/nanokvm-control-api"
