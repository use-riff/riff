# Security review: riff-dbc and riff-passport (November 2026)

An internal review done with Claude Code at commit `fd85fd2`. **It is not a substitute for the external audit** planned in `AUDIT_PACKAGE_DBC.md`; it is input for it.

- **Scope:** all of `programs/riff-dbc/src` and `programs/riff-passport/src` (mainnet build), about 2,000 lines, plus the Meteora config in `programs/riff-dbc/config/gen-config.mjs`. Meteora's programs, Anchor and off-chain code are out of scope, as in the audit package.
- **Method:** a line-by-line manual review against the trust model, invariants and threat notes in `AUDIT_PACKAGE_DBC.md`. riff-passport was built here with the pinned toolchain (Solana CLI 4.1.2, SBPF v0; same 481,888-byte size as the audit package) and its 25 tests pass. The two findings marked **PoC** were reproduced with LiteSVM tests against that binary (Appendix A). riff-dbc's tests weren't run here: they need Meteora's mainnet binaries.

## Summary

| ID | Severity | Program | Title |
|---|---|---|---|
| H-1 | High | passport, dbc | Whoever holds the verifier key can take passports: those not yet issued, and existing ones whose artist doesn't veto within the time-lock |
| H-2 | High | passport | The passport admin can unlock or take every vault through `update_config`, despite the trust model (**PoC**) |
| M-1 | Medium | passport | A pending recovery survives `set_wallet` / `set_passkey` / `add_proofs` and still hands over the passport (**PoC**) |
| M-2 | Medium | dbc | A claimed artist's ongoing payouts depend on the launcher keeping a WSOL account open |
| L-1 | Low | dbc | Changing `launcher_share_bps` applies to existing coins, not only new ones |
| L-2 | Low | dbc | The launch-buy cap can be bypassed with a second transaction or bundle, not only with nested calls |
| L-3 | Low | dbc | `initialize` doesn't check the `dbc_config`, which can never change afterwards |
| L-4 | Low | passport | Passkey check ignores `origin` and `crossOrigin` in `clientDataJSON` |
| L-5 | Low | passport | A single recovery slot lets an attacker with the inbox block a real recovery indefinitely |
| I-1 … I-8 | Info | both | See the end |

No finding lets an outside attacker with no keys move funds. Every High and Medium needs a privileged key (H-1, H-2), a mistake by the user (M-1), or a launcher acting in bad faith (M-2).

---

## High

### H-1. Whoever holds the verifier key can take passports

**Where:** `riff-passport/src/instructions/proofs.rs` (`record_proof`), `passport.rs` (`issue_passport`), `recovery.rs` (`request_recovery`, `veto_recovery`); `riff-dbc/src/instructions/claim_artist.rs`.

The trust model says the verifier can't "move funds alone". In practice the verifier key, a hot key on riff's service, is the only on-chain barrier to the following:

1. **Artists without a passport.** With the verifier key, an attacker records two proofs for their own wallet (`record_proof` needs only the wallet and the verifier), calls `issue_passport` with their own passkey, then claims every coin for that artist (`claim_artist` needs only the passport wallet and the verifier). The fees land in the vault, and after `recovery_delay` (72 h) `withdraw` pays them out. Only the passport admin revoking within 72 h stops it. The off-chain reviews of claims on new passports don't help, because the verifier key bypasses them.
2. **Artists with a passport.** The attacker records fresh proofs for a new wallet and calls `request_recovery`. If the artist doesn't veto within 72 h, the attacker gets the passport and the vault (with no probation afterwards, since `issued_at` doesn't change). Artists aren't expected to watch the chain.
3. **The guardian veto needs the verifier too.** A guardian's proof is a fresh `ProofRecord`, which only `record_proof` creates, so it needs the verifier's signature. If the verifier is compromised or offline, only a passkey veto is left.

**Recommendation:**
- Keep the verifier key in an HSM or KMS, and use separate keys for `record_proof` and for co-signing claims.
- Watch `PassportIssued`, `RecoveryRequested` and large `ArtistClaimed` events, with alerts and a fast path to `revoke_passport`.
- Notify the artist off-chain (email or the riff app) when a recovery is requested.
- Consider requiring an admin co-signature, or a longer time-lock, for a recovery on a passport whose vault holds more than a threshold.
- Correct the trust table so it says what the verifier can do.

### H-2. The passport admin can unlock or take any vault (PoC)

**Where:** `riff-passport/src/instructions/admin.rs`, `PassportConfigParams::apply`.

`update_config` accepts any `free_withdraw_per_day`, any `recovery_delay > 0`, any `verifier` and any `rp_id_hash`. The trust table says the passport admin "can't withdraw from a vault", but with config changes alone the admin can:

