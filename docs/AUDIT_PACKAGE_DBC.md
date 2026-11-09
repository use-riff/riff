# riff-dbc + riff-passport: audit package

Everything an external auditor needs to quote and start. **Neither program has been audited.** Humans choose and hire the auditor: see the end of this file.

## What it is

A launchpad for music-artist community coins on Solana. Meteora's Dynamic Bonding Curve (DBC) runs the curve, trading and graduation. `riff-dbc` creates each coin's pool, holds the artist's share of fees until the artist claims with an Artist Passport, sends it to charity if they don't, and splits riff's share with whoever launched the coin. `riff-passport` is the artist's identity and the vault their earnings go to. Real user funds will pass through both.

Read first: `programs/riff-dbc/README.md`, `programs/riff-passport/README.md`.

## Scope

| Program | Source | Instructions |
|---|---|---|
| riff-dbc | `programs/riff-dbc/src/` | `initialize`, `update_config`, `propose_admin`, `accept_admin`, `create_coin`, `collect_fees`, `collect_lp_fees`, `claim_artist`, `pay_artist`, `sweep_charity` |
| riff-passport | `programs/riff-passport/src/` (mainnet build, without the `devnet` feature) | see its README's table |

- **Commit:** the `main` commit named in the engagement. Anchor 1.2.0, Rust 1.89, Solana CLI 4.1.2, SBPF v0.
- **Out of scope:** Meteora's DBC and DAMM v2 (behaviour relied on is listed below and worth checking), Anchor, the token programs, all off-chain code.

## Accounts (riff-dbc)

| Account | Seeds | What it is |
|---|---|---|
| `Config` | `["config"]` | Admin, verifier, charity, treasury, DBC config, launcher share, launch buy cap, claim window |
| `Coin` | `["coin", mint]` | Pool, launcher, artist ID and name, `artist` (the passport vault once claimed), deadline, totals, launcher share (fixed at creation) |
| Escrow | `["escrow", mint]` | No data. The pool's creator for good, owner of the creator LP position, authority of the artist vault |
| Artist vault | `["artist_vault", mint]` | WSOL account: the artist's share until paid or swept |
| Unwrap | `["unwrap", mint]` | WSOL account that lives for one instruction, to turn WSOL into SOL |
| Fee authority | `["fee_authority"]` | No data. The DBC config's fee claimer, owner of every coin's partner LP position |
| Fees vault | `["fees_vault"]` | WSOL account the partner share passes through, shared by all coins |
| Pending admin | `["pending_admin"]` | A proposed new admin, until it accepts (closed then) |

## Money flow

Launch fee: 0.1 SOL per coin (`LAUNCH_FEE_LAMPORTS`), paid to `config.treasury` as plain SOL by `create_coin`. Trade fee 2%: Meteora 0.4%, artist 0.8%, launcher 0.3%, riff 0.5%. After graduation: 1% LP fee in SOL, on two permanently locked positions (50/50). The creator side follows the artist's route, the partner side the launcher and riff's split.

## Trust model

