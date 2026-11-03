# riff

riff is a Solana launchpad for music-artist community coins. Fans launch a coin for an artist they love; every trade sets aside a share of the fee for the artist, held for them until they claim it with their Artist Passport, and sent to a charity wallet if they don't (riff donates from it by hand; no promise of amount or date). The app is at [riffpad.fun](https://www.riffpad.fun).

**Status:** an **unaudited beta**. Live on devnet for testing; **not on mainnet yet, and not audited**. [PLACEHOLDER: update at mainnet go-live; when live, coins are real money and can lose all their value.] riff hasn't launched a token; official launches are only announced by [@useRiffPad](https://x.com/useRiffPad). Devnet coins have no value. riff makes no promise about the value of any coin, and nothing here is financial advice. Choosing an artist doesn't mean they're involved: a coin is marked as not endorsed until the artist claims it. The riffpad.fun website is not offered in the EU, Iceland, Liechtenstein and Norway, but the programs are open to everyone and riff can't stop other apps from trading its coins. Upgrade authority and admin keys can change the programs: see [`docs/AUDIT_PACKAGE_DBC.md`](docs/AUDIT_PACKAGE_DBC.md) (trust model).

The code is public to read; all rights are reserved, and no licence is granted beyond that: see [LICENSE](LICENSE). The web app lives in a separate repository.

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
