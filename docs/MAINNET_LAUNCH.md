# Mainnet parameters, keys and launch steps

A plan, not a record: **nothing here has been run on mainnet.** Every step that spends SOL or changes an authority needs the owner's go-ahead at the time.

## Parameters

| Parameter | Value | Where | Why |
|---|---|---|---|
| Curve | 30 → 300 SOL market cap, about 72 SOL in the curve at graduation | `config/gen-config.mjs` | Fixed in riff's Meteora config once created. A different curve needs a new config |
| Trade fee | 2%: 0.8% artist, 0.8% partner, 0.4% Meteora | `gen-config.mjs` | Fixed the same way |
| Graduated pool | DAMM v2, 1% in SOL, all liquidity locked 50/50 | `gen-config.mjs` | Fixed the same way |
| Launch fee | 0.1 SOL a coin, to the treasury | `LAUNCH_FEE_LAMPORTS` in `programs/riff-dbc/src/lib.rs` | Funds promotion and puts a price on spam. A different amount needs a program upgrade |
| Launcher share | 3750 bps of the partner share (0.3% of a trade) | `mainnet.params.json` | Admin can change it for new coins; each coin keeps the share it was created with |
| `max_launch_buy` | 1 SOL | `mainnet.params.json` | Top-level swaps in the creating transaction only, so not a security control: see the audit package |
| Claim window | 90 days (7,776,000 s) | `mainnet.params.json` | Decided 2026-09-27 for the old program (`riff-site/deploy/mainnet.json`), and kept. New coins only if changed. At most a year (`MAX_CLAIM_WINDOW_SECS`) |
| Passport recovery and probation | 72 hours (259,200 s) | `riff-site/deploy/mainnet.json` | As planned. The program refuses less than 24 hours (`MIN_RECOVERY_DELAY`) |
| Passport free withdrawal | 0.1 SOL a day without the passkey | same | Devnet's value. **Owner to confirm.** At most 1 SOL (`MAX_FREE_WITHDRAW_PER_DAY`) |
| Passport proof age | 7 days | same | Devnet's value. At most 30 days (`MAX_PROOF_MAX_AGE`) |
| Passkey site | `riffpad.fun` | same | Passkeys only work on this site and its subdomains from then on, at the top level of the page. **It can't be changed after initialization** (every passkey is bound to it). Don't serve anything untrusted on a `riffpad.fun` subdomain |

Keeper check: Meteora's docs list 10 SOL as the minimum threshold for their keeper to migrate a SOL pool, and it runs on mainnet only. riff's is about 72. Not yet observed on riff's own config; the crank steps in after an hour either way.

## Keys

Keep them in `~/.config/riff/mainnet`, never in a repo.

| Role | What it can do | Holder |
|---|---|---|
| Upgrade authority (both programs) | Replace the code, so **everything** | **Squads vault with a time lock** (24–48 h), after launch-day handoff |
| riff-dbc admin, passport admin | Redirect treasury and charity, change the verifier; revoke and reissue passports (with the verifier, that hands a passport and its vault to a new wallet after its probation) | The same Squads vault. riff-dbc's hands over in two steps: the deployer proposes, the vault accepts in a Squads transaction. The passport's admin is whoever signs `initialize_config`, so the vault signs it (see the launch steps); after that it changes only by `propose_admin` and `accept_admin` |
| Treasury | Receives riff's fees | Separate Squads vault |
| Charity | Receives unclaimed artist fees; donates by hand | Separate Squads vault, used for nothing else |
| Verifier | Co-signs claims and proofs. Vouches for the LP position in late claims. **The most exposed key:** see below | Hot key on the server only, never in a multisig. New keys for mainnet, and **two of them**: riff-dbc's `verifier` (claims) and the passport's `verifier` (proofs) are separate config fields, so give them separate keys, ideally in a KMS or HSM that signs without exporting |
| Keeper | Pays crank network fees, no other power | Hot key (`KEEPER_SECRET_KEY`), about 1 SOL |
| Deployer | Temporary upgrade authority and admin; pays | Fresh key for launch day, emptied afterwards |
| Program keys (`riff-dbc-program.json`, `riff-passport-program.json`) | Decide the addresses, at first deploy only | Copied from `~/.config/riff/devnet`, then offline. Mainnet uses the same addresses as devnet (checked free on 2026-11-02) because `declare_id!` is compiled in |

### The verifier key

