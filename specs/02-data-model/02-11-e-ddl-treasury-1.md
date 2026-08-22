_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.5 Treasury

```sql
-- =====================================================================
-- 11.5  Treasury
-- Legacy: DCheck, DCheck2, DFish, CheckMaster, CheckDetail,
--         TankhahMaster, TankhahDetail, TCheck
-- Cross-reference: docs/06-treasury.md
-- =====================================================================

-- ---------------------------------------------------------------------
-- cheques   <- legacy DCheck   (06-treasury.md §1.1)
-- RECEIVED cheques only. There is no discriminator column — issued cheques live in a
-- different table entirely (06-treasury.md §3.1).
-- ---------------------------------------------------------------------
CREATE TABLE cheques (
    id                          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy S_SSN
    tenant_id                   bigint        NOT NULL REFERENCES tenants(id),     -- [NEW] §A3
    fiscal_year_id              bigint        NOT NULL REFERENCES fiscal_years(id),  -- legacy S_COID
    status                      cheque_status NOT NULL DEFAULT 'in_hand',            -- legacy S_State
    cheque_number               text          NOT NULL,   -- legacy S_CheckNo varchar(15) — free text, never validated
    voucher_number              integer       NOT NULL,   -- legacy S_Sanad — the RECEIPT posting's voucher
    received_date               date          NOT NULL,   -- legacy S_Date  varchar(10) Jalali
    due_date                    date          NOT NULL,   -- legacy S_DateS varchar(50) (!) سررسید — §12.2
    amount                      bigint        NOT NULL,   -- legacy S_Mab
    description                 text          NOT NULL,   -- legacy S_Desc varchar(200) — non-blank enforced
    payer_account_id            bigint        NOT NULL REFERENCES accounts(id),   -- legacy S_BesSSN
    notes_receivable_account_id bigint        NOT NULL REFERENCES accounts(id),   -- legacy S_BedSSN
    source_module               smallint      NOT NULL DEFAULT 0,   -- legacy S_linkPrg: 0 manual, 1 goods invoice
    source_id                   bigint,                             -- legacy S_LinkSSN
    -- MIGRATION ONLY (§6.8 rule 4, §12.1, §12.2)
    legacy_received_date_jalali text,
    legacy_due_date_raw         text,     -- the full 50-char S_DateS value, verbatim
    created_at                  timestamptz  NOT NULL DEFAULT now(),
    updated_at                  timestamptz,
    created_by                  bigint       REFERENCES users(id),   -- legacy S_UserID — really "last editor"
    updated_by                  bigint       REFERENCES users(id),
    CONSTRAINT cheques_amount_positive  CHECK (amount > 0),                        -- [AS-IS] CheckDaryaftU.pas:235-240
    CONSTRAINT cheques_desc_nonblank    CHECK (length(btrim(description)) > 0),    -- [AS-IS] CheckDaryaftU.pas:258-264
    CONSTRAINT cheques_number_nonblank  CHECK (length(btrim(cheque_number)) > 0),  -- [NEW] never validated today
    CONSTRAINT cheques_source_pair      CHECK ((source_module = 0) = (source_id IS NULL))  -- [NEW]
);

CREATE INDEX cheques_year_status_idx ON cheques (tenant_id, fiscal_year_id, status);
CREATE INDEX cheques_due_date_idx    ON cheques (tenant_id, due_date);   -- the ageing filter and the sole list sort key
CREATE INDEX cheques_payer_idx       ON cheques (tenant_id, payer_account_id);
CREATE INDEX cheques_number_idx      ON cheques (tenant_id, cheque_number);
CREATE INDEX cheques_source_idx      ON cheques (tenant_id, source_module, source_id) WHERE source_id IS NOT NULL;

ALTER TABLE cheques ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheques FORCE ROW LEVEL SECURITY;
CREATE POLICY cheques_tenant_isolation ON cheques
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- [NEW] §13.14 — cheque-number uniqueness. Commented out DELIBERATELY: a bare UNIQUE on
-- cheque_number is WRONG (two banks legitimately issue the same number), and the correct scope
-- (drawer bank account) does not exist until §13.13 adds a banks model.
-- CREATE UNIQUE INDEX cheques_number_key ON cheques (tenant_id, drawer_bank_account_id, cheque_number);

COMMENT ON COLUMN cheques.status IS
  'Legacy S_State. ⚠ Legacy code 1 means BOTH "received, never deposited" AND "deposited, then '
  'bounced" — they are distinguishable only by the free-text S_StateName. Code 3 is never written '
  'by any path; the bounce screen sets the cheque back to 1 (CheckBargashtu.pas:209). Migration '
  'must classify every S_State = 1 row by inspecting its DCheck2 history (§13.12).';
COMMENT ON COLUMN cheques.due_date IS
  'Legacy S_DateS, declared TStringField Size = 50 (Dmu.dfm:947-950) — the widest field mismatch in '
  'the schema (§6.6). A 50-character column is not a date. §12.2 is BLOCKING for this column. '
  'Note also that the cheque list orders by S_Dates — a THIRD spelling (CheckListDU.pas:329).';
COMMENT ON COLUMN cheques.notes_receivable_account_id IS
  'Legacy S_BedSSN. Debited at receipt; becomes the CREDIT side both when the cheque is deposited '
  'and when it is returned (06-treasury.md §1.1).';
COMMENT ON TABLE cheques IS
  'MISSING COLUMNS the rebuild will need (§13.13): issuing bank, branch, drawer account number, '
  'drawer name — today the operator can only smuggle these into description. Also missing: real '
  'deposited_at / cleared_at / bounced_at / returned_at; those dates exist only on cheque_events. '
  'Tenant-scoped per §A3 — RLS on tenant_id.';

-- DROPPED: S_StateName (denormalised Persian label — §13.11);
--          S_BesCR, S_BesName, S_BedCR, S_BedName (denormalised account code/name; note the
--            asymmetry S_BedName varchar(50) vs S_BesName varchar(100) — §13.11);
--          S_Zssn, S_ZCR, S_ZName (endorsee columns — DEAD, never read or written,
--            06-treasury.md §2.3 T8, §4).

-- Legacy → Rust:
--   MakeSanad_CheckDaryafti (CheckDaryaftU.pas:356) → treasury::posting::post_cheque_receipt()
--     ⚠ HIGHEST extraction priority (§12.3 item 2) — the entire debit/credit generation.
--   The narration builder at CheckDaryaftU.pas:324 calls dbo.Noto3(S_Mab) server-side; in the
--   rebuild the narration is composed in Rust and amounts are formatted in the frontend (§7.7).


-- ---------------------------------------------------------------------
-- cheque_events   <- legacy DCheck2   (06-treasury.md §1.2)
-- Append-only event log. NO row is written for the initial receipt.
-- ---------------------------------------------------------------------
CREATE TABLE cheque_events (
    id                bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy S_SSN (also the ordering key)
    tenant_id         bigint        NOT NULL REFERENCES tenants(id),     -- [NEW] §A3
    cheque_id         bigint        NOT NULL REFERENCES cheques(id),     -- legacy S_Link  [NEW] FK §13.5
    fiscal_year_id    bigint        NOT NULL REFERENCES fiscal_years(id),-- legacy S_COID — MAY DIFFER from the cheque's
    voucher_number    integer       NOT NULL,      -- legacy S_Sanad
    event_date        date          NOT NULL,      -- legacy S_Date varchar(10) Jalali
    amount            bigint        NOT NULL,      -- legacy S_Mab — always a copy of the cheque's
    status_after      cheque_status NOT NULL,      -- legacy S_State — ⚠ WRONG for bounces, see COMMENT
    debit_account_id  bigint        REFERENCES accounts(id),   -- legacy S_BedSSN (declared TStringField?! §12.5)
    credit_account_id bigint        REFERENCES accounts(id),   -- legacy S_BesSSN — NULL for collections
    description       text,                        -- legacy S_Desc
    legacy_event_date_jalali text,                 -- MIGRATION ONLY
    created_at        timestamptz   NOT NULL DEFAULT now(),    -- [NEW] legacy has NO timestamp column at all
    created_by        bigint        REFERENCES users(id),      -- legacy S_UserID
    CONSTRAINT cheque_events_amount_positive CHECK (amount > 0)   -- [NEW]
);

CREATE INDEX cheque_events_cheque_idx ON cheque_events (tenant_id, cheque_id, id);
CREATE INDEX cheque_events_year_idx   ON cheque_events (tenant_id, fiscal_year_id, event_date);

ALTER TABLE cheque_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE cheque_events FORCE ROW LEVEL SECURITY;
CREATE POLICY cheque_events_tenant_isolation ON cheque_events
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN cheque_events.status_after IS
  '⚠ Legacy S_State on this table DISAGREES WITH THE MASTER ROW ON EVERY BOUNCE: '
  'CheckBargashtu.pas:214 inserts S_State = 2 while CheckBargashtu.pas:209 sets the cheque to 1 '
  '(06-treasury.md §2.1). The migration must decide which is authoritative — recommend the '
  'EVENT SEQUENCE, not either stored value.';
COMMENT ON COLUMN cheque_events.credit_account_id IS
  'Legacy S_BesSSN is OMITTED by the collection screen (CheckVosoolU.pas:225), so it is NULL/0 on '
  'every collection event. Hence nullable here rather than NOT NULL.';
COMMENT ON TABLE cheque_events IS
  'No code ever deletes or updates a DCheck2 row — including the unused Delete_Check, which '
  'ORPHANS history (06-treasury.md §1.2). The FK above closes that. Append-only by policy. '
  'Tenant-scoped per §A3 — RLS on tenant_id.';


-- ---------------------------------------------------------------------
-- deposit_slips   <- legacy DFish   (06-treasury.md §1.3)
-- One slip = one amount = one counterparty = one bank account. NO line items.
-- ---------------------------------------------------------------------
CREATE TABLE deposit_slips (
    id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy S_SSN
    tenant_id        bigint      NOT NULL REFERENCES tenants(id),      -- [NEW] §A3
    fiscal_year_id   bigint      NOT NULL REFERENCES fiscal_years(id),  -- legacy S_COID
    deposit_method   smallint    NOT NULL,   -- legacy S_State 1-4 — a CHANNEL, not a lifecycle (§12.9)
    slip_number      text,                   -- legacy S_FishNo varchar(15) — free text, never validated
    voucher_number   integer     NOT NULL,   -- legacy S_Sanad
    deposit_date     date        NOT NULL,   -- legacy S_Date varchar(10) Jalali
    amount           bigint      NOT NULL,   -- legacy S_Mab
    description      text,                   -- legacy S_Desc — NOT blank-checked, unlike the cheque screen
    payer_account_id bigint      NOT NULL REFERENCES accounts(id),   -- legacy S_BesSSN (credited)
    bank_account_id  bigint      NOT NULL REFERENCES accounts(id),   -- legacy S_BankSSN (debited)
    source_module    smallint    NOT NULL DEFAULT 0,   -- legacy S_LinkPRG
    source_id        bigint,                           -- legacy S_LinkSSN
    legacy_deposit_date_jalali text,                   -- MIGRATION ONLY
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz,
    created_by       bigint      REFERENCES users(id),  -- legacy S_UserID
    updated_by       bigint      REFERENCES users(id),
    CONSTRAINT deposit_slips_amount_positive CHECK (amount > 0),                  -- [AS-IS] FISHDaryaftU.pas:386-391
    CONSTRAINT deposit_slips_method_range    CHECK (deposit_method BETWEEN 1 AND 4),  -- [NEW] §12.9
    CONSTRAINT deposit_slips_source_pair     CHECK ((source_module = 0) = (source_id IS NULL))  -- [NEW]
);

CREATE INDEX deposit_slips_year_date_idx ON deposit_slips (tenant_id, fiscal_year_id, deposit_date);
CREATE INDEX deposit_slips_payer_idx     ON deposit_slips (tenant_id, payer_account_id);
CREATE INDEX deposit_slips_bank_idx      ON deposit_slips (tenant_id, bank_account_id);
CREATE INDEX deposit_slips_source_idx    ON deposit_slips (tenant_id, source_module, source_id)
    WHERE source_id IS NOT NULL;

ALTER TABLE deposit_slips ENABLE ROW LEVEL SECURITY;
ALTER TABLE deposit_slips FORCE ROW LEVEL SECURITY;
CREATE POLICY deposit_slips_tenant_isolation ON deposit_slips
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN deposit_slips.bank_account_id IS
  'Legacy S_BankSSN. Note the naming trap: the SCREEN calls this control S_Bed and the DB column '
  'S_Bank*, mapping @BedSSN → S_BankSSN (FISHDaryaftU.pas:439-441).';
COMMENT ON COLUMN deposit_slips.payer_account_id IS
  'Legacy S_BesSSN. > 0 is enforced (FISHDaryaftU.pas:361-365) but LEAF-NESS IS NOT — unlike the '
  'cheque screens. The FK here is stricter than the legacy.';
COMMENT ON TABLE deposit_slips IS
  'Legacy DFish also has an S_DateS column that is read at FISHDaryaftU.pas:178 but declared on no '
  'dataset — either it exists and is never written, or that line raises at runtime (§12.2). '
  'Unmodelled here pending the DDL dump. Tenant-scoped per §A3 — RLS on tenant_id.';

-- DROPPED: S_StateName (denormalised channel label), S_BesCR, S_BesName, S_BankCR, S_BankName.
```

_(§11.5 "Treasury" continues in [02-11-f-ddl-treasury-2.md](02-11-f-ddl-treasury-2.md))_

---

[← 02-11-d-ddl-accounting-core.md](02-11-d-ddl-accounting-core.md) | [02-11-f-ddl-treasury-2.md →](02-11-f-ddl-treasury-2.md)
