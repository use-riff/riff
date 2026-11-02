# riff-dbc

riff's artist fees on top of [Meteora's Dynamic Bonding Curve](https://github.com/MeteoraAg/dynamic-bonding-curve) (DBC).

Meteora runs the curve, trading and graduation. Launchpads built on DBC (Audius Artist Coins, Bags, Jupiter Studio) trade in Jupiter, Axiom and other apps from their first trade, not only after graduation. This program adds what makes riff riff: the artist's share is held for them until they claim with their [Artist Passport](../riff-passport/README.md), and goes to music charity if they don't.

**Status:** tested against Meteora's real mainnet programs, set up for devnet. Not audited. It replaces `programs/riff`'s own curve, swaps and graduation.

## How it works

| Piece | What it is |
|---|---|
| riff's DBC config | Created once with Meteora. Its fee claimer is riff's **fee authority** (a PDA of this program). |
| `create_coin` | Creates the coin's Meteora pool with a per-coin **escrow** PDA as the pool creator, for good, and records the launcher, artist and claim deadline. The launcher's first buy can go in the same transaction, capped (see below). |
| `collect_fees` | Anyone can call it. It claims this pool's partner share and splits it between the coin's launcher and riff's treasury, and moves the creator share into the coin's **artist vault**. |
| `claim_artist` | The passport's wallet, co-signed by riff's verifier. The coin's artist becomes the **passport's vault**: the held fees are paid there in SOL. A late claim (after the window) sends what was held to charity; only later fees go to the artist. |
| `pay_artist` | Anyone can call it. It pays a claimed coin's collected fees to the passport vault, in SOL. |
| `sweep_charity` | After the claim window, while unclaimed, anyone can call it. It sends held artist fees to the charity wallet. |
| `collect_lp_fees` | After graduation, anyone can call it. It collects the LP fees of the coin's two locked DAMM v2 positions: riff's side split with the launcher, the artist's side into the artist vault. |
| `update_config` | The admin: verifier, charity, treasury, launcher share, launch buy cap, claim window (for new coins). |

Trades don't go through this program. A trade on Jupiter or Axiom pays the same fees as one on riffpad.fun, because Meteora charges them.

**Why the escrow stays the creator.** The artist's earnings follow the passport, not a wallet: if the artist loses their wallet, the passport's recovery moves the passport to a new one, and the vault (and every coin's fees) comes with it. Keeping the escrow as creator also means the creator's locked LP position after graduation stays with riff's program, so its fees keep following the same route.

**Why the vault is paid in SOL.** Meteora pays fees as wrapped SOL, and the passport's vault holds SOL. WSOL only unwraps by closing a token account, so payouts go through a WSOL account (`["unwrap", mint]`) that lives for one instruction: the caller fronts its rent and gets it back in the same instruction.

## Fees

The trade fee is 2%, the same from the first trade on:

| Share | Of each trade | How |
|---|---|---|
| Meteora | 0.4% | Meteora's protocol fee: 20% of the trade fee |
| Artist | 0.8% | Pool creator share: `creatorTradingFeePercentage = 50` |
| Launcher | 0.3% | 37.5% of the partner share: `launcher_share_bps = 3750` |
| riff | 0.5% | The rest of the partner share |

The coin graduates at about 72 SOL in the curve (30 → 300 SOL market cap; devnet 100x less). After graduation the DAMM v2 pool charges 1%, in SOL only. All liquidity is locked for good, half on the creator's side (the escrow) and half on the partner's (the fee authority), and each side's LP fees follow the same split as before.

## The launch buy

The launcher can buy in the transaction that creates the coin: plain Meteora `swap` instructions after `create_coin`. `create_coin` reads the transaction and refuses it if those buys total more than `max_launch_buy` (1 SOL planned for mainnet, 0.01 SOL on devnet), or if any other kind of Meteora instruction touches the new pool.

It caps the obvious path, not every path: buys nested inside another program's instruction (a router, say) don't show up in the transaction's top-level instructions, so a determined launcher can still buy more in the same transaction. Those buys pay the same 2% as everyone's.

There's no anti-sniping fee. Meteora's launch fee schedule only lets a buy in the pool's creating transaction pay the normal fee when that transaction creates the pool itself, at the top level. riff's program creates the pool (its escrow signs as creator), so the launcher's own buy would pay the anti-sniping fee too. The tests found this.

## What the tests prove

`tests/test_dbc.rs` runs against Meteora's real mainnet DBC and DAMM v2 programs:

- a coin is a Meteora pool with riff's escrow as creator;
- fees from trades made straight on Meteora are collected **per coin** and split exactly as above, and only reach the right wallets;
- the launch buy is capped;
- the artist claims with their passport (not another wallet, another artist's passport, a revoked passport or without riff's verifier), and the held fees and later fees arrive in the passport's vault as SOL;
- unclaimed fees go to charity after the window; a late claim gets only fees from then on;
- graduation to DAMM v2 gives one locked position to the escrow and one to the fee authority, and their LP fees follow the same routes, to charity too.

```bash
make dbc-fixtures   # downloads Meteora's programs (pinned hashes)
make test
```

The Passport accounts in the tests are written directly (riff-passport's own tests cover issuing them).

## Devnet

Meteora's programs have the same addresses on devnet. Keys stay in `~/.config/riff/devnet`, never in the repo.

```bash
make deploy-dbc-devnet              # riff-dbc (keys: deployer.json, riff-dbc-program.json)
cd programs/riff-dbc/config && npm install
npm run devnet:setup                # riff's Meteora config and riff-dbc's config, once
npm run devnet:e2e                  # a coin's whole life, on devnet
npm run devnet:status
```

`devnet:e2e` launches a coin for the test artist `spotify:0RiffPassportDemo00001` with a launch buy, trades it, collects fees, issues a test passport (riff's verifier records two proofs; a software passkey in `rehearsal/e2e-passkey.jwk` signs), claims the coin with it, buys out the curve, migrates it to DAMM v2, trades there, collects the LP fees, pays the artist, withdraws from the passport's vault and finally resets the passport (devnet-only `reset_passport`), so none is left on riffpad.fun. It prints an explorer link for each step and needs about 1 SOL in `deployer.json` per run. `node devnet.mjs reset-passport` cleans up after a run that stopped early.

## Files

- `config/gen-config.mjs` builds the config parameters with Meteora's SDK and validates them. It writes the mainnet ones to `tests/fixtures/config_parameters.bin`. Run `npm install && npm run gen` in `config/`.
- `config/devnet.mjs`: the devnet setup and end-to-end run.
- `../../idls/dynamic_bonding_curve.json` (DBC 0.2.2) and `../../idls/cp_amm.json` (DAMM v2 0.2.5) are Meteora's interfaces, from their MIT-licensed SDKs. `declare_program!` uses them for type-safe calls.
- `tests/fixtures/damm_v2_config.json` is Meteora's mainnet DAMM v2 config for DBC pools with a customizable fee (`A8gMrEPJkacWkcb3DGwtJwTe16HktSEfvwtuDh2MCtck`).
- Meteora's program source is under a [non-commercial licence](https://github.com/MeteoraAg/dynamic-bonding-curve/blob/main/license.md). Its compiled programs are downloaded for tests, never committed.

## Limits and next steps

1. **Late claims after graduation.** `claim_artist` collects the curve's held fees before deciding what goes to charity, but not uncollected LP fees: run `collect_lp_fees` before a late claim, or those go to the artist.
2. **Surplus.** The graduating buy fills the curve exactly, so Meteora's surplus is zero and this program doesn't collect it.
3. **Admin.** The admin and the program's upgrade authority should be a multisig on mainnet.
4. **App.** riffpad.fun needs to create coins, trade (through Meteora or Jupiter), run the cranks (`collect_fees`, `collect_lp_fees`, `pay_artist`, `sweep_charity`) and claim through riff-dbc.
5. **Audit** before mainnet.
