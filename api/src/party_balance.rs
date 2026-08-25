//! Step 3.2 (docs/phase-3-parties.md §3.2): `Jari_Rem`'s exact algorithm
//! (specs/07-parties-and-shareholders/07-06-a.md §6.2), ported as a Rust
//! function rather than a stored procedure, plus the API endpoint that
//! serves it.
//!
//! **Deviation from the legacy, decided by the Build bullet, not a judgment
//! call made here:** the legacy's `Jari_Rem` SQL has no `M_Tx` (voucher
//! status) filter at all — it sums draft, confirmed and posted lines alike.
//! docs/phase-3-parties.md §3.2 explicitly specifies "posted voucher lines"
//! for the rebuild, so `compute_balance` below filters `status = 'posted'`.
//!
//! **Sign convention preserved exactly** (07-06-a.md §6.2 step 4): each
//! coordinate's remainder is `credit − debit`; the total is positive when
//! the entity owes the party (party is a creditor), negative when the party
//! owes the entity — do not flip this while renaming `Bes`/`Bed` to
//! `credit`/`debit`.

use crate::parties::{coordinate, fetch_all_config, find_account_id_by_codes, ConfigRow};
use serde::Serialize;
use sqlx::{Postgres, Transaction};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlAccountBalance {
    pub config_id: i64,
    pub name: String,
    pub account_id: i64,
    pub debit: i64,
    pub credit: i64,
    pub remainder: i64, // credit - debit; positive = entity owes the party
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyBalance {
    pub total: i64, // sum of every breakdown row's remainder; positive = party is a creditor
    pub breakdown: Vec<ControlAccountBalance>,
}

/// `Jari_Rem`, step by step (07-06-a.md §6.2):
/// 1. Every `party_account_config` row flagged `counts_toward_balance`.
/// 2. Slot the card number into Tafsil-1 or Tafsil-2 per `fixed_tafsil1_code` (`coordinate()`,
///    shared with parties.rs — this is the exact same coordinate a leaf account was created at).
/// 3. Sum posted debit/credit for the requested fiscal year at that coordinate, if a leaf exists
///    there at all (an un-provisioned or never-ticked control account contributes nothing, same as
///    the legacy's `isnull(..., 0)`).
/// 4. `remainder = credit - debit`.
/// 5. Drop untouched (both zero) rows from the breakdown — cosmetic, doesn't change the total.
/// 6. `total = Σ remainder`.
pub async fn compute_balance(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: i64,
    card_number: i32,
    fiscal_year_id: i64,
) -> Result<PartyBalance, sqlx::Error> {
    let configs: Vec<ConfigRow> = fetch_all_config(tx, tenant_id)
        .await?
        .into_iter()
        .filter(|c| c.counts_toward_balance)
        .collect();

    let mut breakdown = Vec::new();
    let mut total: i64 = 0;

    for config in &configs {
        let codes = coordinate(config, card_number);
        let Some(account_id) = find_account_id_by_codes(tx, tenant_id, codes).await? else {
            continue; // no leaf at this coordinate -> 0/0, dropped same as step 5
        };

        let (debit, credit): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(debit_amount), 0)::bigint, COALESCE(SUM(credit_amount), 0)::bigint \
             FROM voucher_lines \
             WHERE tenant_id = $1 AND account_id = $2 AND fiscal_year_id = $3 AND status = 'posted'",
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(fiscal_year_id)
        .fetch_one(&mut **tx)
        .await?;

        if debit == 0 && credit == 0 {
            continue; // step 5
        }
        let remainder = credit - debit;
        total += remainder;
        breakdown.push(ControlAccountBalance {
            config_id: config.id,
            name: config.name.clone(),
            account_id,
            debit,
            credit,
            remainder,
        });
    }

    Ok(PartyBalance { total, breakdown })
}
