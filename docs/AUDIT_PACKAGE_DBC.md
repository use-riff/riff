# riff-dbc + riff-passport: audit package

Everything an external auditor needs to quote and start. **Neither program has been audited.** Humans choose and hire the auditor; `AUDIT_PACKAGE.md` covers the old `programs/riff`, which this replaces (candidate firms and what to ask them are listed there).

## What it is

A launchpad for music-artist community coins on Solana. Meteora's Dynamic Bonding Curve (DBC) runs the curve, trading and graduation. `riff-dbc` creates each coin's pool, holds the artist's share of fees until the artist claims with an Artist Passport, sends it to charity if they don't, and splits riff's share with whoever launched the coin. `riff-passport` is the artist's identity and the vault their earnings go to. Real user funds will pass through both.

Read first: `programs/riff-dbc/README.md`, `programs/riff-passport/README.md`.

## Scope

| Program | Source | Instructions |
|---|---|---|
| riff-dbc | `programs/riff-dbc/src/` | `initialize`, `update_config`, `create_coin`, `collect_fees`, `collect_lp_fees`, `claim_artist`, `pay_artist`, `sweep_charity` |
| riff-passport | `programs/riff-passport/src/` (mainnet build, without the `devnet` feature) | see its README's table |

- **Commit:** the `main` commit named in the engagement. Anchor 1.2.0, Rust 1.89, Solana CLI 4.1.2, SBPF v0.
- **Out of scope:** Meteora's DBC and DAMM v2 (behaviour relied on is listed below and worth checking), Anchor, the token programs, all off-chain code. `programs/riff` is being retired: see "Known limits" for how riff-passport still depends on it.

## Accounts (riff-dbc)

| Account | Seeds | What it is |
|---|---|---|
| `Config` | `["config"]` | Admin, verifier, charity, treasury, DBC config, launcher share, launch buy cap, claim window |
| `Coin` | `["coin", mint]` | Pool, launcher, artist ID and name, `artist` (the passport vault once claimed), deadline, totals |
| Escrow | `["escrow", mint]` | No data. The pool's creator for good, owner of the creator LP position, authority of the artist vault |
| Artist vault | `["artist_vault", mint]` | WSOL account: the artist's share until paid or swept |
| Unwrap | `["unwrap", mint]` | WSOL account that lives for one instruction, to turn WSOL into SOL |
| Fee authority | `["fee_authority"]` | No data. The DBC config's fee claimer, owner of every coin's partner LP position |
| Fees vault | `["fees_vault"]` | WSOL account the partner share passes through, shared by all coins |

## Money flow

Trade fee 2%: Meteora 0.4%, artist 0.8%, launcher 0.3%, riff 0.5%. After graduation: 1% LP fee in SOL, on two permanently locked positions (50/50). The creator side follows the artist's route, the partner side the launcher and riff's split.

## Trust model