- Set `free_withdraw_per_day = u64::MAX`. From then on any passport wallet, including a stolen one, empties its vault without the passkey. **Reproduced:** `audit_poc_admin_config_disables_passkey_on_withdraw`.
- Set `verifier` to a key the admin holds and `recovery_delay = 1`, then `revoke_passport`, `record_proof` × 2 for the admin's own wallet, `reissue_passport`, wait 1 second and `withdraw`. That takes any vault in a few transactions.
- Set `rp_id_hash` to another domain, so every passkey stops working (a denial of service).

There is also no way to change `PassportConfig.admin`: a lost or compromised admin key can only be dealt with by a program upgrade.

**Recommendation:** put bounds on the config in the program: a minimum `recovery_delay` (for example 24 h), a maximum `free_withdraw_per_day`, and a minimum `proof_max_age`. Or make the security-critical values constants. Correct the trust table. Put the passport admin behind the same multisig and time-lock as the upgrade authority. Add a two-step admin transfer.

---

## Medium

### M-1. A pending recovery survives a wallet or passkey change (PoC)

**Where:** `riff-passport/src/instructions/passport.rs` (`handle_set_wallet`, `handle_set_passkey`, `handle_add_proofs`), `recovery.rs` (`handle_finalize_recovery`).

Neither `set_wallet` nor `set_passkey` nor `add_proofs` clears `passport.recovery`, and `finalize_recovery` checks nothing except the time. An artist who sees a hostile `RecoveryRequested` might well react by rotating their wallet and passkey, both approved with the passkey, rather than calling `veto_recovery`. The recovery still finalizes after the time-lock. It replaces the new wallet and passkey with the attacker's, and the attacker can withdraw everything at once. **Reproduced:** `audit_poc_pending_recovery_survives_set_wallet_and_set_passkey`. The test also shows the 4 SOL leaving the vault.

The `verified_at > wallet_since` freshness check runs only at request time, so a wallet change after the request doesn't count against it.

**Recommendation:**
- In `set_wallet` and `set_passkey`, set `passport.recovery = None` and emit `RecoveryVetoed`. They already need the passkey, the same authority as a veto. Consider doing the same in `add_proofs`.
- As a backstop, have `finalize_recovery` require `recovery.requested_at >= passport.wallet_since`.

### M-2. A claimed artist's payouts depend on the launcher's WSOL account

**Where:** `riff-dbc/src/instructions/collect_fees.rs`, `collect_lp_fees.rs`.

Once a coin is claimed, `collect_fees` and `collect_lp_fees` are the only instructions that move the creator share and the creator LP fees into the artist vault. `claim_artist` runs once, and `sweep_charity` refuses claimed coins. Both instructions also require `launcher_quote_account`: an existing WSOL account owned by `coin.launcher`. If the launcher closes theirs, the artist's payouts stop until a crank recreates the account. The launcher can then close it again and keep the rent, about 0.002 SOL each time. The audit package says this "blocks only the launcher's and treasury's split ... not a claim or sweep". That's true, but it does block every later payout to a claimed artist.

**Recommendation:** split the creator side into its own permissionless instruction that needs no launcher account. Alternatively, when the launcher's account is missing, send the launcher's part to the treasury or hold it in a per-coin PDA.

---

## Low

### L-1. Changes to the launcher share apply to existing coins

`split_partner_share` reads `config.launcher_share_bps` when fees are collected, not when the coin was created. The admin can cut every existing launcher's 0.3% to 0. The README's "(for new coins)" and the trust table read as if this only applies to new coins. **Fix:** save the share in `Coin` at creation, or correct the docs.

### L-2. The launch-buy cap is easy to bypass

`check_launch_buy` reads only the top-level instructions of the creating transaction. Apart from the nested-call case already documented, a launcher can buy in a second transaction of a Jito bundle, in the next slot, or from other wallets. The cap therefore only limits a buy in the same transaction, and it isn't a security control. **Fix:** document it as such. If riff wants to limit sniping, it needs a time-based limit (such as Meteora's fee scheduler, or a time window of riff's own) rather than a check on the transaction's instructions.

### L-3. `initialize` doesn't check the DBC config it stores

`dbc_config` can never change after `initialize`, but nothing checks that the config has `fee_claimer == fee_authority` PDA, WSOL as the quote mint, `collectFeeMode = QuoteToken`, `creatorTradingFeePercentage = 50`, migration fee 0, leftover 0 and an immutable token authority. A mistaken config would send partner fees elsewhere or lock funds, and fixing it would need an upgrade. **Fix:** deserialize Meteora's `PoolConfig` in `initialize` and assert the fields that riff's invariants depend on.

