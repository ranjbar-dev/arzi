-- Step 5.1 (docs/phase-5-inventory.md §5.1 / specs/05-inventory/05-01-entity-model.md,
-- 05-02-a/b-item-master-crud-rules.md): one merged item master + warehouse model, replacing the
-- legacy's two structurally-incompatible subsystems (subsystem A's Anbar_Jens/Anbar_Config,
-- single-scalar home-warehouse; subsystem B's Cala/Anbar, CSV-column multi-warehouse membership).
-- §1.0's "do not build two and reconcile later" is why there is exactly one of each table here.
--
-- Global per-tenant master data (no fiscal-year column — §1.0: "Anbar_Jens has no *_COID column",
-- same as Anbar_Config/Anbar_Vahed/Kinds), but real tenant_id + RLS on every table per A3, same
-- convention as every other "legacy global" table (accounts, parties, party_account_config).

-- ---------------------------------------------------------------------
-- warehouses   <- legacy Anbar_Config
-- ---------------------------------------------------------------------
CREATE TABLE warehouses (
    id                          bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id                   bigint  NOT NULL REFERENCES tenants(id),
    name                        text    NOT NULL,                                -- AC_Name
    vat_rate_pct                numeric(5,2) NOT NULL DEFAULT 0,                 -- AC_DMaliat
    purchase_account_id         bigint  NOT NULL REFERENCES accounts(id),        -- AC_Kharid
    purchase_return_account_id  bigint  NOT NULL REFERENCES accounts(id),        -- AC_BKharid
    sales_account_id            bigint  NOT NULL REFERENCES accounts(id),        -- AC_Foroosh
    sales_return_account_id     bigint  NOT NULL REFERENCES accounts(id),        -- AC_BForoosh
    discount_account_id         bigint  NOT NULL REFERENCES accounts(id),        -- AC_Kasr
    vat_account_id              bigint  NOT NULL REFERENCES accounts(id),        -- AC_Maliat
    -- §1.1: the legacy's "N2" delete menu item was declared and never implemented, so a warehouse
    -- was permanent once created. The rebuild adds a real deactivate action (once empty) instead.
    is_active                   boolean NOT NULL DEFAULT true,
    created_at                  timestamptz NOT NULL DEFAULT now(),
    updated_at                  timestamptz,
    created_by                  bigint  REFERENCES users(id),
    updated_by                  bigint  REFERENCES users(id),
    CONSTRAINT warehouses_name_nonblank CHECK (length(btrim(name)) > 0)
);

CREATE INDEX warehouses_tenant_idx ON warehouses (tenant_id);

