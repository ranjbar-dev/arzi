_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.6 Inventory

`warehouses`, `units_of_measure`, `product_grades` and `items` carry no `fiscal_year_id` — not
year-scoped, exactly as in the legacy (§8.4, §2.2). All four **do** carry `tenant_id` (§A3, §11.0
convention 1): they are exactly the "global master data" the A3 ruling calls out by name — each
client has their own warehouses, units of measure, product grades and item catalogue.
`inventory_invoices` / `inventory_invoice_lines` keep their existing `fiscal_year_id` and gain
`tenant_id` alongside it, denormalised rather than derived (§11.0 convention 2).

```sql
-- =====================================================================
-- 11.6  Inventory
-- Legacy: Anbar_Config, Anbar_Vahed, Kinds, Anbar_Jens,
--         Anbar_Factor, Anbar_FactorD
-- Cross-reference: docs/05-inventory.md
-- =====================================================================

-- ---------------------------------------------------------------------
-- warehouses   <- legacy Anbar_Config   §8.4
-- RESOLVED (§12.5, §12.15 item 2): legacy PK is Anbar_Config.AC_ID int NOT NULL PRIMARY KEY.
--    Confirmed by a schema-only dump — CONFIRMED, not inferred. Anbar_Jens.AJ_ID is confirmed as
--    its foreign key by Anbar_AddToFactor's body (§12.3 item 4), which looks up
--    Anbar_Config WHERE AC_ID = (SELECT AJ_ID FROM Anbar_Jens WHERE AJ_Code=@Code) — a direct
--    reference, not a presumption. Full legacy Anbar_Config columns: AC_ID, AC_Name varchar(50),
--    AC_Kharid/AC_Foroosh/AC_BKharid/AC_BForoosh/AC_Maliat int DEFAULT (0), AC_Kasr int (no
--    default), AC_DMaliat varchar(5) DEFAULT ('6.0'). No FK, no index — matches §11's "add what's
--    missing" premise.
-- ---------------------------------------------------------------------
CREATE TABLE warehouses (
    id                          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id                   bigint      NOT NULL REFERENCES tenants(id),   -- [NEW] §A3
    legacy_id                   integer UNIQUE,          -- MIGRATION ONLY — the value in AJ_ID (default 93 is real)
    name                        text        NOT NULL,    -- legacy AC_Name
    default_tax_rate            numeric(5,2) NOT NULL DEFAULT 0,   -- legacy AC_DMaliat — VAT %
    purchase_account_id         bigint REFERENCES accounts(id),    -- legacy AC_Kharid     خرید
    purchase_return_account_id  bigint REFERENCES accounts(id),    -- legacy AC_BKharid    برگشت خرید
    sales_account_id            bigint REFERENCES accounts(id),    -- legacy AC_Foroosh    فروش
    sales_return_account_id     bigint REFERENCES accounts(id),    -- legacy AC_BForoosh   برگشت فروش
    deduction_account_id        bigint REFERENCES accounts(id),    -- legacy AC_Kasr       کسر
    tax_account_id              bigint REFERENCES accounts(id),    -- legacy AC_Maliat     مالیات
    created_at                  timestamptz NOT NULL DEFAULT now(),
    updated_at                  timestamptz,
    created_by                  bigint REFERENCES users(id),
    updated_by                  bigint REFERENCES users(id),
    CONSTRAINT warehouses_name_nonblank CHECK (length(btrim(name)) > 0),               -- [NEW]
    CONSTRAINT warehouses_tax_rate_range
        CHECK (default_tax_rate >= 0 AND default_tax_rate <= 100)                      -- [NEW] §8.6
);

CREATE INDEX warehouses_tenant_idx ON warehouses (tenant_id);

ALTER TABLE warehouses ENABLE ROW LEVEL SECURITY;
ALTER TABLE warehouses FORCE ROW LEVEL SECURITY;
CREATE POLICY warehouses_tenant_isolation ON warehouses
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN warehouses.default_tax_rate IS
  'Legacy AC_DMaliat, written from a TEdit AS A STRING with no numeric validation beyond "not '
  'blank" (AnbarTanzimU.pas:186), and read AsFloat (AnbarFactorAddU.pas:145). Exact numeric here '
  'removes a class of off-by-one-rial bugs (§7.7).';
COMMENT ON TABLE warehouses IS
  'The six *_account_id columns are the only inventory settings that change accounting behaviour; '
  'every change must be audited (§13.19). All six are picked through the Taraf 4-segment account '
  'widget, confirming that Taraf is a PICKER, not a table (01-glossary.md §6b).';


-- ---------------------------------------------------------------------
-- units_of_measure   <- legacy Anbar_Vahed   (AnbarCalaAddU.dfm:268)
-- ---------------------------------------------------------------------
CREATE TABLE units_of_measure (
    id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id  bigint      NOT NULL REFERENCES tenants(id),   -- [NEW] §A3
    legacy_id  integer UNIQUE,        -- legacy AV_Code
    name       text NOT NULL,         -- legacy AV_Name  واحد
    CONSTRAINT units_of_measure_name_key UNIQUE (tenant_id, name)     -- changed from a global UNIQUE — §A3
);

CREATE INDEX units_of_measure_tenant_idx ON units_of_measure (tenant_id);

ALTER TABLE units_of_measure ENABLE ROW LEVEL SECURITY;
ALTER TABLE units_of_measure FORCE ROW LEVEL SECURITY;
CREATE POLICY units_of_measure_tenant_isolation ON units_of_measure
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);


-- ---------------------------------------------------------------------
-- product_grades   <- legacy Kinds   §2.2 (confidence C)
-- Pistachio grading (05-inventory.md §8). Bound as TDM.Kind_Table (Dmu.dfm:301-307), a bare
-- TADOTable with NO persistent fields, consumed as a lookup list by PestehD_U.pas (DS_Kinds).
-- ⚠ A schema-only dump (Full_Script_14050527.sql, 2026-08-19) does NOT contain a table named
--    Kinds, under any case/spelling — searched across all 39 CREATE TABLE statements. Either it
--    was dropped before that dump was taken, the confidence-C listing in §2.2 was itself wrong, or
--    it lives in a different database on the same server. Column names remain UNKNOWN either way —
--    this is now open item 11-open-decisions.md A14, not simply "not yet dumped."
-- ---------------------------------------------------------------------
CREATE TABLE product_grades (
    id        bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id bigint      NOT NULL REFERENCES tenants(id),   -- [NEW] §A3
    legacy_id integer UNIQUE,
    name      text NOT NULL          -- the display column the combo binds to — name unknown
    -- Further columns depend on 11-open-decisions.md A14 (does Kinds exist at all).
);

CREATE INDEX product_grades_tenant_idx ON product_grades (tenant_id);

ALTER TABLE product_grades ENABLE ROW LEVEL SECURITY;
ALTER TABLE product_grades FORCE ROW LEVEL SECURITY;
CREATE POLICY product_grades_tenant_isolation ON product_grades
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);


-- ---------------------------------------------------------------------
-- items   <- legacy Anbar_Jens   (AnbarCalaAddU.pas:160-176)
-- NOT year-scoped (§2.2), but IS tenant-scoped (§A3) — each client has their own item catalogue.
-- ---------------------------------------------------------------------
CREATE TABLE items (
    id                   bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id            bigint  NOT NULL REFERENCES tenants(id),   -- [NEW] §A3
    code                 integer NOT NULL,        -- legacy AJ_Code — the business key, manually assigned
    warehouse_id         bigint  NOT NULL REFERENCES warehouses(id),   -- legacy AJ_ID
    name                 text    NOT NULL,        -- legacy AJ_Name
    specification        text,                    -- legacy AJ_Prop
    unit_of_measure_id   bigint  REFERENCES units_of_measure(id),      -- legacy AJ_VahedC
    unit_of_measure_text text,                    -- legacy AJ_Vahed — the denormalised free-text copy
    unit_of_measure_2    text,                    -- legacy AJ_Vahed2   (see COMMENT)
    unit_of_measure_3    text,                    -- legacy AJ_Vahed3
    default_unit_price   bigint  NOT NULL DEFAULT 0,      -- legacy AJ_Phi — rial per unit
    is_taxable           boolean NOT NULL DEFAULT true,   -- legacy AJ_Maliat 0/1
    allow_negative_stock boolean NOT NULL DEFAULT false,  -- legacy AJ_Manfi 0/1
    reorder_level        numeric(18,3),                   -- legacy AJ_Alarm
    tax_system_item_code text,                            -- legacy SSTID varchar(13)
    updated_from_host    text,                            -- legacy AJ_Net — the workstation name
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz,                     -- legacy AJ_DateTime (GetDate())
    created_by           bigint REFERENCES users(id),
    updated_by           bigint REFERENCES users(id),     -- legacy AJ_UserID
    CONSTRAINT items_code_key       UNIQUE (tenant_id, code),               -- changed from a global UNIQUE — §A3
    CONSTRAINT items_name_nonblank  CHECK (length(btrim(name)) > 0),        -- [AS-IS] AnbarCalaAddU.pas
    CONSTRAINT items_price_nonneg   CHECK (default_unit_price >= 0),        -- [NEW]
    CONSTRAINT items_sstid_len      CHECK (tax_system_item_code IS NULL
                                           OR length(tax_system_item_code) <= 13)   -- [AS-IS] 13-char declared local
);

CREATE INDEX items_warehouse_idx ON items (tenant_id, warehouse_id, code);
CREATE INDEX items_name_idx      ON items (tenant_id, name);
-- The legacy name search is `PATINDEX('%'+@Name+'%', AJ_Name) > 0` (Dmu.dfm AnbarCala_SeekName) —
-- a leading-wildcard scan. A trigram index makes it sargable:
-- CREATE INDEX items_name_trgm_idx ON items USING gin (name gin_trgm_ops);

ALTER TABLE items ENABLE ROW LEVEL SECURITY;
ALTER TABLE items FORCE ROW LEVEL SECURITY;
CREATE POLICY items_tenant_isolation ON items
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN items.allow_negative_stock IS
  'Legacy AJ_Manfi (منفی = negative). Checked at AnbarFactorAddU.pas:177 before allowing an issue '
  'that would drive stock below zero. See 05-inventory.md §5.2.';
COMMENT ON COLUMN items.is_taxable IS
  'Legacy AJ_Maliat. When 0 the VAT rate is FORCED to 0 on the invoice line, overriding the '
  'warehouse default (AnbarFactorAddU.pas:145-146, §7.4).';
COMMENT ON COLUMN items.unit_of_measure_2 IS
  '⚠ Legacy AJ_Vahed2 / AJ_Vahed3. Their purpose is unconfirmed. Anbar_Mandeh returns TWO parallel '
  'quantity/value pairs (R1/R2, TedIn1/2, Mabin1/2, Phiin1/2 — §3.1), which may be two units of '
  'measure or two warehouses. Resolve via §12.3 item 10 before designing dual-unit stock.';
COMMENT ON COLUMN items.tax_system_item_code IS
  'Legacy SSTID — the Iranian tax portal (سامانه مؤدیان) item code. Feeds the Moadian submission.';


-- ---------------------------------------------------------------------
-- inventory_invoices / _lines   <- legacy Anbar_Factor / Anbar_FactorD
-- ---------------------------------------------------------------------
CREATE TABLE inventory_invoices (
    id                      bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy AF_SSN
    tenant_id               bigint  NOT NULL REFERENCES tenants(id),          -- [NEW] §A3
    fiscal_year_id          bigint  NOT NULL REFERENCES fiscal_years(id),      -- legacy AF_COID
    invoice_number          integer NOT NULL,     -- legacy AF_Factor — allocated from the year counter (§5.7)
    document_type           smallint NOT NULL,    -- legacy AF_Type: 1 receipt, 2 issue, 3–9 unlabelled (§12.9)
    invoice_date            date    NOT NULL,     -- legacy AF_Date
    counterparty_account_id bigint  NOT NULL REFERENCES accounts(id),          -- legacy AF_Customer
    voucher_number          integer,              -- legacy AF_Sanad — NULL until posted
    subtotal                bigint  NOT NULL DEFAULT 0,   -- legacy AF_Mab    = SUM(line_gross)
    total_deduction         bigint  NOT NULL DEFAULT 0,   -- legacy AF_Kasr   = SUM(line_deduction)
    total_tax               bigint  NOT NULL DEFAULT 0,   -- legacy AF_Maliat = SUM(line_tax)
    total_amount            bigint  NOT NULL DEFAULT 0,   -- legacy AF_Total  = SUM(line_total)
    description             text,                 -- legacy AF_Desc
    legacy_invoice_date_jalali text,              -- MIGRATION ONLY
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz,
    created_by              bigint REFERENCES users(id),
    updated_by              bigint REFERENCES users(id),
    CONSTRAINT inventory_invoices_number_key
        UNIQUE (tenant_id, fiscal_year_id, invoice_number),                    -- tenant_id led in, per §11.0 convention 6 — §A3
    CONSTRAINT inventory_invoices_type_range CHECK (document_type BETWEEN 1 AND 9),   -- [NEW] §12.9
    CONSTRAINT inventory_invoices_totals_nonneg
        CHECK (subtotal >= 0 AND total_deduction >= 0 AND total_tax >= 0 AND total_amount >= 0)  -- [NEW]
    -- DELIBERATELY OMITTED: CHECK (total_amount = subtotal + total_tax - total_deduction).
    -- §7.7 check 3 warns that the truncation compounding in AnbarFactorAddU.pas:107,168-170 means
    -- stored totals MAY NOT REPRODUCE from stored components. Adding this constraint would reject
    -- historical invoices. Run the probe first; adopt only if it comes back clean.
);

CREATE INDEX inventory_invoices_year_date_idx ON inventory_invoices (tenant_id, fiscal_year_id, invoice_date, invoice_number);
CREATE INDEX inventory_invoices_counterparty_idx ON inventory_invoices (tenant_id, counterparty_account_id);
CREATE INDEX inventory_invoices_voucher_idx ON inventory_invoices (tenant_id, fiscal_year_id, voucher_number)
    WHERE voucher_number IS NOT NULL;

ALTER TABLE inventory_invoices ENABLE ROW LEVEL SECURITY;
ALTER TABLE inventory_invoices FORCE ROW LEVEL SECURITY;
CREATE POLICY inventory_invoices_tenant_isolation ON inventory_invoices
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE inventory_invoices IS
  '⚠ NOT MODELLED HERE: the legacy repeating groups AF_Desc1..5 / AF_Date1..5 / AF_Mab1..5 / '
  'AF_Sel2..5 — a five-slot settlement array flattened into columns, surfacing in the Anbar_Tasfieh '
  'settlement query (Dmu.dfm:1017-1110). The correct target is a child table '
  'inventory_invoice_settlements, which is a STRUCTURAL CHANGE (§14.14, §13). A schema-only dump '
  '(2026-08-19) CONFIRMS all 32 Anbar_Factor columns exist with these exact names and types '
  '(AF_Sel1..5 int, AF_Date1..5 varchar(10), AF_Mab1..5 bigint, AF_Desc1..5 varchar(100/200), '
  'plus a table-level extended property naming AF_Kasr "takhfif" i.e. discount) — the physical '
  'shape is no longer inferred. Their business MEANING (what each of the 5 slots represents) is '
  'not stated anywhere in the DDL and still comes only from Anbar_Tasfieh usage in source.';
COMMENT ON COLUMN inventory_invoices.total_amount IS
  'Legacy AF_Total, maintained by four separate correlated-subquery UPDATEs '
  '(AnbarFactorU.pas:654-660) run OUTSIDE a transaction. See §9.4 for the torn-document failure mode.';

CREATE TABLE inventory_invoice_lines (
    id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- legacy AFD_SSN
    tenant_id       bigint  NOT NULL REFERENCES tenants(id),           -- [NEW] denormalised from inventory_invoices, see §11.0 convention 2
    invoice_id      bigint  NOT NULL REFERENCES inventory_invoices(id) ON DELETE CASCADE,  -- legacy AFD_Factor + AFD_Coid
    fiscal_year_id  bigint  NOT NULL REFERENCES fiscal_years(id),      -- legacy AFD_Coid
    line_number     integer NOT NULL,                                  -- [NEW]
    item_id         bigint  NOT NULL REFERENCES items(id),             -- legacy AFD_Code
    specification   text,                            -- legacy AFD_Prop
    unit_of_measure text,                            -- legacy AFD_Vahed
    quantity        numeric(18,3) NOT NULL,          -- legacy AFD_Num
    unit_price      bigint  NOT NULL DEFAULT 0,      -- legacy AFD_Phi
    line_gross      bigint  NOT NULL DEFAULT 0,      -- legacy AFD_Kol   — computed CLIENT-SIDE, see COMMENT
    line_deduction  bigint  NOT NULL DEFAULT 0,      -- legacy AFD_Kasr
    line_tax        bigint  NOT NULL DEFAULT 0,      -- legacy AFD_Maliat
    line_total      bigint  NOT NULL DEFAULT 0,      -- legacy AFD_Total
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz,
    created_by      bigint REFERENCES users(id),
    updated_by      bigint REFERENCES users(id),
    CONSTRAINT inventory_invoice_lines_number_key UNIQUE (tenant_id, invoice_id, line_number),   -- tenant_id led in — §A3
    CONSTRAINT inventory_invoice_lines_quantity_nonzero CHECK (quantity <> 0),        -- [NEW]
    CONSTRAINT inventory_invoice_lines_money_nonneg
        CHECK (unit_price >= 0 AND line_gross >= 0 AND line_deduction >= 0
               AND line_tax >= 0 AND line_total >= 0)                                 -- [NEW]
);

CREATE INDEX inventory_invoice_lines_invoice_idx ON inventory_invoice_lines (tenant_id, invoice_id, line_number);
CREATE INDEX inventory_invoice_lines_item_idx    ON inventory_invoice_lines (tenant_id, item_id);
-- The stock-on-hand query sums AFD_Num per item across invoice types (Dmu.dfm:667-690), so:
CREATE INDEX inventory_invoice_lines_stock_idx
    ON inventory_invoice_lines (tenant_id, item_id, fiscal_year_id) INCLUDE (quantity, unit_price);

ALTER TABLE inventory_invoice_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE inventory_invoice_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY inventory_invoice_lines_tenant_isolation ON inventory_invoice_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON COLUMN inventory_invoice_lines.line_gross IS
  'Legacy AFD_Kol. ⚠ The CLIENT computes line_gross, line_deduction and line_tax and passes them to '
  'Anbar_AddToFactor (AnbarFactorAddU.pas:107,168-170) — the money arithmetic lives in the client '
  'and the procedure must NOT recompute it (§3.1). The rebuild must reproduce the same rounding '
  'rule (truncate toward zero on this path — §7.4, §13.16) or the totals change.';
COMMENT ON COLUMN inventory_invoice_lines.quantity IS
  'Legacy AFD_Num. ⚠ Crosses the wire to Anbar_AddToFactor as ftFloat — the ONE place a '
  'floating-point quantity is used, while the column itself is decimal(_,3) (§3.4 item 2). '
  'Rounding at that boundary must be checked against live data before cutover.';

-- DROPPED legacy columns, and why:
--   AFD_Type, AFD_Date    — copies of the header's type and date.
--   AFD_Customer          — a back-filled copy of AF_Customer (Anbar_Amalkard.pas:168).
--   AFD_Name              — denormalised item name (§13.11).
--   AFD_Vahed2, AFD_Vahed3 — purpose unconfirmed, mirroring items.unit_of_measure_2/3.
--   AFD_TypeN, AFD_IN, AFD_OUT, AF_CustomerN, Af_typeN — computed in SQL (CASE expressions,
--     AnbarListU.pas:540), never stored.

-- Legacy → Rust:
--   Anbar_AddToFactor (AnbarFactorU.dfm:433) → inventory::invoices::add_line()
--     ⚠ HIGH extraction priority (§12.3 item 4) — it maintains the four header totals and the
--       line count. In the rebuild those are maintained in the SAME transaction as the line.
--   Anbar_Mandeh (Dmu.dfm:721)     → inventory::stock_on_hand()  — §12.3 item 10
--   Anbar_CardJensi (Dmu.dfm:453)  → inventory::stock_card()     — §12.3 item 11
--   Anbar_AjnasView (Dmu.dfm:396)  → inventory::items::list()
--   Anbar_PrintFactor (Dmu.dfm:532)→ reporting::invoice_print()
--   Anbar_ReportKharidForoosh      → reporting::purchase_sales()
--     ⚠ Takes NO fiscal-year parameter; the date range alone scopes it (§12.12).
--   Dmu.pas:1258 (MAX(AF_Factor)+1)→ numbering::next_invoice_number() (§5.7)
--   AnbarFactorU.pas:654-660 (four untransacted total UPDATEs) → one statement, one transaction.
```

---


---

[← 02-11-f-ddl-treasury-2.md](02-11-f-ddl-treasury-2.md) | [02-11-h-ddl-compliance-and-deferred.md →](02-11-h-ddl-compliance-and-deferred.md)
