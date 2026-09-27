# Test fixtures

Snapshots of mainnet accounts the graduation tests run against, so the
tests exercise Raydium's real deployed program rather than a mock.

| File | What | Address |
|---|---|---|
| `raydium_cpmm.so` | Raydium CPMM program (sha256 `36537be95ba356056fa38b2847d928078c68bf6cd79b875c140e157e6452cc71`) | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` |
| `amm_config_0.json` | Fee tier 0 (0.25% trade fee, 0.15 SOL pool-creation fee) | `D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2` |
| `amm_config_1.json` | Fee tier 1 (1% trade fee), used to test that other tiers are rejected | `G95xxie3XbkCqtE39GgQ9Ggc7xBC8Uceve7HFDEFApkc` |
| `create_pool_fee_receiver.json` | Raydium's pool-creation fee receiver | `DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8` |
| `wsol_mint.json` | Wrapped SOL mint | `So11111111111111111111111111111111111111112` |

Taken 2026-09-26 from mainnet-beta. Raydium CPMM is licensed Apache-2.0:
https://github.com/raydium-io/raydium-cp-swap

To refresh (read-only mainnet calls):

```bash
U=https://api.mainnet-beta.solana.com
solana program dump CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C raydium_cpmm.so --url $U
solana account D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2 --output json --output-file amm_config_0.json --url $U
solana account G95xxie3XbkCqtE39GgQ9Ggc7xBC8Uceve7HFDEFApkc --output json --output-file amm_config_1.json --url $U
solana account DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8 --output json --output-file create_pool_fee_receiver.json --url $U
solana account So11111111111111111111111111111111111111112 --output json --output-file wsol_mint.json --url $U
```

If Raydium upgrades its program or changes its fees, refresh these and
re-run `make test` before deploying.