| Party | Can | Can't |
|---|---|---|
| **Upgrade authority** (planned: multisig with time-lock) | Anything, by upgrading either program | |
| **riff-dbc admin** (planned: same multisig) | Redirect the treasury and charity (applies to all later payouts, including already-unclaimed coins' fees); change the verifier and the launch buy cap; change the launcher share and claim window for coins created afterwards (each coin keeps its own). Hand over the admin in two steps (`propose_admin`, then the new key's `accept_admin`) | Change the DBC config; touch vaults directly; change a coin's deadline, launcher share or artist; set a claim window over a year |
| **Verifier** (riff's hot service; riff-dbc's and the passport's are separate config fields and should be separate keys) | Co-sign claims and record passport proofs, so it decides who can get a passport. **Vouches for `creator_position` on late claims.** Holding the passport verifier key is enough to issue a passport for an artist who has none (its vault pays out after the probation) and to start a recovery on any passport (it finalizes after the time-lock unless vetoed); guardian vetoes need it too. Limited by probation, the time-lock, revocation, and riff's monitoring (`MAINNET_LAUNCH.md`) | Move funds immediately: claims also need the passport's wallet, fees only reach the passport vault or charity, and a vault pays out only after the probation or time-lock |
| **Passport admin** | Revoke a passport, and reissue it with a wallet's fresh proofs and a new passkey. With the verifier (which the admin can set), that hands any passport and its vault to a wallet of the admin's choosing, after a probation of at least `MIN_RECOVERY_DELAY` (24 h on mainnet), visible on chain. Hand over the admin in two steps | Withdraw from a vault directly; set the time-lock under `MIN_RECOVERY_DELAY`, the free daily withdrawal over 1 SOL or the proof age over 30 days; change the passkey site |
| **Launcher** | Pick any artist ID in the valid format for their coin (selection ≠ affiliation); close their WSOL account, which only delays their own and riff's split (see griefing below) | Change anything after creation; hold up the artist's share |
| **Meteora** | Upgrade DBC or DAMM v2, whose authority isn't riff's: a change could break or redirect anything below. Tests pin the binaries' hashes; mainnet can't | |

## Behaviour relied on from Meteora

Tested against the real mainnet binaries (hashes in the `Makefile`) unless marked.

1. A pool's creator and fee claimer are the only ones able to claim those shares.
2. `claim_position_fee` checks that the pool's token A/B mints match the accounts passed, the position is in the pool, and the signer holds the NFT.
3. `is_migrated` in the DBC pool state is set once the pool has migrated to DAMM v2. The offset comes from the vendored IDL (`idls/`).
4. The graduating buy fills the curve exactly, so surplus is 0 and nothing else accrues to the escrow or fee authority (they have no instructions to claim it).
5. Base-side fees are zero with `collectFeeMode: QuoteToken`.
6. The mint has no mint or freeze authority, and metadata has no update authority (read from a devnet coin, `Ax5YbxL1…`, not asserted in tests).

## Invariants

1. A coin's artist share only ever leaves its artist vault to that coin's `coin.artist` passport vault, or to `config.charity`'s account.
2. `coin.artist` is set once, by a claim co-signed by the verifier and signed by the wallet of the unrevoked passport derived from `coin.artist_id`.
3. On time (`now <= claim_deadline`) the artist gets all of it. Late, everything accrued before the claim goes to charity, and only what accrues later to the artist.
4. Sweep needs `now > claim_deadline` and no artist. A sweep and a claim can't pay out the same fees.
5. Partner share of a coin: exactly the coin's `launcher_share_bps`, fixed at its creation (rounded down), to that coin's launcher, the rest to the treasury. Only the amount that just arrived counts, so another coin's or a donated balance in the shared fees vault is never split. A collect without the partner side's accounts leaves the partner share with Meteora.
6. A coin's fees can't be credited to another coin by substituting accounts.
7. The vault address depends only on the artist ID, so it survives recovery, revocation and reissue.
8. On a mainnet build, `reset_passport` doesn't exist, so a passport or vault is never closed.

## Threat notes by instruction (riff-dbc)

**`initialize`:** only the upgrade authority, once (`init`). Config parameters aren't range-checked beyond share and window: admin trust. The Meteora config, which can never change afterwards, is checked for what riff relies on (`check_dbc_config`): riff's fee authority as fee claimer, WSOL, fees in SOL before and after graduation, DAMM v2, all liquidity permanently locked, no migration fee, vesting or creation fee, Token-2022 with immutable metadata. Offsets come from the vendored IDL.

**`update_config`, `propose_admin`, `accept_admin`:** admin only, then the proposed key. See trust model. `dbc_config` can't change, since every coin depends on it.

**`create_coin`:**
- Anyone can launch for any artist ID in the passport format; the coin's name and symbol are free text. Impersonation is handled by claims and endorsements, not here.
- The launch fee is charged inside the handler, so a CPI caller pays it too. The receiver is bound to `config.treasury`, and the launcher must sign.
- `check_launch_buy` reads the creating transaction's top-level instructions only. A swap nested in another program, `create_coin` called by CPI, a second transaction in the same block or Jito bundle, or other wallets all escape the cap. Those buys pay the same fees. **The cap is not a security control**: it only stops the plain launch-and-buy transaction from going over it.
- It looks at Meteora instructions naming the pool as their third account (where swap and swap2 have it); only `swap` is allowed, and its amount counts. A sell counts its token amount against the cap, which only makes it stricter.

**`collect_fees`, `collect_lp_fees`:** permissionless. Receivers are bound to `coin.launcher` and `config.treasury`.
- The partner side's accounts (the launcher's and treasury's accounts, and for LP fees the partner position and its NFT account) are optional, all or none (`PartnerAccountsIncomplete`). Without them only the artist's side is collected, and the partner share waits with Meteora for a later call.
- *Griefing:* a launcher can close their WSOL account. The crank then collects without the partner side instead of recreating it, so the artist's share never waits on the launcher; the launcher's and treasury's split waits until the launcher has an account again.
- A fake escrow-owned position passed to `collect_lp_fees` collects nothing real: harmless, since the real one can still be collected.

**`claim_artist`:**
- Doesn't check the passport's probation: claiming only moves money into the vault, and `withdraw` checks probation. A late claim by a takeover does end the charity route for later fees, which then wait in a vault the real artist gets through reissue.
- Late claim on a migrated coin: see "Known limits" 1.
- The `unwrap` PDA can be pre-funded by anyone. `init` copes, and any extra lamports go to the caller when it closes.

**`pay_artist`:** permissionless. The destination is bound to `coin.artist`. The payer must be able to front rent and gets it back in the same instruction.

**`sweep_charity`:** permissionless after the window. Destination bound to `config.charity`'s account. Sweeps only what the DBC side and the vault hold: LP fees must be collected first or they wait for the next sweep.

**Common:** `base_mint` is unchecked but binds `coin`, escrow and vaults through seeds. `pool` is bound to `coin.pool`. `escrow_base_account` is bound to the escrow's ATA. Overflow checks are on in release. Neither Meteora nor the token programs call back into riff-dbc (the base mint has no transfer hook), so reloads after CPI see final balances.

## riff-passport notes

Internal review points, on top of its README and `tests/all/test_takeover.rs`:

- `Passport` layout is read by riff-dbc through the crate: a layout change needs both redeployed (this caused the devnet `ConstraintSeeds` failure).
- The hand-rolled secp256r1 instruction parsing in `passkey.rs`, and the JSON parsing of `clientDataJSON`: `type`, `challenge`, `origin` (https, the passkey site or a subdomain, compared by hashing each label suffix of the host against the rpId hash) and no `crossOrigin` or `topOrigin`. The rpId hash is checked in the authenticator data too.
- Each way a passkey gets onto a passport signs its own action (`Register` to issue, `Recover` naming the new wallet, `Reissue` naming the wallet, `SetPasskey` signed by both keys).
- Config bounds in `constants.rs` (`MIN_RECOVERY_DELAY`, `MAX_FREE_WITHDRAW_PER_DAY`, `MAX_PROOF_MAX_AGE`); the passkey site is fixed at initialization; the admin hands over in two steps.
- A pending recovery ends when the holder changes the wallet, passkey or proofs (all passkey-approved), and `finalize_recovery` also refuses one requested before the current wallet took over. While one is pending, every withdrawal needs the passkey. A guardian can replace a pending recovery with its own (`contest_recovery`).
- `withdraw` moves lamports directly, keeping the vault's rent.
- Guardian veto and proof freshness rules: `recovery.rs`.

## Known limits

1. **Late claim LP fees rely on the verifier.** Nothing on chain tells the migration's creator position from a dust one handed to the escrow (`create_position` takes any owner; NFT accounts have no immutable owner). A late claim passing another position leaves the real one's fees to the artist. The verifier co-signs, so it must pass the right position. Bounded by how often the crank collects.
2. **Late claim size:** it fits a plain transaction without a compute-budget instruction (asserted in a test, and it landed on devnet); with one it is 1239 bytes. A priority fee needs a lookup table.
3. **The launch-buy cap** covers top-level swaps in the creating transaction only: not a security control.
4. **No anti-sniping fee.** Meteora's first-swap exemption doesn't work for pools created by CPI.
5. **No pause** in riff-dbc. Trading is Meteora's; riff's instructions can only be stopped by an upgrade.
6. **Pools on riff's Meteora config made elsewhere.** Anyone can create a DBC pool on riff's config without `create_coin`. Its creator fees go to its own creator, and its partner fees and partner LP position to riff's fee authority, where no instruction can collect them (there's no `Coin`). Indexers must identify riff coins by the `Coin` account or riff-dbc's `CoinCreated`, not by the Meteora config.
7. **`emit!` events** are plain logs that other programs can imitate. Readers must check the program ID of the emitting instruction.
8. **Fuzzing:** a deterministic per-account substitution test exists; no coverage-guided fuzzing has been run.

