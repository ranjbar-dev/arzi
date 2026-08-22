-- Step 1.1: tenants, users, permissions, user_permissions, fiscal_years,
-- organization, account_code_format, app_settings, system_accounts,
-- user_preferences, settings_audit_log — verbatim from
-- specs/02-data-model/02-11-b-ddl-platform.md §11.2, plus the two enum types
-- and extension it depends on (specs/02-data-model/02-11-a §11.1). Every
-- table gets ENABLE/FORCE ROW LEVEL SECURITY per specs/11-open-decisions.md A3.
--
-- Only the pieces §11.1 declares that this step's tables actually reference
-- are pulled in here (system_account_role, setting_value_type, btree_gist).
-- voucher_status/journal_kind/party_type/cheque_status and the rial/quantity/
-- percentage domains belong to later phases' tables and are deliberately
-- left for those steps' migrations.

DROP TABLE IF EXISTS _bootstrap_check;

CREATE EXTENSION IF NOT EXISTS btree_gist;   -- needed by fiscal_years_no_overlap

CREATE TYPE system_account_role AS ENUM (
    'cash',
    'cheques_in_transit',
    'notes_payable',
    'notes_receivable',
    'notes_in_collection'
);

CREATE TYPE setting_value_type AS ENUM ('string', 'boolean', 'integer', 'text_block', 'account_id');

