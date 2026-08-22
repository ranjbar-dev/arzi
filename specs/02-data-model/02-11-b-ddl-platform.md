_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 11.2 Platform: tenants, fiscal years, organisation, settings, users

```sql
-- =====================================================================
-- 11.2  Platform
-- Legacy: Base, Base_Config, Tanzim, PassWord, Pass_Config, arzi.ini
-- NEW: tenants — the legacy has no tenancy model at all (§1.4); this table and the
-- tenant_id column it anchors are the mechanism behind 11-open-decisions.md A3.
-- =====================================================================

-- ---------------------------------------------------------------------
-- tenants   -- [NEW] no legacy equivalent. One row per client who buys the product.
-- Declared first: everything else in the schema hangs off this.
-- ---------------------------------------------------------------------
CREATE TABLE tenants (
    id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug         text        NOT NULL,             -- URL-safe identifier, e.g. subdomain
    name         text        NOT NULL,              -- the client's display name (their organization.name is separate — that's letterhead)
    is_active    boolean     NOT NULL DEFAULT true, -- suspend a client (non-payment) without deleting their data
    plan         text        NOT NULL DEFAULT 'standard',   -- entitlement tier, see 08-14 proposal E3
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz,
    CONSTRAINT tenants_slug_key      UNIQUE (slug),
    CONSTRAINT tenants_slug_nonblank CHECK (length(btrim(slug)) > 0)
);

COMMENT ON TABLE tenants IS
  'The isolation boundary for the whole product (11-open-decisions.md A3). Every other table '
  'either carries tenant_id directly or is one of the two intentional exceptions: journal_sources '
  '(a fixed lookup describing the product, not client data) and permissions (the fixed permission '
  'catalogue — grants in user_permissions are tenant-scoped via the user).';


-- ---------------------------------------------------------------------
-- users   <- legacy PassWord  (08-platform-and-security.md §3.1)
-- Declared second because every audit column references it.
-- ---------------------------------------------------------------------
CREATE TABLE users (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- legacy UserCode (manually assigned, MAX+1, racy)
    tenant_id     bigint      NOT NULL REFERENCES tenants(id),      -- [NEW] §A3 — legacy has no tenancy at all
    username      text        NOT NULL,                             -- legacy UserName varchar(20)
    password_hash text        NOT NULL,                             -- legacy Password varchar(20) PLAINTEXT (§13.17)
    is_active     boolean     NOT NULL DEFAULT true,                -- legacy Enabled int 0/1
    is_superuser  boolean     NOT NULL DEFAULT false,               -- legacy Supervisor int 0/1 — bypasses the whole matrix, WITHIN its tenant only
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz,
    created_by    bigint,
    updated_by    bigint,
    CONSTRAINT users_username_key      UNIQUE (tenant_id, username),            -- changed from a global UNIQUE — every client can have their own "admin"
    CONSTRAINT users_username_nonblank CHECK (length(btrim(username)) > 0),     -- [NEW]
    CONSTRAINT users_username_len      CHECK (length(username) <= 20)           -- [AS-IS] Admin.pas:170
);
ALTER TABLE users ADD CONSTRAINT users_created_by_fkey FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE users ADD CONSTRAINT users_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES users(id);

CREATE INDEX users_tenant_idx ON users (tenant_id);

ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;
CREATE POLICY users_tenant_isolation ON users
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE  users IS 'Legacy PassWord. Tenant-scoped (§A3) — a username is unique per client, not globally.';
COMMENT ON COLUMN users.password_hash IS
  'Legacy stored the password in PLAINTEXT (GetPassu.pas:84). Argon2id here — §13.17. '
  'Migration cannot convert; a forced reset on first login is required.';

-- Legacy → Rust: GetPassu.pas login flow → auth::login(); ChangePasswordU.pas → auth::change_password().
-- No stored procedure existed for authentication.
-- login() resolves tenant_id from the request (subdomain/slug), THEN looks up username within
-- that tenant, THEN verifies the password — never the reverse, so a login attempt never even
-- queries across tenants.


-- ---------------------------------------------------------------------
-- user_permissions   <- legacy Pass_Config  (08-platform §4.2)
-- ---------------------------------------------------------------------
-- permissions is the one deliberately cross-tenant catalogue (§11.2 tenants comment): the set of
-- things the PRODUCT can gate is the same for every client. What a given client's users HOLD
-- (user_permissions) is tenant-scoped transitively through user_id, which already carries tenant_id.
CREATE TABLE permissions (
    id       integer PRIMARY KEY,          -- legacy P_ID, observed range 1100–2125 (§12.9)
    code     text    NOT NULL UNIQUE,
    label_fa text    NOT NULL              -- legacy P_DESC, denormalised into every grant row
);

CREATE TABLE user_permissions (
    tenant_id     bigint      NOT NULL REFERENCES tenants(id),                   -- [NEW] denormalised from users, see §11.0 convention 2
    user_id       bigint      NOT NULL REFERENCES users(id) ON DELETE CASCADE,   -- legacy P_User
    permission_id integer     NOT NULL REFERENCES permissions(id),               -- legacy P_ID
    granted_at    timestamptz NOT NULL DEFAULT now(),
    granted_by    bigint      REFERENCES users(id),
    PRIMARY KEY (user_id, permission_id)                                          -- [NEW] legacy had no key at all
);

CREATE INDEX user_permissions_tenant_idx ON user_permissions (tenant_id);

ALTER TABLE user_permissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_permissions FORCE ROW LEVEL SECURITY;
CREATE POLICY user_permissions_tenant_isolation ON user_permissions
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE user_permissions IS
  'Legacy Pass_Config. Legacy save was delete-all-then-reinsert with NO transaction (Admin.pas:192-214) '
  'and the runtime check was one round-trip per permission (Dmu.pas:1552). §13.18. Resolves 08-14 '
  'proposal B7 ("decide whether permissions are tenant-scoped"): yes, via this table; the permission '
  'catalogue itself (which actions exist) stays global.';

-- Legacy → Rust: TDM.IsEnabel (Dmu.pas:1552) → authz::has_permission(), loaded once per session
-- and enforced SERVER-SIDE (the legacy check was presentation-only — 08-platform §4.2).


-- ---------------------------------------------------------------------
-- fiscal_years   <- legacy Base (year columns only)   §1.4, §2.3, §8.3
-- ---------------------------------------------------------------------
CREATE TABLE fiscal_years (
    id                    bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id             bigint      NOT NULL REFERENCES tenants(id),  -- [NEW] §A3
    year                  integer     NOT NULL,          -- legacy CO_ID, e.g. 1403 (Jalali year)
    start_date            date        NOT NULL,          -- legacy FromDate char(10) Jalali
    end_date              date        NOT NULL,          -- legacy ToDate   char(10) Jalali
    is_active             boolean     NOT NULL DEFAULT true,   -- legacy IsActive int; 1 = open, else blocks all posting
    -- Gapless per-tenant-per-year counters, replacing every SELECT MAX(...)+1 (§5.3, §5.7).
    next_voucher_number   integer     NOT NULL DEFAULT 1,
    next_invoice_number   integer     NOT NULL DEFAULT 1,
    -- Year-end close bookkeeping, making the close idempotent (§10.8).
    closed_at             timestamptz,
    closing_voucher_id    bigint,      -- FK added after vouchers exists
    opening_voucher_id    bigint,
    -- MIGRATION ONLY — the original Jalali strings, kept for audit (§6.8 rule 4, §12.1)
    legacy_start_date_jalali text,
    legacy_end_date_jalali   text,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz,
    created_by            bigint      REFERENCES users(id),
    updated_by            bigint      REFERENCES users(id),
    CONSTRAINT fiscal_years_year_key    UNIQUE (tenant_id, year),                  -- changed from a global UNIQUE — every client has their own 1403
    CONSTRAINT fiscal_years_year_range  CHECK (year BETWEEN 1300 AND 1600),        -- [NEW] sanity only
    CONSTRAINT fiscal_years_date_order  CHECK (end_date > start_date),             -- [NEW] §6.8, §10.8
    CONSTRAINT fiscal_years_no_overlap
        EXCLUDE USING gist (tenant_id WITH =, daterange(start_date, end_date, '[]') WITH &&)  -- changed to per-tenant — periods only can't overlap within the same client
);

CREATE INDEX fiscal_years_tenant_idx ON fiscal_years (tenant_id, is_active) WHERE is_active;

ALTER TABLE fiscal_years ENABLE ROW LEVEL SECURITY;
ALTER TABLE fiscal_years FORCE ROW LEVEL SECURITY;
CREATE POLICY fiscal_years_tenant_isolation ON fiscal_years
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE fiscal_years IS
  'Legacy Base. CO_ID is a FISCAL YEAR id, not a company/tenant id (§1.4) — that fact is unchanged. '
  'tenant_id is a separate, new scoping dimension layered on top (§A3): one physical database, '
  'years separated by tenant_id + the *_COID-derived fiscal_year_id stamp on transactional tables.';
COMMENT ON COLUMN fiscal_years.is_active IS
  'Legacy IsActive. NO SCREEN WRITES THIS COLUMN (§2.3, §12.15) yet Dmu.pas:1008-1014 blocks all '
  'posting when it is not 1. Archiving is presumably done by hand in SSMS today.';

-- Legacy → Rust: TDM.Is_New_Sanad_Valid (Dmu.pas:1008) → fiscal_year::assert_open();
--                MakeNewU.pas (§10.4) → POST /api/v1/fiscal-years;
--                TDM.New_Sanad / New_AnbarFactor (Dmu.pas:1247,1258) → the counters above (§5.7).


-- ---------------------------------------------------------------------
-- organization   <- legacy Base (letterhead columns)   §2.3, §8.3
-- SINGLE ROW.  This is §13.9 — the legacy schema allows the letterhead to differ per year.
-- ---------------------------------------------------------------------
CREATE TABLE organization (
    id                  bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id           bigint      NOT NULL REFERENCES tenants(id),  -- [NEW] §A3 — one letterhead per CLIENT, not one for the whole product
    name                text        NOT NULL,       -- legacy Co_Name    varchar(100)
    subtitle            text,                       -- legacy Co_Sub     varchar(100)  نام سيستم
    address             text,                       -- legacy Co_Address varchar(100)
    phone               text,                       -- legacy Co_Tel     varchar(20)
    fax                 text,                       -- legacy Co_Fax     varchar(20)
    website             text,                       -- legacy Co_Web     varchar(30)
    email               text,                       -- legacy Co_EMail   varchar(30)
    registration_number text,                       -- legacy Co_Sabt    شماره ثبت
    national_id         text,                       -- legacy Co_Melli   شناسه ملی
    economic_code       text,                       -- legacy Co_Egh     varchar(20) کد اقتصادی
    postal_code         text,                       -- legacy Co_Post    varchar(20)
    logo                bytea,                      -- legacy ARM image — excluded from the ABS backup (Backup_U.pas:116)
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz,
    created_by          bigint      REFERENCES users(id),
    updated_by          bigint      REFERENCES users(id),
    CONSTRAINT organization_one_per_tenant UNIQUE (tenant_id)   -- changed from a product-wide singleton (§13.9) to one row per client
);

CREATE INDEX organization_tenant_idx ON organization (tenant_id);

ALTER TABLE organization ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization FORCE ROW LEVEL SECURITY;
CREATE POLICY organization_tenant_isolation ON organization
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE organization IS
  'Legacy Base letterhead columns, lifted out of the fiscal-year row (§13.9 — still applies WITHIN '
  'a tenant; run the probe in §12.15 item 4 before adopting per-client). A literal product-wide '
  'singleton was never viable once the product is sold to more than one client (§A3); this is now '
  'one row per tenant instead.';

-- If §13.9 is REJECTED at the per-tenant level, drop organization_one_per_tenant, add
--   fiscal_year_id bigint NOT NULL UNIQUE REFERENCES fiscal_years(id)
-- and keep one letterhead per (tenant, year) exactly as the legacy did per year.


-- ---------------------------------------------------------------------
-- account_code_format   <- legacy Base.No_Ko / No_Mo / No_Ta1 / No_Ta2
-- Global, because the chart of accounts is global (§1.4, §8.6).
-- ---------------------------------------------------------------------
CREATE TABLE account_code_format (
    tenant_id  bigint   NOT NULL REFERENCES tenants(id),  -- [NEW] §A3 — different clients may want different code widths
    level      smallint NOT NULL,          -- 1 = Kol, 2 = Moein, 3 = Tafsil1, 4 = Tafsil2
    width      smallint NOT NULL,             -- legacy No_Ko / No_Mo / No_Ta1 / No_Ta2
    label_fa   text     NOT NULL,
    PRIMARY KEY (tenant_id, level),                                                  -- changed from a global PK on level alone
    CONSTRAINT account_code_format_level_range CHECK (level BETWEEN 1 AND 4),        -- [NEW]
    CONSTRAINT account_code_format_width_range CHECK (width BETWEEN 1 AND 9)         -- [NEW] §8.6
);

ALTER TABLE account_code_format ENABLE ROW LEVEL SECURITY;
ALTER TABLE account_code_format FORCE ROW LEVEL SECURITY;
CREATE POLICY account_code_format_tenant_isolation ON account_code_format
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Seeded per tenant at onboarding time (defaults from Dmu.pas:1200-1222), not a fixed global row set.
-- Example for one tenant:
-- INSERT INTO account_code_format (tenant_id, level, width, label_fa) VALUES
--   (1, 1, 3, 'کل'), (1, 2, 2, 'معین'), (1, 3, 3, 'تفصیلی ۱'), (1, 4, 3, 'تفصیلی ۲');
-- The real widths for the FIRST migrated client must come from their Base row (§12.15 item 4).

-- Legacy → Rust: Dmu.pas:1196-1226 (code formatting) → accounts::format_code().


-- ---------------------------------------------------------------------
-- app_settings   <- legacy Tanzim (ids 1001–1015)   §8.2, §8.6
-- ---------------------------------------------------------------------
CREATE TABLE app_settings (
    tenant_id    bigint             NOT NULL REFERENCES tenants(id),  -- [NEW] §A3 — every client sets their own invoice text
    key          text               NOT NULL,           -- legacy T_ID, now readable
    value        text               NOT NULL,           -- legacy T_Str (always a string, even for booleans)
    value_type   setting_value_type NOT NULL,           -- [NEW] legacy had no type at all
    label_fa     text               NOT NULL,           -- legacy T_Desc
    legacy_id    integer,                                -- MIGRATION ONLY — the old T_ID, unique per migrated tenant
    updated_at   timestamptz        NOT NULL DEFAULT now(),
    updated_by   bigint             REFERENCES users(id),
    PRIMARY KEY (tenant_id, key)                                       -- changed from a global PK on key alone
);
-- Legacy T_Int is dropped: written as the string '0' on creation and never read (§8.2).

ALTER TABLE app_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE app_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY app_settings_tenant_isolation ON app_settings
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Seeded per tenant at onboarding (these 15 keys, one INSERT per key, tenant_id bound to the new
-- client's id) — not a fixed set of global rows. Keys and defaults unchanged from the legacy Tanzim
-- catalogue:
--   invoice.signature_1..4, invoice.heading_1..2, invoice.counterparty_label,
--   invoice.show_amount/show_discount/show_tax, voucher.signature_1..4, invoice.official_footer
-- (legacy_id 1001-1015 respectively).

COMMENT ON TABLE app_settings IS
  'Legacy Tanzim. Seeded by migration per tenant, NOT lazily created on read — the legacy getter '
  'mutated on read and seeded the Persian LABEL as the VALUE, so unconfigured signature lines '
  'printed the literal text "فاکتور امضا 1" on real customer documents (§8.2 defect 1).';

-- Legacy → Rust: TDM.Get_paramstr / Set_paramstr (Dmu.pas:468-508) → settings::get::<T>() / set(),
-- where set() is INSERT … ON CONFLICT DO UPDATE — killing the legacy silent no-op (§8.2 defect 2).


-- ---------------------------------------------------------------------
-- system_accounts   <- legacy Base_Config + Base.C1081/C1082   §2.4, §8.3, §13.10
-- FK to accounts(id) is added in §11.3 (accounts is declared there).
-- ---------------------------------------------------------------------
CREATE TABLE system_accounts (
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),  -- [NEW] §A3 — each client's own cash/notes accounts
    role        system_account_role NOT NULL,   -- legacy BC_ID / the C1081-C1082 column pair
    account_id  bigint      NOT NULL,              -- legacy BC_SSN / C1081 / C1082 → Sarfasl.S_SSN
    label_fa    text,                              -- legacy BC_Name
    is_enabled  boolean     NOT NULL DEFAULT true, -- legacy BC_Enabled
    updated_at  timestamptz NOT NULL DEFAULT now(),
    updated_by  bigint      REFERENCES users(id),
    PRIMARY KEY (tenant_id, role)                                                  -- changed from a global PK on role alone
);
-- Legacy BC_Default is dropped: with (tenant_id, role) as the PRIMARY KEY a role cannot have
-- several candidates within one tenant, so "which one is default" no longer arises. If §12.9 shows
-- a role legitimately needs multiple accounts, revert to (tenant_id, role, account_id) PK and
-- restore is_default.

ALTER TABLE system_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE system_accounts FORCE ROW LEVEL SECURITY;
CREATE POLICY system_accounts_tenant_isolation ON system_accounts
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE system_accounts IS
  'The ONLY settings in the system that change accounting behaviour (§8.3). Every change must be '
  'audited (§13.19) — the legacy system records nothing. Tenant-scoped per §A3: each client points '
  'these roles at accounts in THEIR OWN chart of accounts.';


-- ---------------------------------------------------------------------
-- user_preferences   <- legacy arzi.ini  (§8.1, §8.6)
-- NEW TABLE — the legacy stored these per WORKSTATION in a file the app wrote.
-- ---------------------------------------------------------------------
CREATE TABLE user_preferences (
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),                     -- [NEW] denormalised from users, see §11.0 convention 2
    user_id     bigint      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope       text        NOT NULL,        -- legacy ini section, e.g. 'Sarfasl_Select', 'AnbarReport_F'
    key         text        NOT NULL,        -- legacy ini key,     e.g. 'MRL', 'GridFontSize'
    value       jsonb       NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, scope, key)
);

ALTER TABLE user_preferences ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_preferences FORCE ROW LEVEL SECURITY;
CREATE POLICY user_preferences_tenant_isolation ON user_preferences
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE user_preferences IS
  'Replaces the ini file (§8.6). Window geometry is dropped — meaningless in a browser. '
  'CS1/CS2/CS3 (connection strings and licence) are NOT migrated: secrets come from the '
  'environment and the application has no code path that persists them (§8.6 rule 1).';


-- ---------------------------------------------------------------------
-- settings_audit_log   -- [NEW] §13.19
-- ---------------------------------------------------------------------
CREATE TABLE settings_audit_log (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),  -- [NEW] §A3
    table_name  text        NOT NULL,
    record_key  text        NOT NULL,
    column_name text        NOT NULL,
    old_value   text,
    new_value   text,
    changed_at  timestamptz NOT NULL DEFAULT now(),
    changed_by  bigint      REFERENCES users(id)
);

CREATE INDEX settings_audit_log_tenant_idx ON settings_audit_log (tenant_id, changed_at DESC);

ALTER TABLE settings_audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE settings_audit_log FORCE ROW LEVEL SECURITY;
CREATE POLICY settings_audit_log_tenant_isolation ON settings_audit_log
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- Append-only. Covers system_accounts, warehouses.*_account_id, warehouses.default_tax_rate,
-- account_code_format and app_settings (§8.6 rule 4).
```

---


---

[← 02-11-a-ddl-overview-and-extensions.md](02-11-a-ddl-overview-and-extensions.md) | [02-11-c-ddl-parties-and-accounts.md →](02-11-c-ddl-parties-and-accounts.md)