## Tests

```bash
make dbc-fixtures   # Meteora's binaries, hash-checked
make test           # riff-dbc: 28 tests in LiteSVM against Meteora's real programs
make lint
```

riff-dbc's tests write Passport accounts directly; `cargo test -p riff-passport` (33 tests, plus 3 unit tests) covers issuing them. LiteSVM doesn't enforce the transaction size limit: the largest claim is asserted in a test, and `node programs/riff-dbc/config/devnet.mjs e2e [--late]` runs the flows on a real validator (evidence in `MAINNET_READINESS.md`).

## Build reproducibility

The toolchain is pinned: `rust-toolchain.toml`, `Cargo.lock`, and CI pins Solana CLI and Anchor by hash.

`make verifiable` builds both programs with `solana-verify` in Docker, using the Solana Foundation's 4.1.2 image pinned by digest. **This is the build to audit and deploy.** From a clean clone of `bf9b472` (main with the security review's fixes), 2026-11-09:

| Program | Size | File sha256 | `solana-verify` executable hash |
|---|---|---|---|
| riff-dbc | 498,688 bytes | `db2ae6eebdbbfb78173cf20dfa14a2dfdaa5d3259b3c091efc21abf15205c09b` | `402652a261dfdd5037a202c8debc9dd67cc051c27fbd9c483e0a45912e5a0312` |
| riff-passport | 519,632 bytes | `45eb0e515e7df2caf1d9d64512343b6e09b61fd1d6b81e516c6d2c9daf9f5656` | `bc0f9226202b89ea3f139c3436e9d8adaf6581b02244615069131d4ba90a6ee2` |

