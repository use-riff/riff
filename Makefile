# Always build for SBPF v0: it deploys on every cluster and runs in LiteSVM's
# default runtime. (Anchor 1.x defaults to v3, which LiteSVM rejects.)
ARCH ?= v0

# Prototype (programs/riff-dbc): Meteora's Dynamic Bonding Curve program, as
# deployed on mainnet, for its tests. Its source is under a non-commercial
# licence, so it's downloaded rather than committed; the hash pins the exact
# build (update it on purpose when Meteora upgrades the program). Building
# compiles the tests too (for the IDL), so `build` fetches it when missing.
DBC_PROGRAM_ID := dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN
DBC_SO := programs/riff-dbc/tests/fixtures/dynamic_bonding_curve.so
DBC_SHA256 := 4c26a8a5da99f8ce932fa0300c46675b527090021fbb74214c9486bedda9f23b

.PHONY: build build-devnet test fmt lint clean deploy-devnet idl vectors dbc-fixtures

build: $(DBC_SO)
	anchor build --arch $(ARCH)

# Publish the program's interface (IDL + TypeScript types) in idl/, so
# clients can use it without building Rust. CI fails if it's stale.
idl: build
	mkdir -p idl
	cp target/idl/riff.json idl/riff.json
	cp target/types/riff.ts idl/riff.ts

# Regenerate vectors/curve.json from the program's curve math.
vectors:
	cargo test -p riff --test curve_vectors -- --ignored

# Devnet build (Raydium's devnet addresses), saved as riff-devnet.so so it
# can't be mistaken for the mainnet riff.so. Then rebuilds the mainnet
# riff.so, which the tests use.
build-devnet: $(DBC_SO)
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

# Downloads Meteora's program if it's missing; `make dbc-fixtures` refreshes it.
$(DBC_SO):
	solana program dump $(DBC_PROGRAM_ID) $(DBC_SO) --url https://api.mainnet-beta.solana.com
	echo "$(DBC_SHA256)  $(DBC_SO)" | sha256sum -c - || (rm -f $(DBC_SO) && false)

dbc-fixtures:
	rm -f $(DBC_SO)
	$(MAKE) $(DBC_SO)
