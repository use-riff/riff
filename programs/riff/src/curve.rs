//! Constant-product bonding curve over virtual reserves.
//!
//! Price follows `virtual_sol * virtual_token = k`. Every rounding step
//! favours the curve, so `k` never decreases across a trade.
//!
//! When the curve sells out it has raised `R = vs0 * T / (vt0 - T)` SOL and
//! ends at price `vs0 * vt0 / (vt0 - T)^2` (`vs0`/`vt0` = starting virtual
//! reserves, `T` = tokens sold on the curve). Pairing `R` with the `G`
//! reserved tokens in a DEX pool starts it at `R / G`. The two prices match
//! exactly when `G = T * (vt0 - T) / vt0`; with `T + G` fixed to the total
//! supply `S`, that means `vt0 = T^2 / (2T - S)`. See
//! [`graduation_virtual_token_reserves`].

use crate::constants::BPS_DENOMINATOR;

const BPS: u128 = BPS_DENOMINATOR as u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reserves {
    pub virtual_sol: u64,
    pub virtual_token: u64,
    /// Tokens still sellable from the curve.
    pub real_token: u64,
    /// SOL paid into the curve, excluding fees.
    pub real_sol: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyQuote {
    pub tokens_out: u64,
    /// SOL added to the curve's reserves.
    pub sol_to_curve: u64,
    pub fee: u64,
    /// Total the buyer pays: `sol_to_curve + fee`, never more than `max_sol_in`.
    pub total_cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellQuote {
    /// SOL removed from the curve's reserves.
    pub sol_from_curve: u64,
    pub fee: u64,
    /// What the seller receives: `sol_from_curve - fee`.
    pub sol_out: u64,
}

fn ceil_div(a: u128, b: u128) -> Option<u128> {
    a.checked_add(b.checked_sub(1)?)?.checked_div(b)
}

fn fee_on(amount: u128, fee_bps: u16) -> Option<u128> {
    ceil_div(amount.checked_mul(fee_bps as u128)?, BPS)
}

/// Spend up to `max_sol_in` (fee included). If that would buy more than the
/// curve has left, buys exactly what is left and charges only for that.
/// Returns `None` on overflow or if nothing would be bought.
pub fn quote_buy(r: &Reserves, fee_bps: u16, max_sol_in: u64) -> Option<BuyQuote> {
    let (vs, vt) = (r.virtual_sol as u128, r.virtual_token as u128);
    let max_in = max_sol_in as u128;

    let fee = fee_on(max_in, fee_bps)?;
    let net = max_in.checked_sub(fee)?;
    let tokens = vt.checked_mul(net)?.checked_div(vs.checked_add(net)?)?;

    let (tokens, net, total) = if tokens >= r.real_token as u128 {
        // Last buy: take the remainder at the smallest net cost that yields it.
        let tokens = r.real_token as u128;
        let net = ceil_div(vs.checked_mul(tokens)?, vt.checked_sub(tokens)?)?;
        let total = ceil_div(net.checked_mul(BPS)?, BPS.checked_sub(fee_bps as u128)?)?;
        (tokens, net, total)
    } else {
        (tokens, net, max_in)
    };

    if tokens == 0 || total > max_in {
        return None;
    }
    Some(BuyQuote {
        tokens_out: tokens.try_into().ok()?,
        sol_to_curve: net.try_into().ok()?,
        fee: (total - net).try_into().ok()?,
        total_cost: total.try_into().ok()?,
    })
}

/// Sell `tokens_in` back to the curve. Returns `None` on overflow, if the
/// curve lacks the SOL, or if the seller would receive nothing.
pub fn quote_sell(r: &Reserves, fee_bps: u16, tokens_in: u64) -> Option<SellQuote> {
    let (vs, vt) = (r.virtual_sol as u128, r.virtual_token as u128);
    let tokens = tokens_in as u128;

    let gross = vs
        .checked_mul(tokens)?
        .checked_div(vt.checked_add(tokens)?)?;
    if gross > r.real_sol as u128 {
        return None;
    }
    let fee = fee_on(gross, fee_bps)?;
    let out = gross.checked_sub(fee)?;
    if out == 0 {
        return None;
    }
    Some(SellQuote {
        sol_from_curve: gross.try_into().ok()?,
        fee: fee.try_into().ok()?,
        sol_out: out.try_into().ok()?,
    })
}

/// A trading fee divided between its recipients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeSplit {
    pub artist: u64,
    pub creator: u64,
    pub protocol: u64,
}

/// Splits `fee` in proportion to each recipient's rate. Rounding dust goes
/// to the protocol, so the parts always sum to `fee`.
pub fn split_fee(fee: u64, artist_bps: u16, creator_bps: u16, protocol_bps: u16) -> FeeSplit {
    let total = artist_bps as u128 + creator_bps as u128 + protocol_bps as u128;
    if total == 0 {
        return FeeSplit {
            artist: 0,
            creator: 0,
            protocol: fee,
        };
    }
    // part = fee * bps / total <= fee, so the casts back are lossless.
    let part = |bps: u16| (fee as u128 * bps as u128 / total) as u64;
    let (artist, creator) = (part(artist_bps), part(creator_bps));
    FeeSplit {
        artist,
        creator,
        protocol: fee - artist - creator,
    }
}

/// Starting virtual token reserves that make the curve's final price equal
/// the graduation pool's starting price, given the total supply and the
/// tokens sold on the curve (the rest is the graduation reserve).
///
/// Needs `total_supply / 2 < curve_supply < total_supply`: at or below half,
/// the reserve is too large to ever match; at the total, nothing is left to
/// pair. Just above half, the result grows without bound and returns `None`
/// once it no longer fits in a `u64`.
pub fn graduation_virtual_token_reserves(total_supply: u64, curve_supply: u64) -> Option<u64> {
    let (s, t) = (total_supply as u128, curve_supply as u128);
    if t >= s || t.checked_mul(2)? <= s {
        return None;
    }
    t.checked_mul(t)?.checked_div(2 * t - s)?.try_into().ok()
}

impl Reserves {
    pub fn apply_buy(&self, q: &BuyQuote) -> Option<Reserves> {
        Some(Reserves {
            virtual_sol: self.virtual_sol.checked_add(q.sol_to_curve)?,
            virtual_token: self.virtual_token.checked_sub(q.tokens_out)?,
            real_token: self.real_token.checked_sub(q.tokens_out)?,
            real_sol: self.real_sol.checked_add(q.sol_to_curve)?,
        })
    }

    pub fn apply_sell(&self, tokens_in: u64, q: &SellQuote) -> Option<Reserves> {
        Some(Reserves {
            virtual_sol: self.virtual_sol.checked_sub(q.sol_from_curve)?,
            virtual_token: self.virtual_token.checked_add(tokens_in)?,
            real_token: self.real_token.checked_add(tokens_in)?,
            real_sol: self.real_sol.checked_sub(q.sol_from_curve)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL: u64 = 1_000_000_000;
    const TOKEN: u64 = 1_000_000;
    const FEE_BPS: u16 = 100;
    const SUPPLY: u64 = 1_000_000_000 * TOKEN;
    const CURVE_SUPPLY: u64 = 793_100_000 * TOKEN;

    fn fresh() -> Reserves {
        Reserves {
            virtual_sol: 30 * SOL,
            virtual_token: graduation_virtual_token_reserves(SUPPLY, CURVE_SUPPLY).unwrap(),
            real_token: CURVE_SUPPLY,
            real_sol: 0,
        }
    }

    fn k(r: &Reserves) -> u128 {
        r.virtual_sol as u128 * r.virtual_token as u128
    }

    #[test]
    fn buy_charges_fee_on_amount_spent() {
        let q = quote_buy(&fresh(), FEE_BPS, SOL).unwrap();
        assert_eq!(q.fee, 10_000_000);
        assert_eq!(q.sol_to_curve, 990_000_000);
        assert_eq!(q.total_cost, SOL);
        // floor(1_073_025_605_595_359 * 0.99e9 / (30e9 + 0.99e9))
        assert_eq!(q.tokens_out, 34_278_649_549_512);
    }

    #[test]
    fn buy_never_decreases_k() {
        let mut r = fresh();
        for amount in [1_000, 7 * SOL / 3, 12_345_678, 5 * SOL] {
            let q = quote_buy(&r, FEE_BPS, amount).unwrap();
            let next = r.apply_buy(&q).unwrap();
            assert!(k(&next) >= k(&r));
            r = next;
        }
    }

    #[test]
    fn oversized_buy_takes_remainder_and_charges_less() {
        let q = quote_buy(&fresh(), FEE_BPS, 1_000 * SOL).unwrap();
        assert_eq!(q.tokens_out, 793_100_000 * TOKEN);
        assert!(q.total_cost < 1_000 * SOL);
        assert_eq!(q.total_cost, q.sol_to_curve + q.fee);
        // Fee is at least 3% of what was actually paid.
        assert!(q.fee as u128 * 10_000 >= q.total_cost as u128 * FEE_BPS as u128);
        let after = fresh().apply_buy(&q).unwrap();
        assert_eq!(after.real_token, 0);
        assert!(k(&after) >= k(&fresh()));
    }

    #[test]
    fn full_curve_raises_about_85_sol() {
        let q = quote_buy(&fresh(), FEE_BPS, 1_000 * SOL).unwrap();
        assert!(
            (84 * SOL..86 * SOL).contains(&q.sol_to_curve),
            "{}",
            q.sol_to_curve
        );
    }

    #[test]
    fn dust_buy_returns_none() {
        assert_eq!(quote_buy(&fresh(), FEE_BPS, 0), None);
        assert_eq!(quote_buy(&fresh(), FEE_BPS, 1), None);
    }

    #[test]
    fn buy_then_sell_loses_only_fees_and_rounding() {
        let r = fresh();
        let buy = quote_buy(&r, FEE_BPS, 2 * SOL).unwrap();
        let r = r.apply_buy(&buy).unwrap();
        let sell = quote_sell(&r, FEE_BPS, buy.tokens_out).unwrap();
        let r2 = r.apply_sell(buy.tokens_out, &sell).unwrap();

        assert!(sell.sol_from_curve <= buy.sol_to_curve);
        assert!(buy.sol_to_curve - sell.sol_from_curve <= 1);
        assert!(sell.sol_out < 2 * SOL);
        assert!(k(&r2) >= k(&r));
        assert_eq!(r2.real_token, fresh().real_token);
    }

    #[test]
    fn sell_cannot_exceed_real_sol() {
        let mut r = fresh();
        r.real_sol = 10;
        assert_eq!(quote_sell(&r, FEE_BPS, 1_000_000 * TOKEN), None);
    }

    #[test]
    fn split_fee_is_proportional_and_exact() {
        let f = |fee| split_fee(fee, 50, 20, 30);
        assert_eq!(
            f(10_000_000),
            FeeSplit {
                artist: 5_000_000,
                creator: 2_000_000,
                protocol: 3_000_000
            }
        );
        // Dust goes to the protocol; parts always sum to the fee.
        assert_eq!(
            f(7),
            FeeSplit {
                artist: 3,
                creator: 1,
                protocol: 3
            }
        );
        for fee in [0, 1, 2, 3, 99, 12_345_677] {
            let s = f(fee);
            assert_eq!(s.artist + s.creator + s.protocol, fee);
        }
        assert_eq!(split_fee(100, 0, 0, 0).protocol, 100);
    }

    #[test]
    fn derived_reserves_for_default_split() {
        let vt0 = graduation_virtual_token_reserves(SUPPLY, CURVE_SUPPLY).unwrap();
        assert_eq!(vt0, 1_073_025_605_595_359);
    }

    #[test]
    fn derived_reserves_reject_impossible_splits() {
        let g = graduation_virtual_token_reserves;
        assert_eq!(g(SUPPLY, SUPPLY), None);
        assert_eq!(g(SUPPLY, SUPPLY / 2), None);
        assert_eq!(g(SUPPLY, 0), None);
        // Just above half: valid in theory, but the reserves overflow u64.
        assert_eq!(g(SUPPLY, SUPPLY / 2 + 1), None);
        assert!(g(SUPPLY, 600_000_000 * TOKEN).is_some());
        assert!(g(SUPPLY, SUPPLY - 1).is_some());
    }

    /// Relative gap between the curve's final price and the graduation
    /// pool's starting price, after the curve sells out.
    fn graduation_price_gap(vt0: u64, curve_supply: u64) -> f64 {
        let r = Reserves {
            virtual_sol: 30 * SOL,
            virtual_token: vt0,
            real_token: curve_supply,
            real_sol: 0,
        };
        let q = quote_buy(&r, FEE_BPS, u64::MAX / 2).unwrap();
        let end = r.apply_buy(&q).unwrap();
        assert_eq!(end.real_token, 0);
        let reserve = (SUPPLY - curve_supply) as u128;
        // final = vs / vt, pool = real_sol / reserve; compare cross-multiplied.
        let final_x = end.virtual_sol as u128 * reserve;
        let pool_x = end.real_sol as u128 * end.virtual_token as u128;
        (pool_x as f64 - final_x as f64).abs() / final_x as f64
    }

    #[test]
    fn graduation_price_matches_curve_final_price() {
        for curve_supply in [
            CURVE_SUPPLY,
            600_000_000 * TOKEN,
            800_000_000 * TOKEN,
            950_000_000 * TOKEN,
        ] {
            let vt0 = graduation_virtual_token_reserves(SUPPLY, curve_supply).unwrap();
            let gap = graduation_price_gap(vt0, curve_supply);
            assert!(gap < 1e-9, "{curve_supply}: gap {gap:e}");
        }
    }

    #[test]
    fn pump_fun_reserves_would_leave_a_price_gap() {
        // The hand-picked 1,073,000,000 is close but not exact: the pool
        // would open ~0.0068% below the curve's final price.
        let gap = graduation_price_gap(1_073_000_000 * TOKEN, CURVE_SUPPLY);
        assert!((6.7e-5..6.8e-5).contains(&gap), "gap {gap:e}");
    }
}
