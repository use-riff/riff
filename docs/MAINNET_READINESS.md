# Mainnet readiness: riff-dbc, Artist Passport, riffpad.fun

A running record of what's done, with evidence, and what's left for humans. Nothing here has touched mainnet: every mainnet step waits for the owner's go-ahead.

| # | Item | Status |
|---|---|---|
| 1 | Devnet Passport upgrade, full devnet e2e with the claim | **Done** (2026-10-27) |
| 2 | riff-dbc hardening (late-claim LP fees, events, threat review, tests) | **Done** |
| 3 | Audit package for riff-dbc + riff-passport | **Done**, verifiable build included; the audit itself is for humans |
| 4 | riffpad.fun on riff-dbc | **PR open** (use-riff/riff-site#102), waiting for the owner's OK because merging changes the live devnet site |
| 5 | Mainnet parameters and keys plan | **Written** ([`MAINNET_LAUNCH.md`](MAINNET_LAUNCH.md)); nothing executed |
| 6 | Operations: cranks, alerts, runbook, fee and charity ledger | Not started |
| 7 | Retire the old `riff` program from the app | Not started |
| 8 | Go/no-go report | Not started |

## 1. Devnet Passport upgrade

riff-passport on devnet (`2nke6euX…`) ran a build from before `wallet_since` (PR #13), so riff-dbc misread its passports (`ConstraintSeeds` on `passport`).

- With the owner's OK, the 3 devnet passports (`0RiffPassportDemo00001`, `0RiffPassportDemo00003`, `1xZSDbBW87LC9bvp6S3Orm`) were reset with the old binary first, so none was left in the old layout: [1](https://explorer.solana.com/tx/B144kn2eqiJyhNuUaTkQ8xXfoBpeTqALKfET4aNf8digVq4Vvy3tg31Aq7Jj88b6B5p3bKeJzenQF9s93HEhy7u?cluster=devnet), [2](https://explorer.solana.com/tx/3UirFWsnzp8dvKc6NYCZUdxdmL4LVwGfKrwRXcwjExBfUneWdAA3tJ4duXd3zKWjeu3gmgtFYYCfwG2fTXcLAdXq?cluster=devnet), [3](https://explorer.solana.com/tx/4haLFfMC9C4A2Gh1dEcBpjem1fg8P6JxTXk446vhCneFPba1hsnkCdAJ1pZhjFycsTenCLrYE65vdhRLh4EP1xqe?cluster=devnet). The owner doesn't want them back.
- Devnet build of main deployed: [tx](https://explorer.solana.com/tx/58k2CfYRGXZqgGvB8VYdonJLDq645VzA8bNhZtG6ysyrKxY83oLk5UBjNBsRAEgtpYUah9WE8BXFYURiqeTbXjfn?cluster=devnet), sha256 `5ac1ec0e…` (in `deployments.json`).
- `make deploy-passport-devnet` and `make deploy-devnet` pointed at Anchor's local throwaway keypairs in `target/deploy/` (other program IDs). Fixed: they use the devnet program's key and address.
- `node devnet.mjs e2e` now issues a test passport, claims, and resets it at the end. Full run, every step on devnet:
  - [create_coin + capped launch buy](https://explorer.solana.com/tx/2uxjMaeTvMAj7rH2jQeebWaxCmb6awbsrsJGtFYHY3X5zqqYVh8YGyCykJS2NCWKMe1r7pHvpmTqPU9fSmjwXrZH?cluster=devnet), [trade](https://explorer.solana.com/tx/61iiuktbGhTdQ6kt7mQRRi5wjermfy8PRgxbMqKJz2sTxFdemno8BDQF1xkW3qJWu9czs8zJ32HBqMKfZFHREbYp?cluster=devnet), [collect_fees](https://explorer.solana.com/tx/2Ru5MchE8upf2GNQZ8w6qaVy9dKszrov8EZyYdadHQxNxt4TAtzZ4A5enwE4TyMjHWrtfGjtsCFPAopDYGbYRnoQ?cluster=devnet)
  - [record_proof ×2](https://explorer.solana.com/tx/5U69Y92VgisANNa7M1RFYyaEyLAAVBbVFMofURcuyVoCRwvtctDtTDSjGUmjxhN2Q1tXzXWerrJeEAPtNcuaKSNf?cluster=devnet), [issue_passport with a passkey](https://explorer.solana.com/tx/4K3Sp2hGkDLbabHnQKusijPQP4jYiAw3YqWqrd5dcCUzwvfZy86EjcYihQeQzMpLZiGxGPHALxSLj6oSrqUxQeQu?cluster=devnet)
  - [claim_artist](https://explorer.solana.com/tx/LNDgydeTNnzdE7sJTVzCbkPyr4Xtma5PzScNr6jmYHfPytHTk3HYbM5d4wKHXQbHf4tsUhimjKdzBSFyabmwco2?cluster=devnet): passport vault +0.000880 SOL, the held artist share, as SOL
  - [graduating buy](https://explorer.solana.com/tx/4NWTtkZBv2mU2f5ZDS2hMs66eqFQJyEG2XjxbnZDgcpKWF1EPZPvCvS44vApzTykKJRg9HVYSUDvXLxwcxnFrhsd?cluster=devnet), [migration to DAMM v2](https://explorer.solana.com/tx/3wRoyVg9D2NLTpm1sfKqrAKimQRYnp1n4uHaL2KkRAhjn1BKjd4VMUNGq9nChqpBaVp89ZmK6G7re7Xx1mzXxkgB?cluster=devnet), [DAMM v2 trade](https://explorer.solana.com/tx/3gx62Vu28vh8yxHyUgBxhd6Fu8dmdHezHwmR7hq6bMku8dwnovLfxTLDZKPoxSnMugzWRTh3pFsdrRWzjz6TnRon?cluster=devnet), [collect_lp_fees](https://explorer.solana.com/tx/teqkpTxtiKrKR1PPCFX3stVN2fbcLfEUbCtSShF4UiCFq6A8nJ6ijxWFZrEN5jyedB6RgF5GhEQgz8P2jKtc3pz?cluster=devnet), [collect_fees](https://explorer.solana.com/tx/SeH2xDF9wBtdRSmvjd47Nz2z679JjqaaSBAUBQiapggmbfk56q6UNCHFPmxhAZTSqAf5xUm2igSivgk1tYBbmph?cluster=devnet)
  - [pay_artist](https://explorer.solana.com/tx/4M19kVVp3aW1qYofnRVZYpii7yn8tsqybvkAMVBNZrmMkWcqX56fxZJJFhHWr84GyRdeJzfkb3VZxPncLj2Q7C7c?cluster=devnet): passport vault +0.005404 SOL
  - [passport withdraw](https://explorer.solana.com/tx/d6JR6UnzH1Vy5RSzL4PR42LvVjvG9buYtFq8usMnK7Gi2TwDwsa1DqiVcREKiQMH35Nuo3xEXJVyLmFbhM9oWfb?cluster=devnet) of 0.001 SOL from the vault to the passport's wallet, after its probation
  - [reset of the test passport](https://explorer.solana.com/tx/5gMwBHFkWBsed7Rsec3SqeNwa24jtFSKZn6sNrj2TPMuCyZUHY3BC3AHj9U43a4cPcsn7kdEHzAXcz6WLZqeBq9Q?cluster=devnet): devnet has no passports now
- Found on the way: a probation wait based on this machine's clock ran out minutes before the chain's clock got there, so the wait uses the chain's clock.

**Left:** riffpad.fun's devnet passport pages read the old layout until riff-site syncs the new IDL (item 4). There are no devnet passports to show meanwhile.

## 2. riff-dbc hardening

**Late claim after graduation (was known limit 1): fixed in the program.** `claim_artist` reads the DBC pool's `is_migrated`. On a late claim of a migrated coin it requires the DAMM v2 accounts and collects the creator position's LP fees into the artist vault before sending the vault to charity; without them it refuses (`LateClaimNeedsLpAccounts`).

*What this relies on:* nothing on chain can tell the migration's creator position from another one in the same pool. DAMM v2's `create_position` takes any owner without its signature (per the IDL), and the position NFT accounts checked on devnet have no immutable-owner extension, so a dust position could be made or handed to the escrow. This wasn't run as an attack; the design assumes it works. A late claimer passing such a position would keep the real position's uncollected fees. So the **verifier vouches for `creator_position`**: `api/claim.ts` must pass the migration's position (the escrow's position in the coin's derived DAMM pool with the largest permanently locked liquidity) and refuse anything else. The crank collecting LP fees often keeps the amount at stake small either way.

On devnet (`node devnet.mjs e2e --late`, binary `cfaa3131…` from `make build`): a coin with a 2-minute window graduated, traded on DAMM v2 with nothing collected, and the artist claimed late. Without the LP accounts: refused. [With them, no compute-budget instruction](https://explorer.solana.com/tx/3TY6Aobky4mcRjeHnRP8evdBuNQsk9yj5F9kZTTfbocsXkpfxC43PT9JCjKMjum2cKryHJb7FbGwsK6Fq2ssDHKv?cluster=devnet): charity +0.006284 SOL, passport vault +0. The normal `e2e` also passes on this binary.

**Other changes**

- **Events:** `CoinCreated`, `FeesCollected` (curve and LP), `ArtistClaimed`, `ArtistPaid`, `CharitySwept`.
- **`escrow_base_account` is pinned to the escrow's ATA** in `collect_fees`, `collect_lp_fees`, `sweep_charity` and `claim_artist`. With the check removed from `sweep_charity`, Meteora accepted another coin's account there and the substitution test failed. Base-side fees are zero with riff's config, so this closes a route rather than a live loss.
- **`create_coin` checks the artist ID format** the passport requires (`spotify:` + 22 alphanumerics). Before, a coin with any other ID could never be claimed, and all its artist fees would go to charity.

**Transaction size.** The late claim on a graduated coin was 1239 bytes with a compute-budget instruction, over Solana's 1232 limit (LiteSVM doesn't enforce it). It uses about 96k compute units, under the 200k default, so it's sent without one and fits; a test asserts both. **It has no room for a priority-fee instruction** unless it moves to a v0 transaction with an address lookup table.

**Tests added** (`tests/test_dbc.rs`, 21 in total, against Meteora's real programs): on-time claim after graduation; late claim after graduation (charity gets earlier LP fees, the artist only later ones); late claim with nothing left to collect; `pay_artist` after graduation; sweep and claim in both orders, and at the deadline second itself; launch buy with several swaps; `swap2` in the creation transaction; bad artist IDs; another coin's DAMM pool or positions, and swapped positions, in `collect_lp_fees`; passport recovery, revocation and reissue keeping the vault; events; **every account of every permissionless money instruction and of `claim_artist` swapped for another coin's, one at a time, must fail** (a deterministic stand-in for fuzzing the substitution paths).

Passport recovery and reissue are simulated in these tests by rewriting the passport account; riff-passport's own tests cover those instructions.

**Threat notes** per instruction go in the audit package (item 3).

**Left:** real fuzzing is not done; that's a decision for the audit.

## 3. Audit package

[`AUDIT_PACKAGE_DBC.md`](AUDIT_PACKAGE_DBC.md): scope, trust model, what's relied on from Meteora, invariants, threat notes per instruction, known limits, tests, questions.

**Verifiable build:** `make verifiable` ran from a clean clone, in Docker. Hashes are in the audit package. The riff-dbc and riff-passport suites pass on those binaries (21 and 27 tests). They differ from `make build`'s, which CI and devnet use, so **devnet's e2e hasn't run on the exact binaries mainnet will use**: worth doing once, it needs about 2.4 SOL of buffer in the deployer (0.3 now; the airdrop was rate-limited).

**Left for humans:** choosing and hiring the auditor.

## 4. riffpad.fun on riff-dbc

use-riff/riff-site#102. Tested on devnet:

- `pnpm -C scripts rehearse`: launch with a capped buy, trades matching the client's quotes to the base unit, crank, passport claim, graduation, DAMM v2 trades, LP fees, payout, passport reset. Ran before a later fix to how trades are read back; not rerun since.
- `pnpm -C scripts crank --yes`: 8 coins, 10 steps, 0 failed.
- Production build in a real browser with a test wallet: 9 pages render, and a buy and a sell through the UI confirmed. This found a crash that unit tests couldn't (`process is not defined` in Meteora's SDK).

**Answers to the open questions**

- *Does Meteora auto-migrate riff's pools?* By its docs, on mainnet, yes: SOL pools with a threshold of at least 10 SOL. There's no keeper on devnet. Not observed on mainnet for riff's config. Anyone can migrate, so the crank is the fallback.
- *Do Jupiter and Axiom index pools from riff's config?* **Unverified, and can't be until a mainnet pool exists.** In a sample of 9 other DBC configs, Jupiter routed some through "Dynamic Bonding Curve" and found no route for others (some weren't checked, rate-limited). The README's "from the first trade" is now a hope, not a fact, so its wording changed.

**Not tested:** the launch form, claim button, fee-panel buttons and migrate button through the UI; `/api/passport`'s claim route over HTTP; `/api/crank` on Vercel.

**Behaviour that changed for users:** direct wallet claims are gone (riff-dbc pays passport vaults only); launcher fees arrive as wrapped SOL; volume is curve-total rather than 24h, and all-time high is gone (neither is on chain).

## 5. Mainnet plan

[`MAINNET_LAUNCH.md`](MAINNET_LAUNCH.md): parameters, keys, cost, commands. `make deploy-dbc-mainnet` and `deploy-passport-mainnet` are dry runs without `CONFIRM=mainnet`. `config/mainnet.mjs` simulates without `--yes`, refuses placeholders, and was checked with dry runs against devnet. It has never sent anything.