### L-4. The passkey check ignores `origin` and `crossOrigin`

`passkey::verify` checks the rpId hash, the UP and UV flags, `type` and `challenge`, but not `origin` or `crossOrigin`. That accepts any origin allowed to use the `riffpad.fun` rpId: every subdomain (preview deployments, user content), related origins, native apps, and cross-origin iframes. **Fix:** reject `crossOrigin == true`. Store the allowed origin hash (or hashes) in the config and check them.

### L-5. One recovery slot can be held by an attacker

Only one recovery can be pending at a time (`RecoveryPending`). Take an artist who has lost both their wallet and their passkey, and an attacker who controls the artist's inbox. The attacker can request a recovery first. After each guardian veto they can request again, possibly in the same block, so the real artist's request never lands. Only the admin's revoke-and-reissue gets out of this loop. **Fix:** let a guardian veto also start the vetoer's own recovery atomically, or let a guardian-backed request replace a pending one.

---

## Informational

- **I-1.** The `Register { passkey }` challenge is the same for `issue_passport`, `request_recovery`, `set_passkey` and `reissue_passport` at a given nonce. A `set_passkey` that is visible in flight could have its new-passkey proof reused in a `request_recovery`, though the attacker would end up with a passkey they don't hold. Separate them with one challenge variant per purpose.
- **I-2.** Anyone can create DBC pools directly on riff's DBC config. Their partner fees and partner LP positions go to `fee_authority` and can never be collected, since no `Coin` exists for them. Aggregators that identify "riff coins" by config key will also list these pools, which have no artist escrow.
- **I-3.** `claim_window_secs` has no upper bound. A very large value makes `now + claim_window` overflow, so `create_coin` panics for everyone.
- **I-4.** When a passport is revoked and never reissued, the fees of coins already claimed into its vault stay there for good. Nothing sends them to charity.
- **I-5.** `finalize_recovery` doesn't void endorsements or disavowals made by the previous holder. Readers only compare against `issued_at`.
- **I-6.** Before mainnet, confirm the secp256r1 precompile is active on mainnet-beta and that the client normalizes signatures to low-S.
- **I-7.** `is_migrated` reads a fixed offset taken from the vendored IDL. A layout change in a Meteora upgrade would make late claims fail or skip the LP collection. This is already in the documented Meteora risk; worth adding to monitoring.
- **I-8.** `withdraw` is allowed while a recovery is pending. A thief holding the current wallet can take the free daily amount throughout the time-lock (3 × 0.1 SOL with the mainnet settings).

## Checked and found sound

- **PDA binding.** In every riff-dbc instruction, `base_mint` binds `coin`, escrow, artist vault and unwrap through seeds. `pool` is bound to `coin.pool`, and the escrow's base account to its Token-2022 ATA. The passport and its vault are derived from `coin.artist_id` with `seeds::program = riff_passport::ID`.
- **Signer seeds.** Every CPI signed by the escrow or the fee authority goes to a fixed program (`Program<DBC>`, `Program<CpAmm>`, `Program<Token>`, `Program<Token2022>`) and a fixed instruction, with the receiving account bound. I found no way to redirect a signature or substitute another coin's accounts. Positions from another coin fail Meteora's token-A check, because `base_mint` is bound.
- **Partner split.** The split takes the before/after difference of the fees vault, so donated funds or other coins' balances are never split. Release builds have overflow checks on.
- **Claims and sweeps.** Claim and sweep can't both pay out the same fees: `coin.artist` is set once, and the deadline comparisons are consistent (`> deadline` for late claims and sweeps).
- **WSOL unwrap.** `pay_artist_sol` closes a native account that holds a balance (allowed for native accounts), then forwards exactly `amount`. A pre-funded unwrap PDA works with Anchor's `init`.
- **Initialization.** Both programs' config initialization is gated on the upgrade authority, with a correct `programdata_address` check.
- **Precompile parsing.** The 14-byte offset layout is read correctly. Only entries with all their data inline count (`u16::MAX` indices), and the key and message must match byte for byte. The challenge covers the domain, the passport, the nonce and the Borsh-encoded action, and every passkey action increments the nonce.
- **Proofs.** `fresh_proofs` checks the owner and discriminator (through `Account::try_from`), the artist, the wallet, the age, and one proof per kind. The source bitmask counts the two Spotify proofs as one source.
- **Vault withdrawals.** `withdraw` keeps the vault's rent, and money can only go to the passport wallet.
- **Devnet reset.** `reset_passport` is behind `cfg(feature = "devnet")` and absent from the published IDL.

