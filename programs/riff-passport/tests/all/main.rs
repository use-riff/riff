//! Every integration test in one binary. Each test file used to be its own
//! binary that linked the whole Solana test runtime (LiteSVM and the SPL,
//! Raydium and riff programs it loads); linking it once makes the test build
//! much faster. Run one file's tests with `cargo test -p riff-passport --test all <file>::`.

#[path = "../common/mod.rs"]
mod common;

mod test_identity;
mod test_takeover;
mod test_vault_recovery;
