# Always build for SBPF v0: it deploys on every cluster and runs in LiteSVM's
# default runtime. (Anchor 1.x defaults to v3, which LiteSVM rejects.)
ARCH ?= v0

.PHONY: build build-devnet test fmt lint clean

build:
	anchor build --arch $(ARCH)

# Devnet build (Raydium's devnet addresses), saved as riff-devnet.so so it
# can't be mistaken for the mainnet riff.so. Then rebuilds the mainnet
# riff.so, which the tests use.
build-devnet:
	anchor build --arch $(ARCH) -- --features devnet
	cp target/deploy/riff.so target/deploy/riff-devnet.so
	anchor build --arch $(ARCH)

test: build
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

clean:
	anchor clean
