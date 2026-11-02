# Mainnet readiness: riff-dbc, Artist Passport, riffpad.fun

A running record of what's done, with evidence, and what's left for humans. Nothing here has touched mainnet: every mainnet step waits for the owner's go-ahead.

| # | Item | Status |
|---|---|---|
| 1 | Devnet Passport upgrade, full devnet e2e with the claim | **Done** (2026-10-27) |
| 2 | riff-dbc hardening (late-claim LP fees, events, threat review, tests) | Not started |
| 3 | Audit package for riff-dbc + riff-passport | Not started |
| 4 | riffpad.fun on riff-dbc | Not started |
| 5 | Mainnet parameters and keys plan | Not started |
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
- Found on the way: WSL's clock was minutes behind the chain's, so the probation wait uses the chain's clock.

**Left:** riffpad.fun's devnet passport pages read the old layout until riff-site syncs the new IDL (item 4). There are no devnet passports to show meanwhile.