-- ---------------------------------------------------------------------
-- tenants
-- ---------------------------------------------------------------------
CREATE TABLE tenants (
    id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug         text        NOT NULL,
    name         text        NOT NULL,
    is_active    boolean     NOT NULL DEFAULT true,
    plan         text        NOT NULL DEFAULT 'standard',
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
-- users
-- ---------------------------------------------------------------------
CREATE TABLE users (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id     bigint      NOT NULL REFERENCES tenants(id),
    username      text        NOT NULL,
    password_hash text        NOT NULL,
    is_active     boolean     NOT NULL DEFAULT true,
    is_superuser  boolean     NOT NULL DEFAULT false,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz,
    created_by    bigint,
    updated_by    bigint,
    CONSTRAINT users_username_key      UNIQUE (tenant_id, username),
    CONSTRAINT users_username_nonblank CHECK (length(btrim(username)) > 0),
    CONSTRAINT users_username_len      CHECK (length(username) <= 20)
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

-- ---------------------------------------------------------------------
-- permissions (global catalogue, deliberately NOT tenant-scoped) + user_permissions
-- ---------------------------------------------------------------------
CREATE TABLE permissions (
    id       integer PRIMARY KEY,
    code     text    NOT NULL UNIQUE,
    label_fa text    NOT NULL
);

CREATE TABLE user_permissions (
    tenant_id     bigint      NOT NULL REFERENCES tenants(id),
    user_id       bigint      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission_id integer     NOT NULL REFERENCES permissions(id),
    granted_at    timestamptz NOT NULL DEFAULT now(),
    granted_by    bigint      REFERENCES users(id),
    PRIMARY KEY (user_id, permission_id)
);

CREATE INDEX user_permissions_tenant_idx ON user_permissions (tenant_id);

ALTER TABLE user_permissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_permissions FORCE ROW LEVEL SECURITY;
CREATE POLICY user_permissions_tenant_isolation ON user_permissions
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE user_permissions IS
  'Legacy Pass_Config. Legacy save was delete-all-then-reinsert with NO transaction (Admin.pas:192-214) '
  'and the runtime check was one round-trip per permission (Dmu.pas:1552). §13.18.';

-- ---------------------------------------------------------------------
-- fiscal_years
-- ---------------------------------------------------------------------
CREATE TABLE fiscal_years (
    id                    bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id             bigint      NOT NULL REFERENCES tenants(id),
    year                  integer     NOT NULL,
    start_date            date        NOT NULL,
    end_date              date        NOT NULL,
    is_active             boolean     NOT NULL DEFAULT true,
    next_voucher_number   integer     NOT NULL DEFAULT 1,
    next_invoice_number   integer     NOT NULL DEFAULT 1,
    closed_at             timestamptz,
    closing_voucher_id    bigint,      -- FK added once vouchers exists (Phase 2)
    opening_voucher_id    bigint,
    legacy_start_date_jalali text,     -- MIGRATION ONLY
    legacy_end_date_jalali   text,     -- MIGRATION ONLY
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz,
    created_by            bigint      REFERENCES users(id),
    updated_by            bigint      REFERENCES users(id),
    CONSTRAINT fiscal_years_year_key    UNIQUE (tenant_id, year),
    CONSTRAINT fiscal_years_year_range  CHECK (year BETWEEN 1300 AND 1600),
    CONSTRAINT fiscal_years_date_order  CHECK (end_date > start_date),
    CONSTRAINT fiscal_years_no_overlap
        EXCLUDE USING gist (tenant_id WITH =, daterange(start_date, end_date, '[]') WITH &&)
);

CREATE INDEX fiscal_years_tenant_idx ON fiscal_years (tenant_id, is_active) WHERE is_active;

ALTER TABLE fiscal_years ENABLE ROW LEVEL SECURITY;
ALTER TABLE fiscal_years FORCE ROW LEVEL SECURITY;
CREATE POLICY fiscal_years_tenant_isolation ON fiscal_years
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE fiscal_years IS
  'Legacy Base. CO_ID is a FISCAL YEAR id, not a company/tenant id (§1.4) — that fact is unchanged. '
  'tenant_id is a separate, new scoping dimension layered on top (§A3).';
COMMENT ON COLUMN fiscal_years.is_active IS
  'Legacy IsActive. NO SCREEN WRITES THIS COLUMN in the legacy app (§2.3, §12.15) yet posting is '
  'blocked when it is not true. Step 1.5 adds the first explicit writer.';

-- ---------------------------------------------------------------------
-- organization (single row per tenant — the letterhead)
-- ---------------------------------------------------------------------
CREATE TABLE organization (
    id                  bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id           bigint      NOT NULL REFERENCES tenants(id),
    name                text        NOT NULL,
    subtitle            text,
    address             text,
    phone               text,
    fax                 text,
    website             text,
    email               text,
    registration_number text,
    national_id         text,
    economic_code       text,
    postal_code         text,
    logo                bytea,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz,
    created_by          bigint      REFERENCES users(id),
    updated_by          bigint      REFERENCES users(id),
    CONSTRAINT organization_one_per_tenant UNIQUE (tenant_id)
);

CREATE INDEX organization_tenant_idx ON organization (tenant_id);

ALTER TABLE organization ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization FORCE ROW LEVEL SECURITY;
CREATE POLICY organization_tenant_isolation ON organization
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- account_code_format
-- ---------------------------------------------------------------------
CREATE TABLE account_code_format (
    tenant_id  bigint   NOT NULL REFERENCES tenants(id),
    level      smallint NOT NULL,
    width      smallint NOT NULL,
    label_fa   text     NOT NULL,
    PRIMARY KEY (tenant_id, level),
    CONSTRAINT account_code_format_level_range CHECK (level BETWEEN 1 AND 4),
    CONSTRAINT account_code_format_width_range CHECK (width BETWEEN 1 AND 9)
);

ALTER TABLE account_code_format ENABLE ROW LEVEL SECURITY;
ALTER TABLE account_code_format FORCE ROW LEVEL SECURITY;
CREATE POLICY account_code_format_tenant_isolation ON account_code_format
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- app_settings
-- ---------------------------------------------------------------------
CREATE TABLE app_settings (
    tenant_id    bigint             NOT NULL REFERENCES tenants(id),
    key          text               NOT NULL,
    value        text               NOT NULL,
    value_type   setting_value_type NOT NULL,
    label_fa     text               NOT NULL,
    legacy_id    integer,           -- MIGRATION ONLY
    updated_at   timestamptz        NOT NULL DEFAULT now(),
    updated_by   bigint             REFERENCES users(id),
    PRIMARY KEY (tenant_id, key)
);

ALTER TABLE app_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE app_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY app_settings_tenant_isolation ON app_settings
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE app_settings IS
  'Legacy Tanzim. Seeded by migration per tenant, NOT lazily created on read — the legacy getter '
  'mutated on read and seeded the Persian LABEL as the VALUE (§8.2 defect 1).';

-- ---------------------------------------------------------------------
-- system_accounts
-- FK to accounts(id) added once accounts exists (§11.3 / Phase 2 step 2.1).
-- ---------------------------------------------------------------------
CREATE TABLE system_accounts (
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),
    role        system_account_role NOT NULL,
    account_id  bigint      NOT NULL,
    label_fa    text,
    is_enabled  boolean     NOT NULL DEFAULT true,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    updated_by  bigint      REFERENCES users(id),
    PRIMARY KEY (tenant_id, role)
);

ALTER TABLE system_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE system_accounts FORCE ROW LEVEL SECURITY;
CREATE POLICY system_accounts_tenant_isolation ON system_accounts
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

COMMENT ON TABLE system_accounts IS
  'The ONLY settings in the system that change accounting behaviour (§8.3). Every change must be '
  'audited (§13.19) — logged to settings_audit_log.';

-- ---------------------------------------------------------------------
-- user_preferences
-- ---------------------------------------------------------------------
CREATE TABLE user_preferences (
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),
    user_id     bigint      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope       text        NOT NULL,
    key         text        NOT NULL,
    value       jsonb       NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, scope, key)
);

