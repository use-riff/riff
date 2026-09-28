# riff-dbc (prototype)

riff's artist fees on top of [Meteora's Dynamic Bonding Curve](https://github.com/MeteoraAg/dynamic-bonding-curve) (DBC).

Meteora runs the curve, trading and graduation. Launchpads built on DBC (Audius Artist Coins, Bags, Jupiter Studio) trade in Jupiter, Axiom and other apps from their first trade, not only after graduation. This program adds what makes riff riff: the artist's share is held for them until they verify, and goes to music charity if they don't.

**Status:** prototype on the `meteora` branch. It is not deployed, not audited and not used by the app. riff's production program is `programs/riff`.

## How it works

| Piece | What it is |
|---|---|
| riff's DBC config | Created once with Meteora. Its fee claimer is riff's **fee authority** (a PDA of this program). |
| `create_coin` | Creates the coin's Meteora pool with a per-coin **escrow** PDA as the pool creator, and records the launcher, artist and claim deadline. |
| `collect_fees` | Anyone can call it. It claims this pool's partner share and splits it between the coin's launcher and riff's treasury. Until the artist claims, it also moves the creator share into the coin's **artist vault**. |
| `claim_artist` | The artist, co-signed by riff's verifier, receives the held fees. The pool's creator becomes the artist, so from then on Meteora pays their share to them directly. |
| `sweep_charity` | After the claim window, anyone can call it. It sends unclaimed artist fees to the charity wallet. |

Trades don't go through this program. A trade on Jupiter or Axiom pays the same fees as one on riffpad.fun, because Meteora charges them.

## Fees

The trade fee is 2%:

| Share | Of each trade | How |
|---|---|---|
| Meteora | 0.4% | Meteora's protocol fee: 20% of the trade fee |
| Artist | 0.8% | Pool creator share: `creatorTradingFeePercentage = 50` |
| Launcher | 0.3% | 37.5% of the partner share: `launcher_share_bps = 3750` |
| riff | 0.5% | The rest of the partner share |

After graduation, the DAMM v2 pool charges 1%. All liquidity is locked for good, half on the creator's side and half on the partner's.

## What the tests prove

`tests/test_dbc.rs` runs against Meteora's real mainnet program. It proves:

- riff's program can create a pool with its own PDA as creator;
- fees from trades made straight on Meteora are collected **per coin** and split exactly as above;
- the fee shares only reach the right wallets;
- a verified artist receives the held fees and becomes the pool's creator, after which Meteora pays them directly;
- unclaimed fees go to charity after the window, and claiming is refused after that.

```bash
make dbc-fixtures   # downloads Meteora's program (pinned hash)
make test
```

## Files

- `config/gen-config.mjs` builds the config parameters with Meteora's SDK and validates them. It writes them to `tests/fixtures/config_parameters.bin`. Run `npm install && npm run gen` in `config/`.
- `../../idls/dynamic_bonding_curve.json` is Meteora's interface, from their MIT-licensed SDK. `declare_program!` uses it for type-safe calls.
- Meteora's program source is under a [non-commercial licence](https://github.com/MeteoraAg/dynamic-bonding-curve/blob/main/license.md). Its compiled program is downloaded for tests, never committed.

## Open questions and next steps

1. **Graduation.** Test migration to DAMM v2 with Meteora's DAMM v2 and locker programs. Check where the creator's locked liquidity position lands (the escrow), and pass it on to an artist who claims after graduation. Meteora only allows `transfer_pool_creator` before the curve completes or after migration.
2. **Charity after graduation.** Route an unclaimed coin's post-graduation LP fees the same way.
3. **Launch buy.** Cap the launcher's own first buy, like riff's 3%.
4. **Sniping.** Consider Meteora's fee scheduler (a higher fee at launch that decays).
5. **Curve numbers.** Graduation is at about 72 SOL today (30 → 300 SOL market cap). Pick real targets.
6. **Admin.** Config updates, verifier rotation, and the multisig.
7. **Client and app.** `@riff/client` support, and trading through Meteora (or Jupiter) in the app.
8. **Devnet deploy** of the prototype. Meteora's program has the same address on devnet.
