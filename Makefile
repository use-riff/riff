# Always build for SBPF v0: it deploys on every cluster and runs in LiteSVM's
# default runtime. (Anchor 1.x defaults to v3, which LiteSVM rejects.)
ARCH ?= v0

.PHONY: build build-devnet test fmt lint clean deploy-devnet deploy-passport-devnet idl vectors

build:
	anchor build --arch $(ARCH)

# Publish the program's interface (IDL + TypeScript types) in idl/, so
# clients can use it without building Rust. CI fails if it's stale.
idl: build
	mkdir -p idl
	cp target/idl/riff.json idl/riff.json
	cp target/types/riff.ts idl/riff.ts
	cp target/idl/riff_passport.json idl/riff_passport.json
	cp target/types/riff_passport.ts idl/riff_passport.ts

# Regenerate vectors/curve.json from the program's curve math.
vectors:
	cargo test -p riff --test curve_vectors -- --ignored

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

# Devnet deployment (keys in ~/.config/riff/devnet, never in the repo).
DEVNET_KEYS ?= $(HOME)/.config/riff/devnet

deploy-devnet: build-devnet
	solana program deploy target/deploy/riff-devnet.so \
	  --program-id target/deploy/riff-keypair.json \
	  --keypair $(DEVNET_KEYS)/deployer.json --url devnet

deploy-passport-devnet: build
	solana program deploy target/deploy/riff_passport.so \
	  --program-id target/deploy/riff_passport-keypair.json \
	  --keypair $(DEVNET_KEYS)/deployer.json --url devnet

