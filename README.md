# riff

riff is a Solana launchpad for music-artist community coins. Fans launch a coin for an artist they love; every trade pays the artist a share of the fee, held for them until they claim it with their Artist Passport, and sent to music charity if they don't. The app is at [riffpad.fun](https://www.riffpad.fun).

**Status:** live on devnet for testing. **Not on mainnet, and not audited.** riff hasn't launched a token; official launches are only announced by [@useRiffPad](https://x.com/useRiffPad). Devnet coins have no value. riff makes no promise about the value of any coin. Choosing an artist doesn't mean they're involved: a coin is marked as not endorsed until the artist claims it.

Source-available under the terms in [LICENSE](LICENSE). The web app lives in a separate repository.

## What's here

| Path | What |
|---|---|
| [`programs/riff-dbc`](programs/riff-dbc/README.md) | riff on [Meteora's Dynamic Bonding Curve](https://github.com/MeteoraAg/dynamic-bonding-curve): creates each coin's pool, holds the artist's share, splits riff's share with whoever launched the coin |
| [`programs/riff-passport`](programs/riff-passport/README.md) | Artist Passport: verified artist identity, passkey second factor, recovery time-lock, a vault for earnings |
| `idl/` | The programs' published interfaces (CI fails if they're stale) |
| `idls/` | Meteora's interfaces, from their MIT-licensed SDKs |
| `deployments.json` | Addresses per cluster |
| [`docs/AUDIT_PACKAGE_DBC.md`](docs/AUDIT_PACKAGE_DBC.md) | Scope, trust model, invariants and threat notes for an auditor |
| [`docs/MAINNET_READINESS.md`](docs/MAINNET_READINESS.md) | What's done, with evidence, and what's left |
| [`docs/MAINNET_LAUNCH.md`](docs/MAINNET_LAUNCH.md) | Mainnet parameters, keys, cost and launch commands |

The original own-curve program (`programs/riff`, with Raydium graduation) never reached mainnet and was removed after commit `7cab289`.

## Building and testing

Linux or WSL with Rust (pinned by `rust-toolchain.toml`), Solana CLI 4.1.2 and Anchor CLI 1.2.0:

```bash
make dbc-fixtures   # downloads Meteora's programs (hash-pinned; not committed, for licence reasons)
make test           # builds, then runs everything in LiteSVM against Meteora's real programs
make lint
make idl            # regenerates idl/ (10+ minutes)
make verifiable     # reproducible builds in Docker: what mainnet deploys
```

See [SECURITY.md](SECURITY.md) to report a vulnerability.
