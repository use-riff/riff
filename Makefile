# Always build for SBPF v0: it deploys on every cluster and runs in LiteSVM's
# default runtime. (Anchor 1.x defaults to v3, which LiteSVM rejects.)
ARCH ?= v0

.PHONY: build build-devnet test fmt lint clean deploy-devnet devnet-config

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

# Devnet deployment (keys in ~/.config/riff/devnet, settings in deploy/devnet.json).
DEVNET_KEYS ?= $(HOME)/.config/riff/devnet

deploy-devnet: build-devnet
	solana program deploy target/deploy/riff-devnet.so \
	  --program-id target/deploy/riff-keypair.json \
	  --keypair $(DEVNET_KEYS)/deployer.json --url devnet

# Dry run; add ARGS=--yes to send.
devnet-config:
	pnpm -C scripts init-config --cluster devnet $(ARGS)
