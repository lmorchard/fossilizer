.PHONY: check fmt lint test build serve docs clean

# Format + lint (matches what CI enforces)
check: fmt lint

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --locked -- -D warnings

test:
	cargo test --locked

build:
	cargo build --release --locked

# Serve the built site locally (run `make build` and `fossilizer build` first)
serve:
	cargo run -- serve --open

docs:
	./scripts/build-docs.sh

clean:
	cargo clean