ALTER TABLE warehouses ENABLE ROW LEVEL SECURITY;
ALTER TABLE warehouses FORCE ROW LEVEL SECURITY;
CREATE POLICY warehouses_tenant_isolation ON warehouses
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- units_of_measure   <- legacy Anbar_Vahed
-- §1.3: no conversion factor existed in the legacy at all ("the system cannot tell a kilogram
-- from an 'each'"), which the spec calls out as real truncation damage elsewhere — added here
-- deliberately, not scope creep. base_unit_id is a single-level reference only (no chained
-- conversions) — a deliberate scope limit, not attempted since nothing in this step needs it.
-- ---------------------------------------------------------------------
CREATE TABLE units_of_measure (
    id                 bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id          bigint  NOT NULL REFERENCES tenants(id),
    name               text    NOT NULL,                                -- AV_Name
    base_unit_id       bigint  REFERENCES units_of_measure(id),
    conversion_factor  numeric(18,6) NOT NULL DEFAULT 1,                 -- to base_unit_id, when set
    created_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT units_of_measure_name_nonblank CHECK (length(btrim(name)) > 0),
    CONSTRAINT units_of_measure_not_own_base CHECK (base_unit_id IS NULL OR base_unit_id <> id),
    CONSTRAINT units_of_measure_factor_positive CHECK (conversion_factor > 0),
    CONSTRAINT units_of_measure_name_key UNIQUE (tenant_id, name)
);

CREATE INDEX units_of_measure_tenant_idx ON units_of_measure (tenant_id);

ALTER TABLE units_of_measure ENABLE ROW LEVEL SECURITY;
ALTER TABLE units_of_measure FORCE ROW LEVEL SECURITY;
CREATE POLICY units_of_measure_tenant_isolation ON units_of_measure
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- pistachio_grades   <- legacy Kinds
-- §1.4/§8.1: kept a genuinely separate table, related to items/accounts only by explicit FK —
-- the legacy let one integer double as grade id, item code AND an account-code segment, which is
-- exactly the hazard this table exists to close off. Seeded per-tenant via the admin
-- seed-defaults endpoint (same "no tenant-provisioning flow exists yet" gap 2.1/3.1 documented),
-- not baked into this migration for tenants that don't exist yet.
-- ---------------------------------------------------------------------
CREATE TABLE pistachio_grades (
    id          bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id   bigint  NOT NULL REFERENCES tenants(id),
    name        text    NOT NULL,                                       -- K_name
    sort_order  integer NOT NULL,                                       -- legacy K_id ordering, 1..7
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pistachio_grades_name_nonblank CHECK (length(btrim(name)) > 0),
    CONSTRAINT pistachio_grades_sort_order_key UNIQUE (tenant_id, sort_order)
);

CREATE INDEX pistachio_grades_tenant_idx ON pistachio_grades (tenant_id);

ALTER TABLE pistachio_grades ENABLE ROW LEVEL SECURITY;
ALTER TABLE pistachio_grades FORCE ROW LEVEL SECURITY;
CREATE POLICY pistachio_grades_tenant_isolation ON pistachio_grades
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- items   <- merges legacy Anbar_Jens (subsystem A) + Cala (subsystem B)
-- §1.6: "the two item masters are structurally incompatible" — Cala.C_Anbar's delimited-string
-- multi-warehouse membership was the correct instinct, ported here as a real junction table
-- (item_warehouses below) rather than either legacy's own representation.
-- ---------------------------------------------------------------------
CREATE TABLE items (
    id                     bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id               bigint  NOT NULL REFERENCES tenants(id),
    code                    integer NOT NULL,                           -- AJ_Code — operator-assigned,
                                                                         -- immutable after create (§2.1/§2.3)
    name                    text    NOT NULL,                           -- AJ_Name
    specification           text,                                       -- AJ_Prop
    unit_of_measure_id      bigint  NOT NULL REFERENCES units_of_measure(id),  -- AJ_VahedC
    sale_price              bigint  NOT NULL,                           -- AJ_Phi, rials
    min_stock               bigint  NOT NULL DEFAULT 0,                 -- AJ_Alarm — a real alert once 5.3 lands
    is_taxable              boolean NOT NULL DEFAULT false,             -- AJ_Maliat — legacy default (§2.2.2)
    allow_negative_stock    boolean NOT NULL DEFAULT true,              -- AJ_Manfi — legacy default (§2.2.2),
                                                                         -- preserved as-is per §2.7's "port as-is"
    tax_item_code           text,                                       -- SSTID
    is_active               boolean NOT NULL DEFAULT true,              -- new — legacy had hard-delete only (§1.2)
    pistachio_grade_id      bigint  REFERENCES pistachio_grades(id),    -- explicit FK (§1.4/§8.1.1's fix),
                                                                         -- never a shared integer convention
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz,
    created_by              bigint  REFERENCES users(id),
    updated_by              bigint  REFERENCES users(id),
    CONSTRAINT items_name_nonblank CHECK (length(btrim(name)) > 0),
    CONSTRAINT items_sale_price_nonzero CHECK (sale_price <> 0),        -- [AS-IS] §2.2 check 4 — negative allowed
    CONSTRAINT items_code_key UNIQUE (tenant_id, code)
);

CREATE INDEX items_tenant_idx ON items (tenant_id);
CREATE INDEX items_name_idx ON items (tenant_id, name);

ALTER TABLE items ENABLE ROW LEVEL SECURITY;
ALTER TABLE items FORCE ROW LEVEL SECURITY;
CREATE POLICY items_tenant_isolation ON items
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- item_warehouses   <- replaces legacy Anbar_Jens.AJ_ID (single scalar) and Cala.C_Anbar (CSV)
-- §1.6's explicit call-out: merging the two item masters "requires exactly this" — a real
-- many-to-many junction, not either legacy representation.
-- ---------------------------------------------------------------------
CREATE TABLE item_warehouses (
    id            bigint  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id     bigint  NOT NULL REFERENCES tenants(id),
    item_id       bigint  NOT NULL REFERENCES items(id),
    warehouse_id  bigint  NOT NULL REFERENCES warehouses(id),
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT item_warehouses_unique UNIQUE (item_id, warehouse_id)
);

CREATE INDEX item_warehouses_tenant_idx ON item_warehouses (tenant_id);
CREATE INDEX item_warehouses_warehouse_idx ON item_warehouses (warehouse_id);

ALTER TABLE item_warehouses ENABLE ROW LEVEL SECURITY;
ALTER TABLE item_warehouses FORCE ROW LEVEL SECURITY;
CREATE POLICY item_warehouses_tenant_isolation ON item_warehouses
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
