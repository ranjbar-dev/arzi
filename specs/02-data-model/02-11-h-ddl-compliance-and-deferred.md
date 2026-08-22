_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.7 Compliance and integrations

```sql
-- =====================================================================
-- 11.7  Compliance
-- Legacy: Moadian (مؤدیان — the Iranian tax portal)
-- =====================================================================

-- RESOLVED 2026-08-19 — full DDL from a schema-only dump (Full_Script_14050527.sql, 0 data rows).
-- Previously Moadian was known only from ONE query in the whole codebase — a correlated COUNT(*)
-- at AnbarListU.pas:537:
--     Send = (Select Count(*) from Moadian
--             Where Moadian.M_link = Anbar_Factor.AF_ssn and Moadian.M_id = 1)
-- The legacy table has 19 columns, not 2:
--   M_SSN int IDENTITY(1,1), M_ID tinyint, M_Link int NOT NULL, M_Date datetime, M_UserID int,
--   M_CodeMelli varchar(15), M_Inty tinyint, M_Tob tinyint, M_Name varchar(100),
--   M_Factorinno varchar(20), M_TAXID varchar(512), M_Status varchar(512), M_REFID varchar(512),
--   M_UID varchar(512), M_ERROR varchar(512), M_OK tinyint, M_CodePosti varchar(15).
-- No PK, no FK, no index. The extra columns (M_TAXID/M_Status/M_REFID/M_UID/M_ERROR/M_OK) read as
-- request/response bookkeeping for a real e-invoice submission API call — richer than the single
-- COUNT(*) usage suggested. §12.14 (02-data-model/02-12-b.md) is closed for the DDL side; only the
-- VALUE SETS for M_ID/M_Status/M_OK etc. (§12.9) still need a populated database.

CREATE TABLE tax_submissions (
    id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id       bigint   NOT NULL REFERENCES tenants(id),  -- [NEW] §A3
    source_module   smallint NOT NULL,     -- legacy M_ID (1 = inventory invoice; full set unconfirmed, §12.9)
    source_id       bigint   NOT NULL,     -- legacy M_Link → Anbar_Factor.AF_SSN
    submitted_at    timestamptz,           -- legacy M_Date
    submitted_by    bigint REFERENCES users(id),   -- legacy M_UserID
    counterparty_national_id  text,        -- legacy M_CodeMelli varchar(15)
    counterparty_type         smallint,    -- legacy M_Inty tinyint — value set unconfirmed (§12.9)
    tax_category               smallint,   -- legacy M_Tob tinyint — value set unconfirmed (§12.9)
    counterparty_name          text,       -- legacy M_Name varchar(100)
    invoice_serial              text,      -- legacy M_Factorinno varchar(20)
    tax_authority_id             text,     -- legacy M_TAXID varchar(512)
    tax_authority_status         text,     -- legacy M_Status varchar(512)
    tax_authority_reference_id   text,     -- legacy M_REFID varchar(512)
    tax_authority_uid            text,     -- legacy M_UID varchar(512)
    tax_authority_error          text,     -- legacy M_ERROR varchar(512)
    is_ok           boolean,               -- legacy M_OK tinyint
    counterparty_postal_code text,         -- legacy M_CodePosti varchar(15)
    created_at    timestamptz NOT NULL DEFAULT now(),
    created_by    bigint REFERENCES users(id)
);

CREATE INDEX tax_submissions_source_idx ON tax_submissions (tenant_id, source_module, source_id);

ALTER TABLE tax_submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE tax_submissions FORCE ROW LEVEL SECURITY;
CREATE POLICY tax_submissions_tenant_isolation ON tax_submissions
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
```

**External catalogs — not created here.** `Anbar.dbo.Cala`, `Anbar.dbo.Anbar`,
`Anbar.dbo.FactorMaster`, `Anbar.dbo.FactorDetail`, `Saham.dbo.NSaham` and
`Rppc_Solution.dbo.NewRamz` belong to **other systems** (§1.5, §2.2 rows E1–E6). They are reached
today through separate ADO connections and, in one case, a stored procedure on a foreign connection
(`B_SelectSerial`, §3.1, §5.4). **Do not port them.** Integrate:

- `B_SelectSerial` → an integration endpoint (HTTP, or a read-only FDW/`dblink` if §12.12 confirms
  they share a server) preserving the same two-factor check: exactly one row returned, **and**
  `SerialNoPsnBts` equal to the typed current-account number (§5.7).