## Appendix A: PoC tests

Append to `programs/riff-passport/tests/all/test_vault_recovery.rs`, then run `make build && cargo test -p riff-passport --test all audit_poc`. Both pass on `fd85fd2`, which shows each issue. Once fixed, the first should fail at `assert!(p.recovery.is_some())` and the second at `UpdateConfig` (if bounds are added).

```rust
/// The artist notices a hostile recovery and rotates their wallet and
/// passkey (both passkey-approved) instead of vetoing. The pending recovery
/// survives and still hands the passport to the attacker.
#[test]
fn audit_poc_pending_recovery_survives_set_wallet_and_set_passkey() {
    let mut env = setup();
    let (wallet, passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    env.svm().airdrop(&vault_address(&passport), 5 * SOL).unwrap();

    // Attacker (e.g. with the artist's inbox) starts a recovery.
    let attacker = funded_keypair(env.svm());
    let attacker_passkey = TestPasskey::new();
    request_recovery(&mut env, &attacker, &attacker_passkey, &DEFAULT_KINDS).unwrap();

    // Artist rotates wallet with wallet + passkey.
    let fresh_wallet = funded_keypair(env.svm());
    let (proof, pre) = passkey.approve(
        &passport,
        nonce(&env),
        &PasskeyAction::SetWallet { new_wallet: fresh_wallet.pubkey() },
    );
    let set_wallet = ix(
        riff_passport::instruction::SetWallet { passkey_proof: proof },
        riff_passport::accounts::SetWallet {
            act: act_accounts(&wallet.pubkey()),
            new_wallet: fresh_wallet.pubkey(),
        },
    );
    send_many(env.svm(), &[pre, set_wallet], &[&wallet, &fresh_wallet]).unwrap();

    // ...and rotates the passkey too.
    let next = TestPasskey::new();
    let n = nonce(&env);
    let (old_proof, pre_old) = passkey.approve(
        &passport, n, &PasskeyAction::SetPasskey { new_passkey: next.public() });
    let (new_proof, pre_new) = next.approve(
        &passport, n, &PasskeyAction::Register { passkey: next.public() });
    let set_passkey = ix(
        riff_passport::instruction::SetPasskey { new_passkey: next.public(), old_proof, new_proof },
        act_accounts(&fresh_wallet.pubkey()),
    );
    send_many(env.svm(), &[pre_old, pre_new, set_passkey], &[&fresh_wallet]).unwrap();

    let p: riff_passport::Passport = fetch(&env.svm, &passport);
    assert!(p.recovery.is_some(), "recovery should still be pending");

    // Time-lock passes; anyone finalizes; attacker owns the passport.
    advance_time(env.svm(), RECOVERY_DELAY);
    let anyone = funded_keypair(env.svm());
    send(env.svm(), finalize_ix(), &[&anyone]).unwrap();
    let p: riff_passport::Passport = fetch(&env.svm, &passport);
    assert_eq!(p.wallet, attacker.pubkey());
    assert_eq!(p.passkey, attacker_passkey.public());

    // Attacker drains the vault with their own passkey.
    let before = lamports(&env.svm, &attacker.pubkey());
    let (proof, pre) = attacker_passkey.approve(
        &passport, nonce(&env), &PasskeyAction::Withdraw { to: attacker.pubkey(), amount: 4 * SOL });
    send_many(env.svm(), &[pre, withdraw_ix(&attacker.pubkey(), 4 * SOL, Some(proof))], &[&attacker]).unwrap();
    assert!(lamports(&env.svm, &attacker.pubkey()) >= before + 4 * SOL - 10_000);
}

/// The passport admin, with update_config alone, turns off the passkey
/// requirement on withdrawals for every vault.
#[test]
fn audit_poc_admin_config_disables_passkey_on_withdraw() {
    let mut env = setup();
    let (stolen_wallet, _passkey) = issued(&mut env);
    let passport = passport_address(ARTIST_ID);
    env.svm().airdrop(&vault_address(&passport), 5 * SOL).unwrap();

    let admin = env.admin.insecure_clone();
    let mut params = config_params(&env);
    params.free_withdraw_per_day = u64::MAX;
    let update = ix(
        riff_passport::instruction::UpdateConfig { params },
        riff_passport::accounts::UpdateConfig { admin: admin.pubkey(), config: config_address() },
    );
    send(env.svm(), update, &[&admin]).unwrap();

    // The wallet alone now empties the vault: no passkey.
    send(env.svm(), withdraw_ix(&stolen_wallet.pubkey(), 4 * SOL, None), &[&stolen_wallet]).unwrap();
}
```
