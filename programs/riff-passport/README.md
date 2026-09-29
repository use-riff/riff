# Artist Passport

A public, hack-resistant identity for musicians on Solana.

## The problem

- Anyone can launch a "$DRAKE" coin, and nothing on-chain says whether the
  artist is behind it.
- One hacked Instagram account has been enough to push a fake artist coin.
- An artist's payout wallet is one lost seed phrase, or one phishing link,
  away from being gone.

## How the passport works

1. **Two strong proofs, or two sources.** A passport needs either:

   - **both Spotify proofs** (the email and the bio code), or
   - proofs from **at least two different sources**, one of them strong.

   | Proof | How riff checks it | Strength |
   |---|---|---|
   | Email from Spotify for Artists | Spotify's DKIM signature (from `artists.spotify.com`), and the artist link inside | strong |
   | Code in the Spotify bio | a code tied to the wallet, on the artist's public Spotify page; only their Spotify for Artists account can edit the bio | strong |
   | Official website | a DNS record | medium |
   | Official YouTube channel | Google login | medium |
   | Instagram, TikTok, X | login or a post | weak |

   Both Spotify proofs come down to the same Spotify for Artists account, so
   they count as one source. But they are two different strong checks (a
   signed email, and a wallet-bound code only that account can publish), so
   together they are enough. Many small or anonymous artists have no official
   YouTube channel or website to prove. YouTube, a website and social
   accounts stay optional: each adds a second source, and becomes a guardian
   (see recovery). One proof alone, or any number of medium and weak proofs
   without a strong one, is never enough. riff's verifier checks each proof off-chain and records
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
   - A new wallet with fresh proofs (the same rule as issuing: both Spotify proofs, or two sources with a strong one) and a new passkey can request a recovery.
   - The recovery waits out a public time-lock (72 hours on mainnet).
   - During the time-lock, the old passkey or any *guardian* can veto it. A guardian is a fresh proof of a kind already on the passport, such as the artist's YouTube. A stolen wallet alone can do nothing.
   - So someone holding the artist's Spotify for Artists account can *start* a recovery with the two Spotify proofs alone. We accept that on purpose: they still have to wait out the public time-lock, and the artist's passkey or any guardian can veto it in that time.

4. **Earnings belong to the passport.** `claim_coin` claims a riff coin with
   the passport's vault as the coin's artist. The vault is a program account
   that only this program can pay out of: riff's `claim_artist` and
   `withdraw_artist_fees` are called with the vault signing through its seeds.
   A lost wallet never loses the artist's money.

5. **Endorsements anyone can read.** `endorse` (wallet and passkey) and `disavow` (wallet alone: always safe) write one account per coin mint, on any launchpad. It lives at `["endorse", passport, mint]`, so any wallet, explorer or launchpad can check whether the verified artist stands behind a coin.

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
| `issue_passport` | the wallet, with enough fresh proofs (both Spotify proofs, or 2 sources with a strong one) and a passkey signature |
| `add_proofs`, `set_wallet`, `set_passkey`, `endorse` | the wallet plus the passkey |
| `disavow` | the wallet |
| `request_recovery` | a new wallet, with fresh proofs and a new passkey |
| `veto_recovery` | the passkey, or a guardian proof |
| `finalize_recovery` | anyone, after the time-lock |
| `claim_coin` | the wallet plus riff's verifier (as riff requires) |
| `collect_fees` | anyone; the money can only go to the vault |
| `withdraw` | the wallet up to the daily amount; the passkey for more |
| `revoke_passport` | the admin, publicly logged |

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
  someone it was forwarded to) could use it. That's why the email alone is
  never enough, and why a passport needs a passkey.
- **A Spotify-only passport rests on one account.** With just the two
  Spotify proofs, one compromised Spotify for Artists account (or a rogue
  team member on it) is enough to take the passport, and with it the
  artist's coin fees. The program allows it so small artists aren't locked
  out. riff limits the damage off-chain: its verifier won't co-sign
  `claim_coin` for a Spotify-only passport when the coin has earned at least
  `SPOTIFY_ONLY_REVIEW_THRESHOLD_LAMPORTS` in artist fees (1 SOL by default;
  passports with a second source keep the usual 8 SOL). The riff team
  reviews those claims first. Adding YouTube or a website removes the lower
  threshold and adds a guardian.

## Tests

`cargo test -p riff-passport` (after `make build`) runs the program in LiteSVM
with riff deployed next to it.

- The passkeys are real P-256 keys that sign the way browsers do, including
  the authenticator data and the client data JSON.