Whoever holds the passport verifier key can record proofs for any wallet. With them they can issue a passport for an artist who has none (then claim that artist's coins; the vault pays out after the 72-hour probation), or start a recovery on any passport (it finalizes after 72 hours unless the artist vetoes). The program can't tell a stolen verifier from the real one, so the defence is operational and must be in place before launch:

- **Monitoring with alerts, around the clock:** every `PassportIssued`, `RecoveryRequested` (it lists the proof kinds used), `RecoveryVetoed`, `PassportReissued` and `ArtistClaimed` from the two programs, compared with what riff's server itself signed. An event the server didn't sign means the key is out: revoke the affected passports (`revoke_passport` stops a recovery from finalizing and a vault from paying out) and rotate the key with `update_config`.
- **Tell the artist off-chain** (email and the app) when a recovery is requested on their passport, so they veto in time.
- **Rate limits and review on the server** for `record_proof`, and a second review for new passports on artists with held fees.

**Treasury and charity need WSOL accounts**: `collect_fees` and `sweep_charity` pay wrapped SOL. The crank creates them; unwrapping needs a Squads transaction.

## Cost

Mainnet rent as of 2026-11-09 (`solana rent`), for the verifiable binaries:

| Item | SOL |
|---|---|
| riff-dbc program rent (498,688 bytes) | 2.53 |
| riff-passport program rent (519,632 bytes) | 2.64 |
| Deploy buffer, one program at a time; refunded when that deploy finishes, so at most about 2.6 is out at once | about 2.6 (back) |
| riff-dbc and passport configs, fees vault, Meteora config, network and priority fees | about 0.05 (estimate) |
| Keeper float | 1.00 |
| **Have in hand** | **about 9** (8 in the deployer, 1 for the keeper), of which about 2.6 comes back. About 5.2 is spent for good |

## Before launch day (humans)

- [x] Decided: launch as an **unaudited beta** (no external audit for now). Say so everywhere; never write "audited". Keep exposure small (launch-buy cap) and the upgrade authority behind a time-locked multisig. Revisit an audit, contest or bounty when funds allow.
- [ ] Squads vaults created; signers hold separate hardware wallets; time lock set.
- [ ] Charity shortlist of organisations that accept crypto and have consented.
- [ ] Private RPC in `~/.config/riff/mainnet/rpc-url` and in Vercel.
- [x] The secp256r1 precompile (`Secp256r1SigVerify1111111111111111111111111`) is active on mainnet-beta (SIMD-0075, since epoch 800, checked 2026-11-09), and riffpad.fun normalizes passkey signatures to low S (`derToLowS` in `clients/ts/src/passkey.ts`; the precompile refuses high S).
- [ ] The verifier monitoring above (`/api/monitor` in riff-site, alerts by Telegram) is live and has fired a test alert.

## Launch-day commands

Replace `<sha>` with the commit you are deploying. Each dry run must look right before its `--yes`.

```bash
git checkout <sha>
make verifiable                                   # compare both hashes with the ones in the audit package
make deploy-dbc-mainnet                           # dry run: prints the hash
make deploy-passport-mainnet
make deploy-dbc-mainnet CONFIRM=mainnet           # costs real SOL
make deploy-passport-mainnet CONFIRM=mainnet
solana-verify verify-from-repo https://github.com/use-riff/riff --commit-hash <sha> \
  --program-id gATYd3vfHCPCrhFupj8HxVX78syyXNfPSbYRsttm9RF --library-name riff_dbc \
  --base-image solanafoundation/solana-verifiable-build@sha256:2e0b78f44ee76612e9260c7c988570c5e14de6fbd93e0ab07115ec7054473b4f
# ...and the same for riff_passport and 2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj

cd programs/riff-dbc/config && npm ci
# fill in mainnet.params.json, then:
node mainnet.mjs setup                            # dry run
node mainnet.mjs setup --yes
# Passport config: its signer becomes admin and must be the upgrade authority, so the vault takes that first and signs it.
solana program set-upgrade-authority 2nke6euXvAnbtdtcbbk8N2Z3SRLuR7i67VYmSdcY5kwj \
  --new-upgrade-authority <ADMIN VAULT> --skip-new-upgrade-authority-signer-check \
  --keypair ~/.config/riff/mainnet/deployer.json --url mainnet-beta
(cd ~/riff-site && pnpm -C scripts passport-config --cluster mainnet)        # dry run only: prints the settings
# ...then propose initialize_config with those settings in Squads, signed by the vault,
# and check the passport config's admin is the vault.

# A small real coin, to check before announcing: launch, trade, crank, claim.
# Then, while this is still the deployer's to change:
node mainnet.mjs handoff                          # dry run
node mainnet.mjs handoff --yes                    # proposes the vault as riff-dbc's admin
# ...then the Squads transaction it prints (the vault accepts) and the
# set-upgrade-authority command it prints for riff-dbc
# (the passport's went to the vault before its config).
node mainnet.mjs status
```

Then riff-site: set `RIFF_CLUSTER=mainnet`, the mainnet keys and RPC in Vercel, point the app at mainnet (`app/src/settings.ts` is still hard-coded to devnet), and promote.

**Check on mainnet right after the first coin:** that Jupiter and Axiom show and route it. This couldn't be checked before a mainnet pool exists: in a sample of other launchpads' DBC configs on 2026-11-02, Jupiter routed some and not others.
