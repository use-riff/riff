# Artist Passport

A public, hack-resistant identity for musicians on Solana.

## The problem

- Anyone can launch a "$DRAKE" coin, and nothing on-chain says whether the
  artist is behind it.
- One hacked Instagram account has been enough to push a fake artist coin.
- An artist's payout wallet is one lost seed phrase, or one phishing link,
  away from being gone.

## How the passport works

1. **Several proofs, at least one strong.** A passport needs proofs from
   at least two different sources, and one of them must be strong.

   | Proof | How riff checks it | Strength |
   |---|---|---|
   | Email from Spotify for Artists | Spotify's DKIM signature (from `artists.spotify.com`), and the artist link inside | strong |
   | Code in the Spotify bio | a code tied to the wallet, on the artist's public Spotify page; only their Spotify for Artists account can edit the bio | strong |
   | Email from the distributor (DistroKid…) | the distributor's DKIM signature, and a release in it that is on the artist's Spotify profile | medium |
   | Email from Apple Music for Artists | Apple's DKIM signature, and an Apple Music artist whose releases match the artist's Spotify releases | medium |
   | Official website | a DNS record | medium |
   | Official YouTube channel | Google login | medium |
   | Instagram, TikTok, X | login or a post | weak |

   Both Spotify proofs come down to the same Spotify for Artists account, so
   they count as one source: a passport also needs another one, such as the
   distributor, Apple Music for Artists or a website. riff's verifier checks each proof off-chain and records
   it with `record_proof`, co-signed by the wallet it's for. Each proof is
   stored per wallet, so nobody can block an artist by proving first. The
   email itself never goes on-chain: only a hash of its signature does.

2. **A passkey as a second factor, checked by Solana.** When the passport is
   issued, the artist registers a passkey: Face ID, a fingerprint, Windows
   Hello or a security key. After that, the following need the wallet *and*
   the passkey:

   - changing the wallet or the passkey;
   - endorsing a coin;
   - adding proofs;
   - withdrawing more than a small daily amount.

   The signature is verified by Solana's secp256r1 precompile in the same
   transaction. The program checks five things:

   - the precompile checked this passport's passkey;
   - the challenge is the hash of this exact action and the passport's nonce, so no signature works twice;
   - the relying-party hash is riff's site, so a signature from a phishing domain fails;
   - the user was present;
   - the user was verified.

3. **Recovery with a time-lock and a veto.** If the artist loses a wallet or a passkey:
   - A new wallet with fresh proofs (again two sources, one strong) and a new passkey can request a recovery.
   - The recovery waits out a public time-lock (72 hours on mainnet).
   - During the time-lock, the old passkey or any *guardian* can veto it. A guardian is a fresh proof of a kind already on the passport, such as the artist's YouTube. A stolen wallet alone can do nothing.

4. **Earnings belong to the passport.** `claim_coin` claims a riff coin with
   the passport's vault as the coin's artist. The vault is a program account
   that only this program can pay out of: riff's `claim_artist` and
   `withdraw_artist_fees` are called with the vault signing through its seeds.
   A lost wallet never loses the artist's money.

5. **Probation for new passports.** A passport made from a hacked inbox
   would pass the proofs: most artists get their Spotify, distributor and
   Apple emails in one inbox. So a new passport waits out the same period as
   a recovery (72 hours on mainnet) before it can `endorse` or `withdraw`.
   It can still `disavow` and `claim_coin` (claiming only moves money into
   the vault). If it was a takeover, riff revokes it in that window, and
   `reissue_passport` gives it to the real artist: riff's admin (a multisig
   with a time-lock on mainnet) and the artist's wallet sign, with fresh
   proofs and a new passkey. The vault stays, so what was claimed is the
   artist's, and the reissued passport starts a new probation.

6. **Endorsements anyone can read.** `endorse` (wallet and passkey) and `disavow` (wallet alone: always safe) write one account per coin mint, on any launchpad. It lives at `["endorse", passport, mint]`, so any wallet, explorer or launchpad can check whether the verified artist stands behind a coin. An endorsement (or disavowal) counts only while the passport isn't revoked and if its `updated_at` is at or after the passport's `issued_at`: a reissue voids whatever the previous holder wrote. Apps may also flag passports younger than a week as new.

## Accounts

| Account | Seeds | Holds |
|---|---|---|
| `PassportConfig` | `["config"]` | admin, riff's verifier, the passkey site hash, time-lock length, free daily withdrawal, proof max age |
| `ProofRecord` | `["proof", artist_id, wallet, kind]` | one verified proof: kind, hash of what was checked, and when |
| `Passport` | `["passport", artist_id]` | wallet, passkey, proofs, nonce, pending recovery, revoked flag, daily withdrawal window |
| `Vault` | `["vault", passport]` | the artist's fees from every coin claimed through the passport |
| `Endorsement` | `["endorse", passport, mint]` | endorsed or disavowed, and when |

## Instructions

| Instruction | Who |
|---|---|
| `initialize_config`, `update_config` | the program's upgrade authority, then the admin |
| `record_proof` | the wallet plus riff's verifier |
| `issue_passport` | the wallet, with 2 or more fresh proofs and a passkey signature |
| `add_proofs`, `set_wallet`, `set_passkey` | the wallet plus the passkey |
| `endorse` | the wallet plus the passkey, after the probation |
| `disavow` | the wallet |
| `request_recovery` | a new wallet, with fresh proofs and a new passkey |
| `veto_recovery` | the passkey, or a guardian proof |
| `finalize_recovery` | anyone, after the time-lock |
| `claim_coin` | the wallet plus riff's verifier (as riff requires) |
| `collect_fees` | anyone; the money can only go to the vault |
| `withdraw` | after the probation: the wallet up to the daily amount; the passkey for more |
| `revoke_passport` | the admin, publicly logged |
| `reissue_passport` | the admin plus the artist's wallet, with fresh proofs and a new passkey; only for a revoked passport |
| `reset_passport` | **devnet build only** (`--features devnet`): the admin closes a passport and its vault so a demo can issue it again; absent from the mainnet build |

## Limits

- Artists share access with their team, so "the artist" means the artist or
  their official team.
- Proofs trust riff's verifier to check them. The passkey, the time-lock,
  the vault and endorsements don't: the program enforces them. A
  zero-knowledge version of the email proof, verified on-chain, would
  remove that trust for the strongest proof.
- A guardian can veto a legitimate recovery. That costs time, not money: the
  artist can prove again once the guardian account is secured, and the admin
  can revoke a compromised passport.
- Coins claimed with a plain wallet before the passport existed stay with
  that wallet.
- Anyone with a copy of a Spotify for Artists email (the artist's team, or
  someone it was forwarded to) could use it. That's why a passport needs a
  second source and a passkey.
- Someone who controls the artist's inbox can usually pass every email
  proof, and reset the Spotify login to set the bio code. The two-source
  rule doesn't stop that. The probation does: the passport can't vouch for a
  coin or pay out until its waiting period ends, riff's verifier reviews
  claims on new passports and large claims before co-signing them, and riff
  can revoke and reissue. An artist who already has a passport is protected
  by their passkey and the recovery time-lock.

## Tests

`cargo test -p riff-passport` (after `make build`) runs the program in LiteSVM
with riff deployed next to it.

- The passkeys are real P-256 keys that sign the way browsers do, including
  the authenticator data and the client data JSON.
