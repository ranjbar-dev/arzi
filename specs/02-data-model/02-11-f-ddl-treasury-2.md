_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### continued (§11.5 "Treasury", cont'd)

```sql
-- ---------------------------------------------------------------------
-- cheque_payment_documents / _lines   <- legacy CheckMaster / CheckDetail
-- ISSUED cheques, as a batch (06-treasury.md §1.4, §1.5).
-- ---------------------------------------------------------------------
CREATE TABLE cheque_payment_documents (
    id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy CM_SSN
    tenant_id       bigint      NOT NULL REFERENCES tenants(id),       -- [NEW] §A3
    fiscal_year_id  bigint      NOT NULL REFERENCES fiscal_years(id),  -- legacy CM_Coid
    batch_number    text,                    -- legacy CM_No — operator-supplied, free text
    voucher_number  integer     NOT NULL,    -- legacy CM_Sanad
    issue_date      date        NOT NULL,    -- legacy CM_Date varchar(10) Jalali
    total_amount    bigint      NOT NULL,    -- legacy CM_Mab — COMPUTED from the lines, not entered
    description     text        NOT NULL,    -- legacy CM_Desc — non-blank enforced
    letter_body     text,                    -- legacy CM_Tittle (sic) — covering-letter body, report RP2
    bank_account_id bigint      NOT NULL REFERENCES accounts(id),   -- legacy CM_Code — credited
    line_count      integer     NOT NULL DEFAULT 0,   -- legacy CM_Count
    legacy_issue_date_jalali text,                    -- MIGRATION ONLY
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz,
    created_by      bigint      REFERENCES users(id),  -- legacy CM_UserID
    updated_by      bigint      REFERENCES users(id),
    CONSTRAINT cheque_payment_documents_amount_nonzero CHECK (total_amount <> 0),   -- [AS-IS] CheckEditU.pas
    CONSTRAINT cheque_payment_documents_desc_nonblank
        CHECK (length(btrim(description)) > 0),                                     -- [AS-IS] CheckEditU.pas:399-404
    CONSTRAINT cheque_payment_documents_line_count_nonneg CHECK (line_count >= 0)   -- [NEW]
);

CREATE INDEX cheque_payment_documents_year_date_idx
    ON cheque_payment_documents (tenant_id, fiscal_year_id, issue_date);

ALTER TABLE cheque_payment_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheque_payment_documents FORCE ROW LEVEL SECURITY;
CREATE POLICY cheque_payment_documents_tenant_isolation ON cheque_payment_documents
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN cheque_payment_documents.bank_account_id IS
  'Legacy CM_Code. Validated only as Tag <> 0, i.e. that the typed code resolved '
  '(CheckEditU.pas:371-376); LEAF-NESS IS NOT CHECKED. The FK is stricter than the legacy.';
COMMENT ON COLUMN cheque_payment_documents.letter_body IS
  'Legacy CM_Tittle — note the misspelling of "Title". A three-line free-text block whose default '
  'is remembered per-user in the ini under T11/T12/T13 (CheckEditU.pas:180-185) — that default '
  'moves to user_preferences (§8.6).';

CREATE TABLE cheque_payment_lines (
    id                        bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy identity, never referenced
    tenant_id                 bigint  NOT NULL REFERENCES tenants(id),           -- [NEW] §A3
    document_id               bigint  NOT NULL
        REFERENCES cheque_payment_documents(id) ON DELETE CASCADE,               -- legacy CD_CMSSN
    fiscal_year_id            bigint  NOT NULL REFERENCES fiscal_years(id),      -- legacy CD_Coid (redundant copy)
    line_number               integer NOT NULL,                                  -- [NEW] see COMMENT
    payee_account_id          bigint  NOT NULL REFERENCES accounts(id),          -- legacy CD_Bed — debited
    amount                    bigint  NOT NULL,                                  -- legacy CD_Mab
    description               text,                                             -- legacy CD_Desc → the voucher Article
    payee_bank_account_number text,       -- legacy CD_BankNo varchar(26) — account number or IBAN
    payee_account_holder_name text,       -- legacy CD_Jari  varchar(200) — نام صاحب حساب
    created_at                timestamptz NOT NULL DEFAULT now(),
    created_by                bigint      REFERENCES users(id),
    CONSTRAINT cheque_payment_lines_amount_nonzero CHECK (amount <> 0),          -- [NEW]
    CONSTRAINT cheque_payment_lines_number_key
        UNIQUE (tenant_id, document_id, line_number),                           -- changed from a document-scoped UNIQUE — §A3; [NEW]
    CONSTRAINT cheque_payment_lines_iban_format
        CHECK (payee_bank_account_number IS NULL
               OR payee_bank_account_number !~ '^IR'
               OR payee_bank_account_number ~ '^IR[0-9]{24}$')                   -- [NEW] §13.14
);

CREATE INDEX cheque_payment_lines_document_idx ON cheque_payment_lines (tenant_id, document_id, line_number);
CREATE INDEX cheque_payment_lines_payee_idx    ON cheque_payment_lines (tenant_id, payee_account_id);

ALTER TABLE cheque_payment_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheque_payment_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY cheque_payment_lines_tenant_isolation ON cheque_payment_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN cheque_payment_lines.line_number IS
  '[NEW] The legacy has no stable line identity at all: lines are DELETED AND RE-INSERTED WHOLESALE '
  'on every save (CheckEditU.pas:447), so ids are not stable across edits. An explicit line_number '
  'gives the rebuild a stable ordering; whether to also keep ids stable across edits is a service-'
  'layer decision.';
COMMENT ON COLUMN cheque_payment_lines.payee_account_holder_name IS
  'Legacy CD_Jari. The name is MISLEADING — جاری means "current [account]", but the column holds '
  'the name ON the payee bank account, which may differ from the account name. Carried to the '
  'covering letter and NEVER used in any posting (06-treasury.md §1.5).';
COMMENT ON CONSTRAINT cheque_payment_lines_iban_format ON cheque_payment_lines IS
  '[NEW] §13.14. Only validates values that LOOK like an IBAN, because the column legitimately '
  'also holds plain account numbers. TUtil.IS_ShabaNo exists (Utility.pas:90) and is NOT called '
  'here today.';


-- ---------------------------------------------------------------------
-- petty_cash_documents / _lines   <- legacy TankhahMaster / TankhahDetail
-- Structurally a clone of the cheque batch with two columns dropped (06-treasury.md §1.6).
-- ---------------------------------------------------------------------
CREATE TABLE petty_cash_documents (
    id                   bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy TM_SSN
    tenant_id            bigint      NOT NULL REFERENCES tenants(id),      -- [NEW] §A3
    fiscal_year_id       bigint      NOT NULL REFERENCES fiscal_years(id),  -- legacy TM_Coid
    claim_number         text,                    -- legacy TM_No — free text
    voucher_number       integer     NOT NULL,    -- legacy TM_Sanad
    claim_date           date        NOT NULL,    -- legacy TM_Date varchar(10) Jalali
    total_amount         bigint      NOT NULL,    -- legacy TM_Mab — computed from the lines
    description          text        NOT NULL,    -- legacy TM_Desc — non-blank enforced
    custodian_account_id bigint      NOT NULL REFERENCES accounts(id),   -- legacy TM_Code تنخواه‌دار, credited
    line_count           integer     NOT NULL DEFAULT 0,   -- legacy TM_Count
    legacy_claim_date_jalali text,                         -- MIGRATION ONLY
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz,
    created_by           bigint      REFERENCES users(id),  -- legacy TM_UserID
    updated_by           bigint      REFERENCES users(id),
    CONSTRAINT petty_cash_documents_amount_nonzero CHECK (total_amount <> 0),      -- [AS-IS]
    CONSTRAINT petty_cash_documents_desc_nonblank  CHECK (length(btrim(description)) > 0)  -- [AS-IS]
);

CREATE INDEX petty_cash_documents_year_date_idx ON petty_cash_documents (tenant_id, fiscal_year_id, claim_date);

ALTER TABLE petty_cash_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE petty_cash_documents FORCE ROW LEVEL SECURITY;
CREATE POLICY petty_cash_documents_tenant_isolation ON petty_cash_documents
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

CREATE TABLE petty_cash_lines (
    id                 bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id          bigint  NOT NULL REFERENCES tenants(id),                                 -- [NEW] §A3
    document_id        bigint  NOT NULL REFERENCES petty_cash_documents(id) ON DELETE CASCADE,  -- legacy TD_TMSSN
    fiscal_year_id     bigint  NOT NULL REFERENCES fiscal_years(id),   -- legacy TD_Coid
    line_number        integer NOT NULL,                               -- [NEW]
    expense_account_id bigint  NOT NULL REFERENCES accounts(id),       -- legacy TD_Bed — debited
    amount             bigint  NOT NULL,                               -- legacy TD_Mab
    description        text,                                          -- legacy TD_Desc → the voucher Article
    created_at         timestamptz NOT NULL DEFAULT now(),
    created_by         bigint      REFERENCES users(id),
    CONSTRAINT petty_cash_lines_amount_nonzero CHECK (amount <> 0),                -- [NEW]
    CONSTRAINT petty_cash_lines_number_key
        UNIQUE (tenant_id, document_id, line_number)    -- changed from a document-scoped UNIQUE — §A3; [NEW]
);

CREATE INDEX petty_cash_lines_document_idx ON petty_cash_lines (tenant_id, document_id, line_number);
CREATE INDEX petty_cash_lines_expense_idx  ON petty_cash_lines (tenant_id, expense_account_id);

ALTER TABLE petty_cash_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE petty_cash_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY petty_cash_lines_tenant_isolation ON petty_cash_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE petty_cash_documents IS
  'A "petty-cash fund" is NOT an entity in this system — no fund table, no float/imprest amount, no '
  'replenishment document. The fund is whatever leaf account the operator selects as TM_Code, and '
  'its balance is whatever the general ledger says (06-treasury.md §1.9). Tenant-scoped per §A3 — '
  'RLS on tenant_id.';
COMMENT ON COLUMN petty_cash_documents.custodian_account_id IS
  'Legacy TM_Code. Note the legacy denormalises the LAST segment name here (Taraf.Get_LastName, '
  'TankhahEdit.pas:211) while the LINES denormalise the FULL path (TankhahEdit.pas:262) — '
  'inconsistently, within the same module (§13.11).';


-- ---------------------------------------------------------------------
-- cheque_types   <- legacy TCheck   §2.2 — RESOLVED 2026-08-19, full DDL from a schema-only dump
-- ---------------------------------------------------------------------
-- TCheck is declared as a bare TADOTable with no persistent fields (Dmu.dfm:891-897) and read by
-- an unfiltered `Select * from TCheck` (Dmu.dfm:903), so no column was ever named in the Delphi
-- source — but `Full_Script_14050527.sql` (schema-only, 0 data rows) has the full legacy DDL:
--   S_SSN int IDENTITY(1,1), S_COID int, S_State int, S_StateName varchar(50),
--   S_CheckNo varchar(15), S_Sanad int, S_Date varchar(10), S_DateS varchar(50), S_Mab bigint,
--   S_Desc varchar(200), S_BankSSN int, S_BankCR varchar(50), S_BankName varchar(100),
--   S_BedSSN varchar(200), S_BedCR varchar(50), S_BedName varchar(100), S_Asnadssn int,
--   S_AsnadCR varchar(50), S_AsnadName varchar(50), S_UserID int.
-- No PRIMARY KEY, no FK, no index — matches every other legacy table in this schema.
-- An MS_Description extended property on S_State gives the full code list directly, no data
-- needed: 1=check naghdi (cash cheque), 2=check moedi, 3=bardasht naghdi (cash withdrawal),
-- 4=bardasht ba kart (card withdrawal), 11=daryaft check (cheque received), 12=variz ba fish ya
-- naghdi (deposit by slip/cash), 13=variz ba kartkhan (deposit by card reader).
-- ⚠ This reads as a GENERAL cash/bank MOVEMENT-TYPE table (cash, card, deposit, withdrawal, AND
-- cheque), not a cheque-specific lookup — "cheque_types" may be too narrow a name/scope for what
-- this actually models. Whether it is still populated/live is unresolved (0 data rows in this
-- dump) — see 06-treasury/06-12-open-questions.md Q21.

CREATE TABLE cheque_types (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    legacy_id   integer UNIQUE,     -- legacy S_State, 1/2/3/4/11/12/13 per the extended property above
    code        text NOT NULL,      -- e.g. 'cash_cheque', 'endorsed_cheque', 'cash_withdrawal', 'card_withdrawal',
                                     --      'cheque_received', 'slip_or_cash_deposit', 'card_reader_deposit'
    label_fa    text NOT NULL
    -- Re-evaluate scope/name against 06-treasury/06-12-open-questions.md Q21 before finalising —
    -- this may belong as a shared `cash_movement_types` lookup rather than cheque-specific.
);
```

> **Not modelled here, deliberately:** `banks`, `bank_branches`, `bank_accounts` and
> `cheque_books`. The legacy system does **not** model any of them — a "bank account" is simply a
> leaf `accounts` row, and there is no cheque-book, series or serial-range entity anywhere
> (`06-treasury.md` §1.7, §1.8). Adding them is **§13.13**. §12.13's dependency is now
> **resolved**: a schema-only dump (2026-08-19) confirms `BN_*` does **not** exist in this
> database — none of the 39 `CREATE TABLE` statements matches those column names. §13.13's
> `banks`/`bank_accounts` design has no legacy starting point to migrate from; it is new, full
> stop.

---


---

[← 02-11-e-ddl-treasury-1.md](02-11-e-ddl-treasury-1.md) | [02-11-g-ddl-inventory.md →](02-11-g-ddl-inventory.md)