| Party | Can | Can't |
|---|---|---|
| **Upgrade authority** (planned: multisig with time-lock) | Anything, by upgrading either program | |
| **riff-dbc admin** (planned: same multisig) | Redirect the treasury and charity (applies to all later payouts, including already-unclaimed coins' fees); change the verifier, launcher share, launch buy cap, claim window (new coins only). Admin transfer is one step, with no acceptance: a wrong address needs a program upgrade to fix | Change the DBC config; touch vaults directly; change a coin's deadline or artist |
| **Verifier** (riff's hot service) | Co-sign claims and record passport proofs, so it decides who can get a passport. **Vouches for `creator_position` on late claims.** Colluding with a wallet, it can issue a false passport, limited by probation and revocation | Move funds alone: claims also need the passport's wallet, and fees only reach the passport vault or charity |
| **Passport admin** | Revoke a passport, and reissue it with the artist's wallet and fresh proofs | Withdraw from a vault |
| **Launcher** | Pick any artist ID in the valid format for their coin (selection ≠ affiliation); close their WSOL account (see griefing below) | Change anything after creation |
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
5. Partner share of a coin: exactly `launcher_share_bps` (rounded down) to that coin's launcher, the rest to the treasury. Only the amount that just arrived counts, so another coin's or a donated balance in the shared fees vault is never split.
6. A coin's fees can't be credited to another coin by substituting accounts.
7. The vault address depends only on the artist ID, so it survives recovery, revocation and reissue.
8. On a mainnet build, `reset_passport` doesn't exist, so a passport or vault is never closed.

## Threat notes by instruction (riff-dbc)

**`initialize`:** only the upgrade authority, once (`init`). Config parameters aren't range-checked beyond share and window: admin trust.

**`update_config`:** admin only. See trust model. `dbc_config` can't change, since every coin depends on it.

**`create_coin`:**
- Anyone can launch for any artist ID in the passport format; the coin's name and symbol are free text. Impersonation is handled by claims and endorsements, not here.
- `check_launch_buy` reads top-level instructions only. A swap nested in another program, or `create_coin` called by CPI, escapes the cap. Those buys pay the same fees. Documented, not treated as security.
- It looks at Meteora instructions naming the pool as their third account (where swap and swap2 have it); only `swap` is allowed, and its amount counts. A sell counts its token amount against the cap, which only makes it stricter.

**`collect_fees`, `collect_lp_fees`:** permissionless. Receivers are bound to `coin.launcher` and `config.treasury`.
- *Griefing:* the launcher's account must exist, and a launcher can close it, so a crank that recreates it pays rent the launcher can take back on each close. The crank should cap this. It blocks only the launcher's and treasury's split in that call, not a claim or sweep of DBC fees.
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
- The hand-rolled secp256r1 instruction parsing in `passkey.rs`, and the JSON parsing of `clientDataJSON` (`type` and `challenge` only; origin isn't checked, rpId hash is).
- `withdraw` moves lamports directly, keeping the vault's rent.
- Guardian veto and proof freshness rules: `recovery.rs`.

## Known limits

1. **Late claim LP fees rely on the verifier.** Nothing on chain tells the migration's creator position from a dust one handed to the escrow (`create_position` takes any owner; NFT accounts have no immutable owner). A late claim passing another position leaves the real one's fees to the artist. The verifier co-signs, so it must pass the right position. Bounded by how often the crank collects.
2. **Late claim size:** it fits a plain transaction without a compute-budget instruction (asserted in a test, and it landed on devnet); with one it is 1239 bytes. A priority fee needs a lookup table.
3. **The launch-buy cap** covers top-level swaps only.
4. **No anti-sniping fee.** Meteora's first-swap exemption doesn't work for pools created by CPI.
5. **No pause** in riff-dbc. Trading is Meteora's; riff's instructions can only be stopped by an upgrade.
6. **One-step admin transfer** in `update_config`.
7. **`emit!` events** are plain logs that other programs can imitate. Readers must check the program ID of the emitting instruction.
8. **riff-passport still depends on the old `riff` program**: `claim_coin` and `collect_fees` CPI into it, and it isn't planned for mainnet. Removing them shrinks the binary and the scope, and needs the owner's decision (readiness item 7).
9. **Fuzzing:** a deterministic per-account substitution test exists; no coverage-guided fuzzing has been run.

## Tests

```bash
make dbc-fixtures   # Meteora's binaries, hash-checked
make test           # riff-dbc: 21 tests in LiteSVM against Meteora's real programs
make lint
```

riff-dbc's tests write Passport accounts directly; `cargo test -p riff-passport` (27 tests) covers issuing them. LiteSVM doesn't enforce the transaction size limit: the largest claim is asserted in a test, and `node programs/riff-dbc/config/devnet.mjs e2e [--late]` runs the flows on a real validator (evidence in `MAINNET_READINESS.md`).

## Build reproducibility

The toolchain is pinned: `rust-toolchain.toml`, `Cargo.lock`, and CI pins Solana CLI and Anchor by hash.

`make verifiable` builds both programs with `solana-verify` in Docker, using the Solana Foundation's 4.1.2 image pinned by digest. **This is the build to audit and deploy.** At `7cab289`, from a clean clone:

| Program | Size | File sha256 | `solana-verify` executable hash |
|---|---|---|---|
| riff-dbc | 467,528 bytes | `d0ac7ecb0e2eea9566b26e6a125db999b520d82ea2ddd46996db7a5f588625a1` | `d6cba7b8dc6ff522b99957ea3ca99b7f0648319572e7f500446221093039c8b0` |
| riff-passport | 515,528 bytes | `8cc94ed640ce4ee78f886a3b432cea93352db342c59986a985be081f9522678f` | `704317afd73359807aa9721fc8803f19f4253c40844b31fe51808111ff35a203` |

riff-dbc's matched a `cargo build-sbf --manifest-path programs/riff-dbc/Cargo.toml --arch v0` run on the developer's machine byte for byte. `make build` (Anchor, building the whole workspace) gives different binaries, for example 450,064 bytes for riff-dbc. CI and devnet use that build. The riff-dbc and riff-passport test suites also pass on the verifiable binaries above (21 and 27 tests).

Still to do at launch: `solana-verify verify-from-repo` against the deployed programs, once they exist.

## Questions for the audit

1. Can any sequence of instructions move a coin's fees to the wrong party, or to the right party twice?
2. Is the account handling sound, given how much is left for Meteora to check?
3. Is the late-claim design sound? Is there a way to identify the migration's creator position on chain?
4. Can a transaction get around the launch-buy cap in a way that matters beyond the documented cases?
5. Is the passkey verification sound (precompile parsing, challenge binding, replay)?
6. Can recovery, veto, revocation or reissue be abused to take a vault?
7. What happens to riff if Meteora upgrades DBC or DAMM v2?
