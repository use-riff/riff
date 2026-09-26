# riff: audit package

Everything an external auditor needs to quote and start. Send this with
access to the repository.

## What riff is

A community-coin launchpad for music on Solana. Anyone launches a coin
tied to an artist (as unverified text until the artist claims it). Coins
trade on a bonding curve; when the curve sells out, liquidity moves to a
Raydium pool and is locked. A share of every trading fee goes to the
artist, or to charity if the artist never claims.

Not deployed yet. Real user funds will be held by the program.

## Scope

- **Program:** `programs/riff/src/` — Anchor 1.2.0, Rust 1.89, built for
  SBPF v0. ~2,400 lines, 13 instructions:
  `initialize_config`, `update_config`, `transfer_admin`, `accept_admin`,
  `create_coin`, `buy`, `sell`, `graduate`, `claim_artist`,
  `withdraw_artist_fees`, `withdraw_creator_fees`, `collect_protocol_fees`,
  `sweep_charity_fees`.
- **Commit:** to be fixed once the open PRs are merged into `main`
  (artist claim, graduation, launch readiness).
- **Out of scope:** Anchor, Token-2022, the ATA program, Raydium CPMM
  itself, and all off-chain code (app, indexer, verification server).

## Architecture in brief

| Account | Seeds | Holds |
|---|---|---|
| `Config` | `["config"]` | Admin, treasury, charity, verifier, fee rates, curve settings, Raydium fee tier |
| `Coin` | `["coin", mint]` | Curve reserves and all fee balances; its lamports back them |
| Vault | `["vault", mint]` | Token-2022 account holding unsold curve tokens + the graduation reserve |
| Graduation authority | `["graduation", mint]` | System-owned; holds SOL only during `graduate` |
| Pool address | `["pool", mint]` | Signs Raydium pool creation; Raydium owns it afterwards |

- **Mint:** Token-2022, 6 decimals, 1B fixed supply, only MetadataPointer
  + TokenMetadata extensions, mint and freeze authority revoked at launch.
- **Curve:** constant product over virtual reserves; the starting virtual
  token reserve is derived so the graduation pool opens at the curve's
  final price (`src/curve.rs`).
- **Fees:** 1% per trade by default (0.5% artist, 0.2% creator, 0.3%
  protocol), accrued on the coin and paid out by separate instructions.
- **Payouts:** every lamport leaving a coin goes through
  `payout::pay_from_coin`, which re-checks solvency afterwards.
- **Graduation:** permissionless CPI into Raydium CPMM; LP tokens are
  burned.

## Invariants we rely on

1. `coin.lamports ≥ rent + real_sol + artist_fees + charity_fees + creator_fees + protocol_fees`
   (equality unless someone donates SOL).
2. `virtual_sol − real_sol` and `virtual_token − real_token` are constant.
3. `k = virtual_sol × virtual_token` never decreases.
4. The curve can always repay every circulating token.
5. Graduation deposits exactly `real_sol_reserves` and exactly the
   graduation reserve, whatever the account balances.

## Trust assumptions

- **Admin** (intended: Squads multisig) can redirect the treasury and
  charity, swap the verifier and the Raydium fee tier, and hand over the
  admin role. It can't touch curve SOL or change fee rates.
- **Upgrade authority** (intended: the same multisig, time-locked) can do
  anything.
- **Verifier** can link unclaimed coins to wallets.

## Known issues and accepted risks

From the internal review (`SECURITY_REVIEW.md` on branch `security/review`;
it should be updated and merged before the audit):

- Creators can still buy from other wallets from the slot after launch;
  only same-slot bundling is blocked.
- Events use `emit!` (plain logs), which other programs can imitate.
- No pause instruction.
- No way to undo a fraudulent artist claim.
- Coin metadata authorities are held by the coin PDA rather than revoked.
- Slippage limits may be zero; the client is expected to set them.
- Tests run on LiteSVM, which skips Solana's rent check for accounts
  without data; that rule is tested explicitly instead. Raydium is tested
  against its real mainnet program, snapshotted in
  `programs/riff/tests/fixtures/`.

## Running the tests

Linux or WSL with Rust (pinned by `rust-toolchain.toml`), Agave/Solana CLI
4.1.2 and Anchor CLI 1.2.0:

```bash
make build   # SBPF v0
make test    # 93 tests, no validator needed
make lint
```

`tests/security_findings.rs` (on `security/review`) holds one test per
internal finding; each should pass once its fix is merged.

## Questions we'd like the audit to answer

1. Can any sequence of instructions move SOL or tokens owed to one party
   to another, or below rent?
2. Is the graduation CPI into Raydium safe against account substitution,
   front-running and griefing?
3. Are the curve math and every rounding direction correct?
4. Is the verifier co-signature design sound?
5. Anything that would let a coin impersonate another after launch.

## Candidate firms

Solana-specialist auditors to request quotes from (availability and
pricing not checked; ask each for a Solana/Anchor track record):
OtterSec, Neodyme, Zellic, Sec3, Halborn, Trail of Bits, Ackee Blockchain
Security, Accretion.

Ask each for: timeline, cost, whether a fix-review round is included, and
whether the report will be public.
