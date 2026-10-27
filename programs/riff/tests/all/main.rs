//! Every integration test in one binary. Each test file used to be its own
//! binary that linked the whole Solana test runtime (LiteSVM and the SPL,
//! Raydium programs it loads); linking it once makes the test build
//! much faster. Run one file's tests with `cargo test -p riff --test all <file>::`.

#[path = "../common/mod.rs"]
mod common;

mod curve_vectors;
mod test_admin;
mod test_artist;
mod test_create_coin;
mod test_graduation;
mod test_initialize_config;
mod test_launch;
mod test_payouts;
mod test_protocol_fees;
mod test_stats;
mod test_trade;