- `FactorMaster.FM_Factor` — the one **server-side** number allocation in the system, seeded at
  `1700001` (§5.3.4). Its format (`warehouse_id * 100000 + counter`) is printed on documents in the
  field and must be preserved (§5.7).
- Do **not** re-implement the external system's numbering.

---

### 11.8 Deferred and cross-cutting objects

```sql
-- =====================================================================
-- 11.8  Cross-cutting
-- =====================================================================

-- updated_at maintenance. The legacy sets timestamps inconsistently — Moein.M_Time is written by
-- some paths and not others (§2.7), and most treasury tables have no timestamp at all
-- (06-treasury.md §1.1). One trigger, applied uniformly, removes that class of gap.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END $$;

-- Apply to every table carrying updated_at:
--   CREATE TRIGGER <table>_set_updated_at BEFORE UPDATE ON <table>
--   FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------
-- Objects deliberately NOT created
-- ---------------------------------------------------------------------
-- temp_* tables            — transient scratch tables (§2.2 row 26); replaced by CTEs.
-- TolidMaster              — every reference in the repository is the literal placeholder SQL
--                            'Select * from tolidMaster' on a TADOQuery whose SQL is replaced at
--                            runtime (§12.14). Confirm before deleting from the target model.
--                            If retained, it is tenant-scoped like every other table — the §11.0
--                            tenant_id / RLS convention applies when (if) it is built.
-- Anbar_Tasfieh, Base_Q, QCheck, QDCheck, Jari_Rem, SahamdarConfig, KharidPeste_List
--                          — parameterised QUERIES, not tables (§2.2). They become endpoints.
-- Any licence / machine-fingerprint table — dropped (§8.6, LockUnit.pas).
```

#### 11.9 Index summary — what the legacy queries imply

None of these is observable in the source; §12.6 will reveal which already exist on the server.

| Table | Index | Serves |
|---|---|---|
| `voucher_lines` | `(fiscal_year_id, line_date)` | date-range reports, `Taraz_6Sotooni` |
| `voucher_lines` | `(fiscal_year_id, account_id, line_date, id)` | `Moein_View_Daftar`, the subsidiary ledger |
| `voucher_lines` | `(fiscal_year_id, kind, status, line_date)` incl. amounts | both trial balances |
| `voucher_lines` | `(source_module, source_id)` | every drill-down from a ledger line |
| `voucher_lines` | `(voucher_id)` | voucher detail |
| `vouchers` | `(fiscal_year_id, voucher_number)` UNIQUE | `MoeinViewSanad`, `Moein_ChapSanad`, `MergeSanad` |
| `vouchers` | `(fiscal_year_id, voucher_date)` partial on `draft` | `Get_NewSanad_DateID` (§5.3.2) |
| `accounts` | `(gl, sub, a1, a2)` UNIQUE | every account lookup (`Dmu.pas:1152-1156`) |
| `accounts` | trigram on `name` | `Sarfasl_Seek_Name` |
| `parties` | `(card_number)` UNIQUE | `Sahamdar_Seek` |
| `cheques` | `(due_date)` | the ageing filter and the list's sole sort key (`CheckListDU.pas:329`) |
| `cheques` | `(fiscal_year_id, status)` | the cheque list's state tabs |
| `cheque_events` | `(cheque_id, id)` | history, ordered by id (`CheckListDU.pas:164`) |
| `deposit_slips` | `(fiscal_year_id, deposit_date)` | slip list |
| `inventory_invoices` | `(fiscal_year_id, invoice_number)` UNIQUE | invoice lookup (`AnbarFactorU.pas:704`) |
| `inventory_invoices` | `(fiscal_year_id, invoice_date, invoice_number)` | the list's `ORDER BY AF_Date, AF_Factor DESC` (`AnbarListU.dfm:2121`) |
| `inventory_invoice_lines` | `(item_id, fiscal_year_id)` incl. `quantity, unit_price` | stock on hand (`Dmu.dfm:667-690`) |
| `items` | `(warehouse_id, code)` | `Anbar_AjnasView` |
| `items` | trigram on `name` | `AnbarCala_SeekName`'s `PATINDEX('%…%')` scan |
| `user_permissions` | PK `(user_id, permission_id)` | the ~35 permission checks per login (`Mainu.pas:907-953`) |


---

[← 02-11-g-ddl-inventory.md](02-11-g-ddl-inventory.md) | [02-12-a-open-questions-dates-and-procedures.md →](02-12-a-open-questions-dates-and-procedures.md)