ALTER TABLE user_preferences ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_preferences FORCE ROW LEVEL SECURITY;
CREATE POLICY user_preferences_tenant_isolation ON user_preferences
    USING (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);

-- ---------------------------------------------------------------------
-- settings_audit_log
-- ---------------------------------------------------------------------
CREATE TABLE settings_audit_log (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id   bigint      NOT NULL REFERENCES tenants(id),
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

-- =====================================================================
-- Permission catalogue seed — 08-04-authorization.md §4.4, ids 1100-2125.
-- 82 of the 85 legacy Pass_Config ids are seeded; skipped per the Build
-- bullet's "known-dead ones": 1129, 1130, 1415, 1416 (orphan checkboxes,
-- no call site) and the 2119 numbering gap (never allocated at all).
-- Dead-but-real ids (1102, 1103, 1108, 1411 — checked in code even though
-- the UI can't grant them or the handler is inert) ARE seeded: they are
-- still real Pass_Config rows, not orphans.
-- =====================================================================
INSERT INTO permissions (id, code, label_fa) VALUES
  (1100, 'chart_of_accounts',                          'سرفصلهاي حسابداري'),
  (1101, 'account_list',                                'ليست سرفصلها'),
  (1102, 'create_account',                              'ايجاد سرفصل'),
  (1103, 'amend_account',                               'اصلاح سرفصل'),
  (1104, 'delete_account',                              'حذف سرفصل'),
  (1105, 'personal_current_accounts',                   'جاري اشخاص'),
  (1106, 'add_current_account',                         'افزودن جاري جديد'),
  (1107, 'amend_current_account',                       'اصلاح جاري'),
  (1108, 'delete_current_account',                      'حذف جاري'),
  (1112, 'subsidiary_documents',                        'اسناد معین'),
  (1113, 'post_subsidiary_document',                    'ثبت سند معين'),
  (1114, 'amend_subsidiary_document',                   'اصلاح سند معين'),
  (1115, 'delete_subsidiary_document',                  'حذف سند معين'),
  (1116, 'approve_subsidiary_document',                 'تاييد سند معين'),
  (1117, 'post_subsidiary_document_permanently',        'ثبت دائم سند'),
  (1118, 'revert_subsidiary_document_to_draft',         'برگشت به تحرير'),
  (1119, 'change_subsidiary_document_date',             'تغيير تاريخ سند'),
  (1120, 'change_subsidiary_document_number',           'تغيير شماره سند'),
  (1121, 'print_subsidiary_document',                   'چاپ سند معين'),
  (1125, 'list_approved_subsidiary_documents',          'ليست اسناد تاييد شده'),
  (1126, 'list_posted_subsidiary_documents',            'ليست اسناد ثبت شده'),
  (1127, 'list_draft_subsidiary_documents',             'ليست اسناد درحال تحرير'),
  (1142, 'copy_subsidiary_document',                    'کپی سند معین'),
  (1143, 'merge_subsidiary_documents',                  'ادغام اسناد'),
  (1144, 'lock_subsidiary_document',                    'قفل سند'),
  (1145, 'revert_subsidiary_document_permanent_posting','برگشت از ثبت دائم'),
  (1132, 'journal_documents',                           'اسناد روزنامه'),
  (1133, 'post_journal_document',                       'ثبت سند روزنامه'),
  (1134, 'delete_journal_document',                     'حذف سند روزنامه'),
  (1135, 'change_journal_document_date',                'تغییر تاریخ سند روزنامه'),
  (1136, 'change_journal_document_number',              'تغییر شماره سند روزنامه'),
  (1137, 'change_journal_document_description',         'تغییر شرح سند روزنامه'),
  (1138, 'change_journal_document_status',              'تغییر وضعیت سند'),
  (1139, 'lock_journal_document',                        'قفل سند روزنامه'),
  (1140, 'print_journal_document',                      'چاپ سند روزنامه'),
  (1122, 'accounting_reports',                          'کزارش حسابداري'),
  (1123, 'view_subsidiary_ledger',                      'مشاهده دفتر معين'),
  (1124, 'trial_balance_4_column',                      'تراز آزمايشي 4 ستوني'),
  (1128, 'subsidiary_document_summary',                 'خلاصه اسناد معين'),
  (1131, 'card_summary',                                'خلاصه کارت'),
  (1141, 'general_ledger',                              'دفتر کل'),
  (1401, 'warehouse_settings',                          'تنظيمات انبار'),
  (1402, 'item_master',                                 'معرفي اجناس'),
  (1403, 'issue_invoice',                               'صدور فاکتور'),
  (1404, 'issue_purchase_invoice',                      'صدور فاکتور خريد'),
  (1405, 'issue_sales_invoice',                         'صدور فاکتور فروش'),
  (1406, 'sales_return_invoice',                        'فاکتور برگشت از فروش'),
  (1407, 'purchase_return_invoice',                     'فاکتور برگشت از خريد'),
  (1408, 'amend_invoice',                               'اصلاح فاکتور'),
  (1414, 'delete_invoice',                              'حذف فاکتور'),
  (1409, 'warehouse_report',                            'گزارش انبار'),
  (1410, 'print_invoice',                               'چاپ فاکتور'),
  (1411, 'warehouse_performance_report',                'گزارش عملکرد انبار'),
  (1412, 'stock_card',                                  'کارت جنسي'),
  (1413, 'inventory_balance_report',                    'گزارش موجودی انبار'),
  (2101, 'receivable_instrument_list',                  'لیست اسناد دریافتی'),
  (2102, 'receive_cheque',                              'دریافت چک'),
  (2103, 'amend_received_cheque',                       'اصلاح چک'),
  (2104, 'delete_received_cheque',                      'حذف چک'),
  (2105, 'deposit_cheque_to_bank',                      'واگذار به بانک'),
  (2106, 'return_cheque_from_bank',                     'برگشت از بانک'),
  (2107, 'collect_cheque',                              'وصول چک'),
  (2108, 'delete_cheque_collection',                    'حذف وصول چک'),
  (2109, 'return_cheque_to_owner',                      'برگشت به صاحب'),
  (2110, 'cheque_issuance_list',                        'لیست صدور چک'),
  (2111, 'issue_cheque',                                'صدور چک'),
  (2112, 'amend_issued_cheque',                         'اصلاح چک'),
  (2113, 'view_issued_cheque',                          'نمایش'),
  (2114, 'print_issued_cheque',                         'چاپ'),
  (2125, 'delete_issued_cheque',                        'حذف چک'),
  (2115, 'cash_deposit',                                'واریز نقدی'),
  (2116, 'new_deposit_slip',                            'فیش جدید'),
  (2117, 'amend_deposit_slip',                          'اصلاح فیش'),
  (2118, 'delete_deposit_slip',                         'حذف فیش'),
  (2120, 'petty_cash_float_list',                       'لیست تنخواه گردان'),
  (2121, 'new_petty_cash_list',                         'لیست جدید'),
  (2122, 'amend_petty_cash_list',                       'اصلاح لیست'),
  (2123, 'view_petty_cash_list',                        'نمایش'),
  (2124, 'print_petty_cash_list',                       'چاپ'),
  (1109, 'application_settings',                        'تنظيمات برنامه'),
  (1110, 'create_backup',                               'ايجاد پشتيبان'),
  (1111, 'change_skin',                                 'تغيير پوسته');
