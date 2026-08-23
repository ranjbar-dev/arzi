-- Step 5.2 (docs/phase-5-inventory.md §5.2 / specs/05-inventory/05-03-a/b-document-types.md,
-- 05-04-a/b/c-invoice-factor-lifecycle.md): a real, queryable document lifecycle replacing
-- subsystem A's "entirely derived from three other tables" state (§4.0) — a document exists, an
-- explicit `status` column tracks draft/posted/frozen, and header<->line link by real surrogate
-- FK (not the legacy's mutable `AF_Factor` business-number link, §4.2.6's "any table the author
-- forgot silently orphans records" hazard).
--
-- Scope note: only the four subsystem-A document types (§3.1) — production/transfer (subsystem
-- B's 15/25/16/26) are 5.8's job to add, per that step's own Build bullet; not reached into here.

-- One atomic per-fiscal-year sequence shared across all four document types (§4.2.2 step 5 —
-- "receipts, issues and both returns draw from one sequence"), same allocator pattern as
-- fiscal_years.next_voucher_number (2.3), not the legacy's racy `SELECT MAX(AF_Factor)+1`.
ALTER TABLE fiscal_years ADD COLUMN next_inventory_document_number integer NOT NULL DEFAULT 1;

CREATE TYPE inventory_document_type AS ENUM ('receipt', 'issue', 'purchase_return', 'sales_return');

-- draft: saved, no voucher posted yet, fully editable/deletable (this step's own reach).
-- posted: a voucher has been generated and linked (5.8), still editable per the legacy's actual
--   model, not deletable once settled (5.7's settlement links).
-- frozen: the linked voucher left draft state in the accounting module — read-only (§4.1's
--   "someone finalises the voucher" transition). Both later transitions are declared here so the
--   schema doesn't need another migration when 5.7/5.8 land, but nothing in this step ever writes
--   anything but 'draft'.
CREATE TYPE inventory_document_status AS ENUM ('draft', 'posted', 'frozen');

-- ---------------------------------------------------------------------
-- inventory_documents   <- merges legacy Anbar_Factor + FactorMaster (§3.4's "rebuild target")
-- ---------------------------------------------------------------------
CREATE TABLE inventory_documents (
    id                       bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id                bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id           bigint  NOT NULL REFERENCES fiscal_years(id),          -- AF_COID
    document_type            inventory_document_type NOT NULL,                     -- AF_Type
    status                   inventory_document_status NOT NULL DEFAULT 'draft',
    document_number          integer NOT NULL,                                     -- AF_Factor —
                                                                                     -- one sequence per
                                                                                     -- fiscal year across
                                                                                     -- all four types (§4.2.2 step 5)
    document_date            date    NOT NULL,                                     -- AF_Date, real date not
                                                                                     -- a Jalali string (05-05-a §5.1.2)
    warehouse_id             bigint  NOT NULL REFERENCES warehouses(id),           -- subsystem B's real
                                                                                     -- warehouse dimension (FM_Anbar),
                                                                                     -- adopted over subsystem A's none
    -- B7 fix: required, no zero-sentinel. The legacy's guard ("if not S_Bed.tag=0") was an
    -- unreachable operator-precedence bug that let AF_Customer=0 through — this column simply
    -- cannot be null or point at a non-leaf account (enforced in api/src/inventory_documents.rs,
    -- the same reasoning as the FK CHECK a plain NOT NULL can't express: leaf-ness).
    counterparty_account_id  bigint  NOT NULL REFERENCES accounts(id),             -- AF_Customer
    description              text,                                                 -- AF_Desc
    gross_amount             bigint  NOT NULL DEFAULT 0,                           -- AF_Mab (naming trap
                                                                                     -- fixed: this really is gross)
    discount_amount          bigint  NOT NULL DEFAULT 0,                           -- AF_Kasr
    tax_amount                bigint  NOT NULL DEFAULT 0,                           -- AF_Maliat
    total_amount              bigint  NOT NULL DEFAULT 0,                           -- AF_Total = gross + tax - discount
    posted_voucher_id         bigint  REFERENCES vouchers(id),                      -- AF_Sanad — left NULL by
                                                                                     -- every handler in this step,
                                                                                     -- wired by 5.8
    created_at                timestamptz NOT NULL DEFAULT now(),
    updated_at                timestamptz,
    created_by                bigint  REFERENCES users(id),
    updated_by                bigint  REFERENCES users(id),
    CONSTRAINT inventory_documents_number_key UNIQUE (tenant_id, fiscal_year_id, document_number)
);

CREATE INDEX inventory_documents_tenant_idx ON inventory_documents (tenant_id, fiscal_year_id);
CREATE INDEX inventory_documents_type_status_idx
    ON inventory_documents (tenant_id, fiscal_year_id, document_type, status);
CREATE INDEX inventory_documents_warehouse_idx ON inventory_documents (tenant_id, warehouse_id);

ALTER TABLE inventory_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE inventory_documents FORCE ROW LEVEL SECURITY;
CREATE POLICY inventory_documents_tenant_isolation ON inventory_documents
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- inventory_document_lines   <- merges legacy Anbar_FactorD + FactorDetail
-- Linked to its header by real surrogate FK (document_id), never a business number — closes the
-- §4.2.6 hazard (renumbering an invoice no longer needs to rewrite five separate tables).
-- ---------------------------------------------------------------------
CREATE TABLE inventory_document_lines (
    id               bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id        bigint  NOT NULL REFERENCES tenants(id),
    fiscal_year_id   bigint  NOT NULL REFERENCES fiscal_years(id),   -- denormalised for scoping, same
                                                                       -- convention as voucher_lines (2.3)
    document_id      bigint  NOT NULL REFERENCES inventory_documents(id),
    item_id          bigint  NOT NULL REFERENCES items(id),          -- AFD_Code
    quantity         numeric(14,3) NOT NULL,                         -- AFD_Num, always positive (§3.1.2)
    unit_price       bigint  NOT NULL DEFAULT 0,                     -- AFD_Phi
    gross_amount     bigint  NOT NULL DEFAULT 0,                     -- AFD_Kol = round(quantity * unit_price) —
                                                                       -- correctly rounded here, not the legacy's
                                                                       -- trunc() (05-06-a §6.1's bias finding)
    discount_amount  bigint  NOT NULL DEFAULT 0,                     -- AFD_Kasr — absolute only in this step;
                                                                       -- 5.5 adds the percentage entry mode on top
    tax_amount       bigint  NOT NULL DEFAULT 0,                     -- AFD_Maliat
    total_amount     bigint  NOT NULL DEFAULT 0,                     -- AFD_Total = gross + tax - discount
                                                                       -- (§4.2.2 step 10's exact formula)
    description      text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz,
    created_by       bigint  REFERENCES users(id),
    updated_by       bigint  REFERENCES users(id),
    CONSTRAINT inventory_document_lines_quantity_positive CHECK (quantity > 0)
);

CREATE INDEX inventory_document_lines_document_idx ON inventory_document_lines (document_id);
CREATE INDEX inventory_document_lines_item_idx ON inventory_document_lines (tenant_id, item_id);

ALTER TABLE inventory_document_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE inventory_document_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY inventory_document_lines_tenant_isolation ON inventory_document_lines
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
