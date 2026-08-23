-- Step 2.3 (docs/phase-2-accounting-core.md §2.3 / specs/02-data-model/02-11-d-ddl-accounting-
-- core.md §11.4): vouchers (legacy DMoein) and voucher_lines (legacy Moein — the general journal,
-- despite the misleading name, specs/03-accounting-core/03-03-a.md §3.1). Header totals are
-- derived, never stored (C3) — kept correct by maintaining them in the SAME transaction as every
-- line mutation, not the legacy's out-of-transaction `Dmoein_UpdateMab` recompute.

CREATE TYPE voucher_status AS ENUM ('draft', 'confirmed', 'posted');  -- legacy M_Tx/DM_Tx 0/1/2
CREATE TYPE journal_kind   AS ENUM ('ledger', 'daybook');             -- legacy M_Kind/DM_Kind 1/2

-- Lookup, not an enum (specs/02-data-model/02-11-a.md §11.1) — the legacy M_Id value set is
-- confirmed incomplete (several observed codes are unidentified), so unknown values must still be
-- able to migrate without loss. Global, not tenant-scoped, same "shared catalogue" exception as
-- `permissions` (01-platform's convention note).
CREATE TABLE journal_sources (
    id           smallint PRIMARY KEY,  -- legacy Moein.M_ID
    code         text     NOT NULL UNIQUE,
    label_fa     text     NOT NULL,
    label_en     text     NOT NULL,
    source_table text
);

-- id 0 ("manual") is this migration's addition — the spec's own INSERT list (02-11-a.md #11.1)
-- starts at 1 and never seeds the "0 = manual" row that 03-03-a.md §3.4 requires and that
-- `voucher_lines.source_module`'s own `DEFAULT 0` depends on.
INSERT INTO journal_sources (id, code, label_fa, label_en, source_table) VALUES
  ( 0, 'manual',                   'دستی',                     'Manual',                     NULL),
  ( 1, 'inventory_invoice',        'فاکتور انبار',              'Inventory invoice',          'inventory_invoices'),
  (21, 'cheque_received',          'چک دریافتی',                'Cheque received',            'cheques'),
  (22, 'cheque_bounced',           'برگشت چک از بانک',          'Cheque bounced',             'cheques'),
  (23, 'cheque_collected',         'وصول چک',                   'Cheque collected',           'cheques'),
  (24, 'cheque_returned',          'استرداد چک',                'Cheque returned to issuer',  'cheques'),
  (25, 'deposit_slip',             'فیش بانکی',                 'Bank deposit slip',          'deposit_slips'),
  (26, 'cheque_payment_document',  'سند پرداخت چک',             'Cheque payment document',    'cheque_payment_documents'),
  (34, 'pistachio_receipt',        'خرید پسته',                 'Pistachio purchase receipt', NULL),
  (41, 'petty_cash',               'تنخواه',                    'Petty cash',                 'petty_cash_documents');

-- ---------------------------------------------------------------------
-- vouchers   <- legacy DMoein
-- ---------------------------------------------------------------------
CREATE TABLE vouchers (
    id              bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy DM_SSN
    tenant_id       bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id  bigint  NOT NULL REFERENCES fiscal_years(id),  -- legacy DM_Coid
    voucher_number  integer NOT NULL,  -- legacy DM_Sanad — allocated from fiscal_years.next_voucher_number
    voucher_date    date    NOT NULL,  -- legacy DM_Date char(10) Jalali -- stored as a real date (03-03-c.md #3.9's rebuild note)
    description     text,
    -- Denormalised, application-maintained in the SAME transaction as every line change (never a
    -- stale out-of-transaction recompute like the legacy's Dmoein_UpdateMab).
    total_debit     bigint         NOT NULL DEFAULT 0,
    total_credit    bigint         NOT NULL DEFAULT 0,
    line_count      integer        NOT NULL DEFAULT 0,
    status          voucher_status NOT NULL DEFAULT 'draft',
    kind            journal_kind   NOT NULL DEFAULT 'ledger',
    is_locked       boolean        NOT NULL DEFAULT false,
    cross_reference integer,  -- legacy DM_Atf عطف
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz,
    created_by      bigint REFERENCES users(id),
    updated_by      bigint REFERENCES users(id),
    CONSTRAINT vouchers_number_key
        UNIQUE (tenant_id, fiscal_year_id, voucher_number),
    CONSTRAINT vouchers_totals_nonneg    CHECK (total_debit >= 0 AND total_credit >= 0),
    CONSTRAINT vouchers_line_count_nonneg CHECK (line_count >= 0),
    CONSTRAINT vouchers_number_positive  CHECK (voucher_number > 0),
    -- The ONLY balance check the legacy has at all: the draft -> confirmed transition
    -- (SanadViewU.pas:298,301). A draft may legitimately be unbalanced while being edited.
    CONSTRAINT vouchers_balanced_when_not_draft
        CHECK (status = 'draft' OR total_debit = total_credit)
);

CREATE INDEX vouchers_year_date_idx   ON vouchers (tenant_id, fiscal_year_id, voucher_date);
CREATE INDEX vouchers_year_status_idx ON vouchers (tenant_id, fiscal_year_id, status, kind);
CREATE INDEX vouchers_open_draft_idx  ON vouchers (tenant_id, fiscal_year_id, voucher_date)
    WHERE status = 'draft';

ALTER TABLE vouchers ENABLE ROW LEVEL SECURITY;
ALTER TABLE vouchers FORCE ROW LEVEL SECURITY;
CREATE POLICY vouchers_tenant_isolation ON vouchers
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Pending since 1.1's migration (fiscal_years predates vouchers).
ALTER TABLE fiscal_years
    ADD CONSTRAINT fiscal_years_closing_voucher_fkey FOREIGN KEY (closing_voucher_id) REFERENCES vouchers(id),
    ADD CONSTRAINT fiscal_years_opening_voucher_fkey FOREIGN KEY (opening_voucher_id) REFERENCES vouchers(id);

-- ---------------------------------------------------------------------
-- voucher_lines   <- legacy Moein — the general journal (despite the name, NOT a subsidiary ledger).
-- ---------------------------------------------------------------------
CREATE TABLE voucher_lines (
    id             bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy M_SSN
    tenant_id      bigint  NOT NULL REFERENCES tenants(id),
    voucher_id     bigint  NOT NULL REFERENCES vouchers(id) ON DELETE CASCADE,
    fiscal_year_id bigint  NOT NULL REFERENCES fiscal_years(id),  -- redundant with voucher, kept for index locality
    line_date      date    NOT NULL,  -- should equal the header date
    debit_amount   bigint  NOT NULL DEFAULT 0,
    credit_amount  bigint  NOT NULL DEFAULT 0,
    quantity       numeric(18,3) NOT NULL DEFAULT 0,  -- legacy M_Ted — statistical only, not part of balancing
    description    text,  -- legacy "Article" (no M_ prefix)
    account_id     bigint  NOT NULL REFERENCES accounts(id),  -- legacy M_Code -- the ONLY account reference (03-03-a.md #3.3's redundancy resolved: no denormalised tuple columns)
    status         voucher_status NOT NULL DEFAULT 'draft',  -- mirrors the header, kept in sync in the same transaction
    kind           journal_kind   NOT NULL DEFAULT 'ledger',
    source_module  smallint NOT NULL DEFAULT 0 REFERENCES journal_sources(id),  -- legacy M_ID; 0 = manual = editable from the voucher editor
    source_id      bigint,  -- legacy M_Link -- polymorphic, no FK (02-13-a.md #13.8)
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz,
    created_by     bigint REFERENCES users(id),
    updated_by     bigint REFERENCES users(id),
    CONSTRAINT voucher_lines_amounts_nonneg  CHECK (debit_amount >= 0 AND credit_amount >= 0),
    CONSTRAINT voucher_lines_one_side_only   CHECK (debit_amount = 0 OR credit_amount = 0),
    CONSTRAINT voucher_lines_not_both_zero   CHECK (debit_amount <> 0 OR credit_amount <> 0),
    CONSTRAINT voucher_lines_quantity_nonneg CHECK (quantity >= 0)
);

CREATE INDEX voucher_lines_voucher_idx   ON voucher_lines (tenant_id, voucher_id);
CREATE INDEX voucher_lines_year_date_idx ON voucher_lines (tenant_id, fiscal_year_id, line_date);
CREATE INDEX voucher_lines_account_idx   ON voucher_lines (tenant_id, account_id, line_date);
CREATE INDEX voucher_lines_ledger_idx
    ON voucher_lines (tenant_id, fiscal_year_id, account_id, line_date, id);
CREATE INDEX voucher_lines_source_idx    ON voucher_lines (tenant_id, source_module, source_id)
    WHERE source_id IS NOT NULL;
CREATE INDEX voucher_lines_trial_balance_idx
    ON voucher_lines (tenant_id, fiscal_year_id, kind, status, line_date)
    INCLUDE (account_id, debit_amount, credit_amount);

ALTER TABLE voucher_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE voucher_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY voucher_lines_tenant_isolation ON voucher_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE voucher_lines IS
  'Legacy Moein. Despite the name (Persian "subsidiary account") this is NOT a subsidiary ledger -- '
  'it is the single line table for every voucher in the system.';
COMMENT ON COLUMN voucher_lines.source_module IS
  'Legacy M_ID. 0 = manual = the line is editable from the voucher editor; any other value means '
  'the line was generated by another domain and is immutable from here (03-03-a.md #3.4) -- '
  'enforced in api/src/vouchers.rs, not just the UI.';
