_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.4 Accounting core — vouchers and voucher lines

```sql
-- =====================================================================
-- 11.4  Accounting core
-- Legacy: DMoein (header), Moein (lines)
-- =====================================================================

-- ---------------------------------------------------------------------
-- vouchers   <- legacy DMoein   §2.8
-- ---------------------------------------------------------------------
CREATE TABLE vouchers (
    id             bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy DM_SSN
    tenant_id      bigint         NOT NULL REFERENCES tenants(id),  -- [NEW] §A3
    fiscal_year_id bigint         NOT NULL REFERENCES fiscal_years(id),  -- legacy DM_Coid
    voucher_number integer        NOT NULL,        -- legacy DM_Sanad — allocated from the year counter (§5.7)
    voucher_date   date           NOT NULL,        -- legacy DM_Date char(10) Jalali
    description    text,                           -- legacy DM_Desc varchar(500)
    total_debit    bigint         NOT NULL DEFAULT 0,   -- legacy DM_TBed — denormalised SUM(lines.debit_amount)
    total_credit   bigint         NOT NULL DEFAULT 0,   -- legacy DM_TBes
    line_count     integer        NOT NULL DEFAULT 0,   -- legacy DM_Count — a header with 0 is DELETED (Dmu.pas:855)
    status         voucher_status NOT NULL DEFAULT 'draft',   -- legacy DM_Tx 0/1/2
    is_locked      boolean        NOT NULL DEFAULT false,     -- legacy DM_Lock — FAIL-CLOSED (Dmu.pas:993, §9.6)
    cross_reference integer,                       -- legacy DM_Atf عطف — purpose unconfirmed (§12.10 item 4)
    -- MIGRATION ONLY (§6.8 rule 4, §12.1)
    legacy_voucher_date_jalali text,
    created_at     timestamptz    NOT NULL DEFAULT now(),   -- legacy DM_MDate (!) — see COMMENT
    updated_at     timestamptz,                             -- legacy DM_CDate (!)
    created_by     bigint         REFERENCES users(id),     -- legacy DM_MUser (!)
    updated_by     bigint         REFERENCES users(id),     -- legacy DM_CUser (!)
    CONSTRAINT vouchers_number_key
        UNIQUE (tenant_id, fiscal_year_id, voucher_number),                   -- changed from a fiscal-year-scoped UNIQUE — §A3; [NEW] §13.2, §5.6 R8
    CONSTRAINT vouchers_totals_nonneg
        CHECK (total_debit >= 0 AND total_credit >= 0),                       -- [NEW]
    CONSTRAINT vouchers_line_count_nonneg CHECK (line_count >= 0),            -- [NEW]
    CONSTRAINT vouchers_number_positive   CHECK (voucher_number > 0),         -- [NEW]
    -- The ONLY balance check in the legacy system is the draft → confirmed transition
    -- (SanadViewU.pas:298,301). Drafts may legitimately be unbalanced.
    CONSTRAINT vouchers_balanced_when_not_draft
        CHECK (status = 'draft' OR total_debit = total_credit)                -- [NEW] §13.3 ⚠
);

CREATE INDEX vouchers_year_date_idx   ON vouchers (tenant_id, fiscal_year_id, voucher_date);
CREATE INDEX vouchers_year_status_idx ON vouchers (tenant_id, fiscal_year_id, status);
CREATE INDEX vouchers_open_draft_idx  ON vouchers (tenant_id, fiscal_year_id, voucher_date)
    WHERE status = 'draft';   -- serves Get_NewSanad_DateID's "reuse today's draft" lookup (§5.3.2)

ALTER TABLE vouchers ENABLE ROW LEVEL SECURITY;
ALTER TABLE vouchers FORCE ROW LEVEL SECURITY;
CREATE POLICY vouchers_tenant_isolation ON vouchers
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- FKs deferred from §11.2:
ALTER TABLE fiscal_years
    ADD CONSTRAINT fiscal_years_closing_voucher_fkey FOREIGN KEY (closing_voucher_id) REFERENCES vouchers(id),
    ADD CONSTRAINT fiscal_years_opening_voucher_fkey FOREIGN KEY (opening_voucher_id) REFERENCES vouchers(id);

COMMENT ON COLUMN vouchers.created_at IS
  '⚠ MAPPED FROM DM_MDate, NOT DM_CDate. In the legacy the C/M prefixes are SWAPPED relative to the '
  'usual create/modify convention: DM_CUser/DM_CDate are written on UPDATE (Dmu.pas:831) and '
  'DM_MUser/DM_MDate on INSERT (Dmu.pas:834-835). CORROBORATED 2026-08-19 by the legacy DDL and the '
  'DMoein_Make_Update procedure body (schema-only dump, 02-data-model/02-12-b.md §12.10 item 5): '
  'DM_MDate is datetime NOT NULL DEFAULT (getdate()), DM_CDate is nullable with no default, and the '
  'procedure''s INSERT explicitly sets DM_MUser/DM_MDate and leaves DM_CUser/DM_CDate at 0/NULL — '
  'consistent with the swap, though that procedure itself has no confirmed live call site. Historical '
  'confirmation (SELECT COUNT(*) FROM DMoein WHERE DM_MDate > DM_CDate) still needs a populated DB.';
COMMENT ON COLUMN vouchers.total_debit IS
  'Legacy DM_TBed. Denormalised and recomputed by Dmoein_UpdateMab OUTSIDE any transaction (§9.4), '
  'so it drifts. Run the §7.7 check 4 probe before migrating. In the rebuild it is maintained in '
  'the SAME transaction as the lines, or derived.';
COMMENT ON CONSTRAINT vouchers_balanced_when_not_draft ON vouchers IS
  '[NEW] §13.3 ⚠ Confirmed-but-unbalanced vouchers almost certainly exist because the file import '
  'path (§10.6) performs no balance check at all. Probe before adopting.';

-- Legacy → Rust:
--   TDM.DMoein_Make (Dmu.pas:828-838)     → accounting::vouchers::upsert_header()
--   TDM.Dmoein_UpdateMab                  → DELETED; totals maintained in-transaction
--   TDM.New_Sanad (Dmu.pas:1247)          → numbering::next_voucher_number() (§5.7)
--   MoeinToRU.pas:264 (a SECOND allocator, reading DMoein instead of Moein — §5.3.1 defect)
--                                         → DELETED. One allocator only.
--   TDM.Get_NewSanad_DateID (§5.3.2)      → numbering::find_or_create_daily_draft(),
--                                            IDList becomes a typed smallint[] bind parameter
--   Asnad_View / MoeinViewSanad / MoeinTotalSanad / Moein_ChapSanad → vouchers::list/get/print
--     ⚠ Asnad_View and Moein_ChapSanad (as declared in RoozViewU.dfm) take NO fiscal-year
--       parameter — a cross-year data-leak hazard (§12.12). The rebuilt endpoints ALWAYS scope.


-- ---------------------------------------------------------------------
-- voucher_lines   <- legacy Moein   §2.7
-- The general journal line table — the heart of the system.
-- ---------------------------------------------------------------------
CREATE TABLE voucher_lines (
    id             bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy M_SSN
    tenant_id      bigint         NOT NULL REFERENCES tenants(id),  -- [NEW] §A3
    voucher_id     bigint         NOT NULL REFERENCES vouchers(id) ON DELETE CASCADE,
    fiscal_year_id bigint         NOT NULL REFERENCES fiscal_years(id),  -- legacy M_COID (redundant with voucher, kept for index locality)
    line_date      date           NOT NULL,        -- legacy M_Date — should equal the header date
    debit_amount   bigint         NOT NULL DEFAULT 0,      -- legacy M_Bed  bigint, whole rial
    credit_amount  bigint         NOT NULL DEFAULT 0,      -- legacy M_Bes
    quantity       numeric(18,3)  NOT NULL DEFAULT 0,      -- legacy M_Ted — CONFIRMED numeric(18,3) exactly (§12.10 item 2)
    description    text,                           -- legacy Article varchar(250) — ONLY this column exists, no M_Article (§12.10 item 1)
    account_id     bigint         NOT NULL REFERENCES accounts(id),   -- legacy M_Code
    status         voucher_status NOT NULL DEFAULT 'draft',   -- legacy M_Tx — duplicated from the header
    source_module  smallint       NOT NULL DEFAULT 0 REFERENCES journal_sources(id),  -- legacy M_ID
    source_id      bigint,                         -- legacy M_Link — POLYMORPHIC, cannot be a FK (§13.8)
    -- MIGRATION ONLY (§6.8 rule 4, §12.1)
    legacy_line_date_jalali text,
    -- MIGRATION ONLY — the denormalised 4-segment code, kept only so unresolvable
    -- M_Code = 0 rows (SanadMoeinu.pas:328, §10.6 defect) can be reconciled after the fact.
    legacy_gl_code integer, legacy_sub_code integer,
    legacy_a1_code integer, legacy_a2_code integer,
    created_at     timestamptz    NOT NULL DEFAULT now(),   -- legacy M_Time (GetDate(); NOT written by every path)
    updated_at     timestamptz,
    created_by     bigint         REFERENCES users(id),     -- legacy M_User
    updated_by     bigint         REFERENCES users(id),
    CONSTRAINT voucher_lines_amounts_nonneg
        CHECK (debit_amount >= 0 AND credit_amount >= 0),                      -- [NEW] §13.4 ⚠ §7.7
    CONSTRAINT voucher_lines_one_side_only
        CHECK (debit_amount = 0 OR credit_amount = 0),                         -- [NEW] §13.4 ⚠
    CONSTRAINT voucher_lines_not_both_zero
        CHECK (debit_amount <> 0 OR credit_amount <> 0),                       -- [NEW] see COMMENT
    CONSTRAINT voucher_lines_quantity_nonneg CHECK (quantity >= 0)             -- [NEW] §2.7
);

-- Indexes the legacy query patterns demand (§2.7). RESOLVED (§12.6): the legacy Moein table has
-- ZERO indexes of any kind (confirmed by a schema-only dump) — none of the below already exist
-- legacy-side; all are genuinely new.
CREATE INDEX voucher_lines_voucher_idx  ON voucher_lines (tenant_id, voucher_id);
CREATE INDEX voucher_lines_year_date_idx ON voucher_lines (tenant_id, fiscal_year_id, line_date);
CREATE INDEX voucher_lines_account_idx  ON voucher_lines (tenant_id, account_id, line_date);
CREATE INDEX voucher_lines_ledger_idx                                   -- serves Moein_View_Daftar
    ON voucher_lines (tenant_id, fiscal_year_id, account_id, line_date, id);
CREATE INDEX voucher_lines_source_idx   ON voucher_lines (tenant_id, source_module, source_id)
    WHERE source_id IS NOT NULL;                                        -- serves every drill-down
CREATE INDEX voucher_lines_trial_balance_idx                            -- serves Taraz4/Taraz_6
    ON voucher_lines (tenant_id, fiscal_year_id, status, line_date)
    INCLUDE (account_id, debit_amount, credit_amount);

ALTER TABLE voucher_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE voucher_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY voucher_lines_tenant_isolation ON voucher_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE voucher_lines IS
  'Legacy Moein. Despite the name (معین = subsidiary account) this is NOT a subsidiary ledger — it '
  'is the single line table for every voucher in the system (§2.7).';
COMMENT ON COLUMN voucher_lines.description IS
  'Legacy column is named "Article" (آرتیکل) with NO M_ prefix in most .dfm files; "M_Article" in '
  'SanadEditU.dfm is that one form''s naming inconsistency, not a second column. RESOLVED 2026-08-19 '
  '(§12.10 item 1): the full Moein column list from a schema-only dump has only "Article" — no '
  '"M_Article" anywhere. Nothing to reconcile.';
COMMENT ON COLUMN voucher_lines.account_id IS
  'Legacy M_Code. NOT enforced as an FK today: the file importer writes 0 (SanadMoeinu.pas:328, '
  '§10.6 defect) and some paths back-fill it after insert (FactorPesteh_U.pas:229-230). Making it '
  'NOT NULL REFERENCES accounts(id) is [NEW] §13.5 — every M_Code = 0 row must be resolved from '
  'the legacy 4-segment code columns first, or the migration fails.';
COMMENT ON COLUMN voucher_lines.source_id IS
  'Legacy M_Link. A polymorphic pointer into the table implied by source_module — impossible to '
  'constrain. §13.8 proposes replacing it with per-source nullable FKs.';
COMMENT ON COLUMN voucher_lines.created_by IS
  'Legacy M_User. The year-end carry-forward routine HARD-CODES 68 here (EnteghalU.pas:254, §10.5 '
  'defect 3). In the rebuild this is always the authenticated user.';
COMMENT ON CONSTRAINT voucher_lines_not_both_zero ON voucher_lines IS
  '[NEW] Not asserted anywhere in the legacy. A zero-zero line is meaningless but may exist as an '
  'artefact of the import path. If the probe finds any, comment this out rather than deleting data.';

-- DROPPED legacy columns, and why:
--   M_Ko, M_Mo, M_Ta1, M_Ta2 — denormalised copies of the account's four segments. Retained only
--       as legacy_* migration columns above, then dropped once every account_id resolves.
--   M_L, M_R, M_Name, M_CR, M_CodeStr — denormalised Sarfasl display values. Whether they are even
--       physical columns on Moein is unresolved (§12.10 item 3). Derive by join.
--   M_Sanad — replaced by voucher_id. The legacy (M_COID, M_Sanad) → DMoein link was a LOGICAL FK
--       kept in step by application code only (TDM.DMoein_Make, Dmu.pas:828-838).

-- Legacy → Rust:
--   MoeinAdd (ArticleMoeinu.dfm:337)  → accounting::voucher_lines::create()
--     ⚠ HIGHEST extraction priority (§12.3 item 1): the legacy procedure receives the amounts as
--       varchar(20) and the quantity as varchar(15) and parses them SERVER-SIDE. That parse is the
--       money semantics. In the rebuild the amounts cross the boundary as i64 and Decimal (§7.7).
--   Moein_All (BastanHesab.dfm:43)    → accounting::year_end::account_balances()
--     ⚠ Must be diffed against the inline #R query at EnteghalU.dfm:330-349, which computes
--       apparently the same year-end balance a DIFFERENT way (§12.3 item 9).
--   Moein_View_Daftar                 → reporting::subsidiary_ledger()
--   Taraz4Setooni / Taraz_6Sotooni    → reporting::trial_balance::{four,six}_column()
--     ⚠ Both take varchar(8) date parameters that SILENTLY TRUNCATE the 10-character values the
--       callers pass (§3.4 item 1) — do not port the truncation.
--   KolState                          → reporting::gl_account_state()
```

---


---

[← 02-11-c-ddl-parties-and-accounts.md](02-11-c-ddl-parties-and-accounts.md) | [02-11-e-ddl-treasury-1.md →](02-11-e-ddl-treasury-1.md)
