//! The one place the rebuild's rounding-mode decision lives (docs/phase-5-inventory.md §5.6's own
//! "decide and document which rounding rule the rebuild uses"). Every money/quantity figure that
//! needs rounding to a whole rial — line gross amounts (5.2), percentage discounts (5.5), the
//! average-cost suggestion (5.4), the pistachio deduction calculator's line amount (5.6) — calls
//! this one function, so the choice is made exactly once, not re-decided ad hoc at each call site.
//!
//! **The decision**: round-half-away-from-zero (`RoundingMode::HalfUp` in the `bigdecimal` crate's
//! own naming — its worked examples show `2.5 -> 3`, `-2.5 -> -3`, i.e. ties move away from zero),
//! per specs/05-inventory/05-08-a.md §8.2.2's explicit recommendation, over Delphi's banker's
//! rounding (half-to-even) that every legacy money calculation used.
//!
//! **Correction, found while implementing 5.6**: `BigDecimal::round()`'s *default* rounding mode
//! is `HalfEven` — Delphi's own banker's rounding, not the away-from-zero convention this
//! project's own 5.2/5.4/5.5 doc comments already claimed to be using. On every exact-`.5` tie
//! whose neighbours are already even (the only cases those steps' own tests happened to exercise,
//! e.g. `333.5 -> 334`), `HalfEven` and `HalfUp` agree, so no previously-asserted test value was
//! ever wrong — but the *documented rationale* was inaccurate until this fix, since a bare
//! `.round(0)` call was silently reproducing the legacy's exact banker's-rounding behaviour rather
//! than the away-from-zero fix those steps described. 5.2/5.4/5.5 now call this function too.
//! See 05-08-a.md §8.2.3 Example C for the one worked case where the two modes actually diverge
//! (`5632.5` → `5632` half-to-even vs. `5633` half-away-from-zero) — reproduced as a regression
//! test below.

use bigdecimal::{BigDecimal, RoundingMode, ToPrimitive};

/// Rounds `value` to the nearest whole rial, ties away from zero, per this module's own decision.
pub fn round_to_rial(value: &BigDecimal) -> i64 {
    value.with_scale_round(0, RoundingMode::HalfUp).to_i64().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ties_round_away_from_zero_not_half_to_even() {
        assert_eq!(round_to_rial(&"2.5".parse().unwrap()), 3); // HalfEven would give 2
        assert_eq!(round_to_rial(&"-2.5".parse().unwrap()), -3); // HalfEven would give -2
        assert_eq!(round_to_rial(&"0.5".parse().unwrap()), 1); // HalfEven would give 0
    }

    /// 05-08-a.md §8.2.3 Example C, verbatim: `NabV = 1877.5`, `Phi = 3` -> product `5632.5`.
    /// Delphi's `Round` (half-to-even) gives 5632; this rebuild's decision gives 5633.
    #[test]
    fn pistachio_example_c_diverges_from_delphi_as_documented() {
        let net_weight: BigDecimal = "1877.5".parse().unwrap();
        let product = net_weight * BigDecimal::from(3);
        assert_eq!(round_to_rial(&product), 5633); // NOT Delphi's 5632
    }

    #[test]
    fn non_tie_values_round_normally() {
        assert_eq!(round_to_rial(&"999.999".parse().unwrap()), 1000);
        assert_eq!(round_to_rial(&"333.4".parse().unwrap()), 333);
    }
}
