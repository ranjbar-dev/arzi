_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 15. PROPOSED IMPROVEMENTS (needs user approval)

**Everything above this line is a description of existing behaviour and is to be ported as-is unless
the team decides otherwise. Everything below is a suggestion only.**

### 15.1 Structural — high value, low risk

| # | Proposal | Rationale |
|---|---|---|
| P1 | **Add a real `account_type` / nature column** (asset, liability, equity, revenue, expense) with a natural-side flag. | §1.8 — the system has no account classification at all. Closing (§9.2) currently depends on the operator ticking the right Kol accounts by hand every year. A type column makes the P&L close deterministic and enables a real balance sheet / income statement. |
| P2 | **Add `line_number` to `voucher_lines`.** | §5.3 — line order currently survives only because identity values happen to be assigned in insertion order. |
| P3 | **Drop the denormalised columns** `S_Bed`, `S_Bes`, `S_Remi`, `S_Count`, `M_R`, `M_L`, `FullName`, `LineName`, `NeedUpdate`, and derive them. Keep `child_count` (or replace with an `is_leaf` computed column / a recursive CTE). | §1.1 — four of them are already dead; the rest need the `Active_Set` procedure to stay correct. |
| P4 | **Drop `Moein.M_Ko/M_Mo/M_Ta1/M_Ta2`; keep `account_id` only.** Expose the tuple through a view. | §3.3 — the duplication forces a back-fill `UPDATE … FROM` after every generator and creates a corruption path. |
| P5 | **Drop `Moein.M_Date`, `M_Tx`; derive from the header.** | §3.3 — they are copies of `DM_Date` / `DM_TX` on every line and can drift. |
| P6 | **Unique constraint** on `(company_id, level1, level2, level3, level4)` for accounts and on `(fiscal_year_id, voucher_number)` for vouchers. | §1.5, §3.7 — currently client-side only. |
| P7 | **Replace `(M_Id, M_Link)` with a typed source reference** — a `source_kind` enum plus per-type FK, or a `document_links` table. | §6.8 — the current pair has no referential integrity and its ranges are undocumented. |
| P8 | **Store dates as `DATE`** plus a derived Jalali string, instead of `varchar(10)`. | §3.9 — enables real date arithmetic and correct Esfand/leap-year validation. |

### 15.2 Correctness fixes (the defects catalogued in §14)

| # | Proposal | Refs |
|---|---|---|
| P9 | Make un-posting delete the same `source_kind` range that posting creates. | §6.7 defect 14 |
| P10 | `Is_Admin_Or_Valid_Sanad` must **allow** access when no header row exists, not deny. | §3.6 defect 15 |
| P11 | Merge must update every table that "change voucher number" updates. Derive both from one list. | §7.3 defect 16 |
| P12 | Recompute header totals after every line mutation, in the same transaction. | §4.6 defect 17 |
| P13 | Journal generation must exclude `voucher_kind = journal` from its source range. | §8.1 defect 18 |
| P14 | **Mark source vouchers as journalised** (a `journalised_in_voucher_id` column) and reject overlapping ranges. | §8.1 defect 19 |
| P15 | Compare account codes as integers, not by string containment, in the closing-destination guard. | §9.2 defect 20 |
| P16 | Validate the voucher date against the fiscal year on save. The function already exists. | §4.1, defect 26 |
| P17 | Enforce account locks on the voucher-entry path. | §2.5, defect 27 |
| P18 | Enforce permissions 1102/1103/1104 on the live chart-of-accounts screen. | §13.2, defect 28 |
| P19 | Wrap every multi-statement operation in one transaction (§6.6 has none; §9.3 uses one per account rather than one per run). | §6.6, §9.3 |
| P20 | Use bound parameters everywhere; eliminate the string-concatenation SQL. | §11.2 |
| P21 | Hash passwords. | §13.1 |

### 15.3 Business-logic improvements

| # | Proposal | Rationale |
|---|---|---|
| P22 | **Enforce the year-end sequence**: refuse the carry-forward until the P&L accounts have been closed (or filter them out automatically using P1's account type). | §9.3 — currently an unenforced ordering dependency that silently produces wrong opening balances. |
| P23 | **Restore the two rules lost from `FinalU`**: the closing destination must be a leaf, and an account cannot be closed to itself. | §9.4 |
| P24 | **Archive the closed year automatically** — set `IsActive = 0` on year N when the carry-forward completes, and provide a UI to re-open it. | §14 Q5 — nothing currently sets this flag. |
| P25 | **Auto-fill the balancing amount** on the last voucher line (prefill debit or credit with the outstanding difference). | §5.7 — every comparable Iranian package does this; its absence is the single biggest data-entry cost on this screen. |
| P26 | **Show the running balance and the out-of-balance difference live** in the grid footer, not only on save. | §4.1 |
| P27 | **Require merge to operate on draft vouchers only.** | §7.1 — merging two permanently-posted vouchers rewrites finalised accounting records. |
| P28 | **Generate vouchers for `FM_ID` 14/15/16/25/26** (production and transfers) or document explicitly that they are non-financial. | §6.3 |
| P29 | **Re-enable the tax lines in `init11`/`init12`** or delete the dead branches. Currently `if false then`. | §6.5 |
| P30 | Make the special-account registry single-sourced: keep `base_config`, migrate `C1081`/`C1082` into it. | §1.9, §1.10 |
| P31 | Move the hard-coded Kol numbers (103/104/109/303) and the hard-coded user id 68 into configuration. | §13.4 |

### 15.4 UX / rebuild-specific

| # | Proposal |
|---|---|
| P32 | Replace the four-box cascading code entry with a single typeahead over `code — full name`, keeping the four-box form as an option. The `Ko-Mo-Ta1-Ta2` parser already exists (`Dm.Split_Code`) and can back the typeahead. |
| P33 | Make the voucher grid inline-editable, keeping the modal line dialog for the account picker only. |
| P34 | Surface the audit fields (`created_by`, `created_at`, `updated_by`, `updated_at`) that the legacy form declares but hides. §12.4 |
| P35 | Normalise the six inconsistent state-label sets (§3.6) into one translation key per state. |
| P36 | Replace `.GGS` INI import/export with CSV or JSON, keeping a `.GGS` reader for migration. §5.9 |
| P37 | Replace `Dm.inttostr3`'s 3-4-4 digit grouping with a locale formatter — **after confirming with the users**, since the grouping appears deliberate. §10 |
| P38 | Replace per-button `IsEnabel` round-trips with a single permission set loaded at login. §13.1 |
| P39 | Add an accounting-period lock finer than the fiscal year (e.g. monthly close) — commonly requested and cheap once P1 and P24 exist. |

---

_Prev: [03-14-open-questions](03-14-open-questions.md) | Next: [03-16-naming-map](03-16-naming-map.md)_