These change whenever the programs' source or dependencies do: **the hashes that matter are the ones from the audited commit.** At the earlier commit `7cab289`, riff-dbc's Docker build matched a plain `cargo build-sbf --manifest-path programs/riff-dbc/Cargo.toml --arch v0` on the developer's machine byte for byte. `make build` (Anchor, building the whole workspace) gives different binaries, for example about 450 KB for riff-dbc. CI and devnet use that build.

Still to do at launch: `solana-verify verify-from-repo` against the deployed programs, once they exist.

## Questions for the audit

1. Can any sequence of instructions move a coin's fees to the wrong party, or to the right party twice?
2. Is the account handling sound, given how much is left for Meteora to check?
3. Is the late-claim design sound? Is there a way to identify the migration's creator position on chain?
4. Can a transaction get around the launch-buy cap in a way that matters beyond the documented cases?
5. Is the passkey verification sound (precompile parsing, challenge binding, replay)?
6. Can recovery, veto, revocation or reissue be abused to take a vault?
7. What happens to riff if Meteora upgrades DBC or DAMM v2?

## Candidate firms

Solana-specialist auditors to request quotes from (availability and
pricing not checked; ask each for a Solana/Anchor track record):
OtterSec, Neodyme, Zellic, Sec3, Halborn, Trail of Bits, Ackee Blockchain
Security, Accretion.

Ask each for: timeline, cost, whether a fix-review round is included, and
whether the report will be public.
