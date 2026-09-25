# riff

Solana programs for riff, a community-coin launch protocol for music.

> Riff is in research and development. It is not live and has not launched a token.
> Official launches are only announced by [@useRiffPad](https://x.com/useRiffPad).

## Layout

```
programs/riff/      Anchor program (Rust)
programs/riff/tests LiteSVM integration tests
app/                client (later)
```

## Development

Develop inside WSL2 (Ubuntu) or Linux. Keep the checkout on the Linux filesystem
(e.g. `~/riff`), not `/mnt/c` or `/mnt/f`, or builds get very slow.

Toolchain: Rust (pinned by `rust-toolchain.toml`), Agave/Solana CLI, Anchor via `avm`.

```bash
make build   # anchor build --arch v0
make test    # build, then cargo test (LiteSVM, no validator needed)
make lint
```

## License

Proprietary. All rights reserved, © 2026 riff. See [LICENSE](LICENSE).
