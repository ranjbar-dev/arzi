_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 11. Proposed PostgreSQL DDL

### 11.0 How to read this section

This is a **proposal**, not a transcription. §2.0 states that no table DDL exists in this
repository; every type, length, nullability, default and identity property below is either inferred
from Delphi field metadata or newly chosen. **Nothing here can be applied until the artefacts in
§12.17 have been dumped from a live database.**

**Constraint novelty is marked on every constraint**, because the default position is port-as-is
and adding a constraint the legacy system lacks is a behaviour change requiring approval (§13):

| Marker | Meaning |
|---|---|
| `-- [AS-IS]` | reproduces a rule the legacy system already enforces somewhere (client, procedure, or schema) |
| `-- [NEW]` | **not enforced by the legacy system.** Needs review. Cross-referenced to §13 where a decision item exists. |
| `-- [VERIFY]` | may already exist in the legacy schema; §12.6 will say. If it does, it is `[AS-IS]`. |

Every `[NEW]` constraint is written inline so it can be commented out in one edit if rejected.

#### Conventions applied throughout

- `snake_case`, plural tables, singular columns (`01-glossary.md` §7).
- Surrogate keys: `id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY`, replacing
  `int IDENTITY(1,1)` `*_SSN` (§5.7). Widened to `bigint` deliberately — there is no reason to
  inherit a 2.1-billion ceiling. `GENERATED ALWAYS` so the application cannot supply one by
  accident, which also removes the `@@IDENTITY` hazard (§5.2, §12.7).
- Text: `text` everywhere, with a `CHECK (length(col) <= n)` **only** where the legacy length is
  load-bearing (e.g. a printed field). PostgreSQL `varchar(n)` buys nothing over `text`.
- Booleans: real `boolean`, replacing the legacy `int` 0/1 flags.
- Enumerations: PostgreSQL `ENUM` types where the value set is closed and known; `smallint` with a
  `CHECK` where §12.9 has not yet confirmed the full set. **Where the set is unconfirmed the DDL
  below deliberately uses `smallint` — do not narrow it to an enum before §12.9 is answered.**

#### Tenant scoping (`tenant_id`) — supersedes §13.10-era "stays global" language

**Superseded note:** an earlier draft of this section said `accounts`, `parties`, `items`,
`warehouses`, `units_of_measure`, `product_grades`, `users` and `app_settings` "stay global" and
that scoping is "not implemented with... row-level security." That was written before
`11-open-decisions.md` A3 was decided. **A3's ruling stands and this section now implements it:**
real multi-tenant, shared database, `tenant_id` on every table, enforced with row-level security.
The product is sold to multiple clients who must never see each other's data — there is no such
thing as "global" data in a sellable multi-tenant panel; even the chart of accounts and the party
register are per-client.

1. **`tenants` is the first table declared** (§11.2), with no dependencies. Every other table in
   this document — with the sole exception of the cross-tenant `journal_sources` lookup and the
   global `permissions` catalogue, which describe the *product*, not a client's data — carries
   `tenant_id bigint NOT NULL REFERENCES tenants(id)`.
2. **`tenant_id` is a direct column on every scoped table, not derived through a join.** RLS
   policies that have to join out to another table to find the tenant are slow and easy to get
   wrong; a flat column keeps every policy a single equality check and keeps `tenant_id` available
   to lead every composite index. This applies even to tables that also carry `fiscal_year_id` —
   the two are orthogonal scoping dimensions (client, then year-within-client) and both are
   denormalised onto the row.
3. **Row-level security is mandatory, not optional**, on every tenant-scoped table:
   ```sql
   ALTER TABLE <table> ENABLE ROW LEVEL SECURITY;
   ALTER TABLE <table> FORCE ROW LEVEL SECURITY;   -- table owners bypass RLS by default; FORCE closes that hole
   CREATE POLICY <table>_tenant_isolation ON <table>
       USING (tenant_id = current_setting('app.tenant_id')::bigint)
       WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
   ```
   The application's Postgres role must **not** own these tables and must **not** have `BYPASSRLS`.
   Migrations run as a separate owner role. The Rust transaction helper (`10-target-architecture.md`
   §2.4) issues `SET LOCAL app.tenant_id = $1` as the first statement of every transaction, with the
   value taken from the authenticated session server-side — never from a client-supplied field. This
   is what stops a defect like §B3 (`UPDATE ... Anbar_FactorD` with no `WHERE`, `Anbar_Amalkard.pas:168`)
   from being able to touch a second client's rows even if it ships again: the missing `WHERE` only
   ever sees the one tenant the session is scoped to.
4. `fiscal_years` is a real table with its own surrogate `id`. The legacy `CO_ID` value (`1403`,
   `1397`, …) becomes `fiscal_years.year`, unique **within a tenant** (`UNIQUE (tenant_id, year)`,
   not globally unique — two clients both have a year 1403), and is what the UI shows.
5. Every table that carried `*_COID` gets
   `fiscal_year_id bigint NOT NULL REFERENCES fiscal_years(id)`, in addition to its own
   `tenant_id`. The inconsistent legacy casing (`M_COID`, `DM_Coid`, `S_CoID`, §2.1) disappears.
   Composite indexes lead with `tenant_id`, then `fiscal_year_id`.
6. Document numbers are unique **within a tenant's fiscal year**, never globally:
   `UNIQUE (tenant_id, fiscal_year_id, voucher_number)` etc. — see §5.7 and §13.2.
7. `fiscal_years` also holds the **allocation counters** (§5.7), replacing every
   `SELECT MAX(...) + 1`. Counters are per-tenant-per-year by construction, since they live on the
   per-tenant `fiscal_years` row.

#### Jalali dates — the decision, and why

**Business dates are stored as PostgreSQL `date` (Gregorian). Jalali is derived at the edge.** This
is exactly the model already committed to in §6.8 and this section does not deviate from it.

Rationale, restated because it is the highest-risk decision in the migration:

- The legacy columns are Jalali **strings** compared **lexicographically** (§6.5) — there is no
  date arithmetic anywhere in the system, only string comparison, which is why the truncation
  hazards in §3.4 (a `varchar(8)` parameter turning `'1403/05/27'` into `'1403/05/'`) silently
  produce wrong report boundaries.
- Storing `date` makes ordering, `BETWEEN`, `age()`, and index range scans correct by construction
  and deletes that entire class of bug.
- Jalali is a **presentation format**, not a data type. The backend serialises both `date` (ISO)
  and `dateJalali` (`YYYY/MM/DD`) and accepts either on input (§6.8).

**The blocking caveat, narrowed 2026-08-19.** A schema-only SQL Server dump
(`Full_Script_14050527.sql`) has since supplied `XNew`'s body (§12.1): it delegates its one date
computation to a **third** Jalali algorithm, `dbo.Farsi_Date`, not previously known. That function
shares its naive-leap-year design with the dead `TUtil.FarsiDate` but is not byte-for-byte
identical (a 2-day epoch offset). The dump has **zero data rows**, so which algorithm actually
produced the historical strings is still unverified — `Tools.TFullDate` remains binary-only and
unobtainable, and even `dbo.Farsi_Date`'s output hasn't been checked against real stored values yet.
Therefore:

- Every business-date column below is accompanied by a **shadow column**
  `legacy_<name>_jalali text` (§6.8 migration rule 4), populated verbatim from the legacy string,
  kept for the first release so discrepancies are auditable, then dropped.
- The shadow columns are marked `-- MIGRATION ONLY` and must **not** be read by application code.
- The `date` columns cannot be populated until §12.1 identifies the correct conversion.

**The rejected alternative** — keeping `char(10)` Jalali columns and reproducing the string
comparison bug-for-bug — is recorded as §13.20, in case the business prefers exact behavioural
fidelity over correctness.

#### Money and quantities

Per §7.7, without deviation:

- Money: **`bigint`, whole rial**, `NOT NULL DEFAULT 0`. Not `numeric`, not `money`, never float.
- Quantities: `numeric(18,3)` for ledger and stock quantities (mirroring the legacy
  `TBCDField p18 s3`, §7.3), `numeric(18,2)` for pistachio weights.
- Unit prices: `bigint` (rial per unit), matching `AJ_Phi`.
- Percentages: `numeric(5,2)`, not float.
- Debit/credit stay **two non-negative columns** — a signed single-column model would change every
  report (§7.7).
- No currency column: single currency, IRR (§7.6).

#### Audit columns

Applied to every table, per §6.8 and §13.19:

```sql
created_at  timestamptz NOT NULL DEFAULT now(),
updated_at  timestamptz,
created_by  bigint REFERENCES users(id),
updated_by  bigint REFERENCES users(id)
```

Three notes carried from the as-is spec:

1. The legacy `M_Time`/`DM_CDate` were written with T-SQL `GetDate()` — a **naked local**
   `datetime`. Here they are `timestamptz` (§6.8: "always `timestamptz`, never naked
   `timestamp`"), with the application timezone `Asia/Tehran` in config.
2. Most treasury tables have **no timestamp at all** and only a `*_UserID` that is overwritten on
   every edit, so it is really "last editor" (`06-treasury.md` §1.1). Migrating that value into
   `created_by` is therefore **approximate**; the migration should copy it to `created_by` *and*
   `updated_by` and record that both are approximate.
3. `created_by`/`created_at` are `NULL`-able only for legacy rows. New rows always carry them —
   `created_by` in particular replaces the hard-coded `M_User = 68` of the carry-forward routine
   (`EnteghalU.pas:254`, §10.5 defect 3).

---

### 11.1 Extensions, enums and shared domains

```sql
-- =====================================================================
-- 11.1  Extensions, enumerated types, shared domains
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS btree_gist;   -- needed by the fiscal-year overlap EXCLUDE (§10.8)

-- Voucher / voucher-line workflow state.  Legacy M_Tx / DM_Tx (§9.7).
--   0 = draft (موقت), 1 = confirmed (تأیید شده), 2 = posted (ثبت قطعی)
-- RESOLVED 2026-08-19: Taraz_6Sotooni's and Taraz4Setooni's bodies (captured in a schema-only
--          dump) show @Sabt=3 means "M_Tx>0" (i.e. either 1 or 2) inside the query's WHERE clause
--          — it is a report-filter sentinel, not a claim that the M_Tx column itself ever holds
--          the value 3. This enum does not need a fourth value on that account. Whether M_Tx=3
--          exists in the data for some other reason is untested (the dump has no rows) but no
--          longer suspected from this angle.
CREATE TYPE voucher_status AS ENUM ('draft', 'confirmed', 'posted');

-- Legacy M_Kind / DM_Kind (§2.7, §2.8).  Only 1 and 2 are ever written.
--   1 = Moein / ledger, 2 = Rooznameh / daybook
CREATE TYPE journal_kind AS ENUM ('ledger', 'daybook');

-- Legacy Sahamdar.S_Kind (§2.6).  Only 1 is ever written (SahamdarEditU.pas:290).
CREATE TYPE party_type AS ENUM ('natural_person', 'legal_entity');

-- Legacy DCheck.S_State (06-treasury.md §2.1).
-- PORT-AS-IS NOTE: legacy code 1 means BOTH 'in_hand' and 'bounced' — it is overloaded, and
-- code 3 (the value the source comment reserves for "bounced") is never written by any path.
-- Splitting them is §13.12.  If §13.12 is REJECTED, replace this enum with
--   status smallint NOT NULL CHECK (status IN (1,2,3,4,5))
-- and keep the ambiguity.
CREATE TYPE cheque_status AS ENUM (
    'in_hand',              -- 1  چک موعدي در صندوق
    'at_bank',              -- 2  چک موعدی در بانک
    'bounced',              -- 1 (overloaded) / 3 (never written)  چک برگشت شده از بانک
    'returned_to_issuer',   -- 4  چک مسترد شد
    'cleared'               -- 5  چک وصول شد
);

-- Legacy Base_Config.BC_ID + Base.C1081/C1082 (§2.4, §8.3), unified per §13.10.
-- [VERIFY] §12.9 — role 11 is unidentified and the full BC_ID set is unknown.
CREATE TYPE system_account_role AS ENUM (
    'cash',                  -- C1081  صندوق
    'cheques_in_transit',    -- C1082  جریان
    'notes_payable',         -- BC_ID 13  اسناد پرداختنی
    'notes_receivable',      -- BC_ID 14  اسناد دریافتنی
    'notes_in_collection'    -- BC_ID 15  اسناد در جریان وصول
);

-- Setting value types for app_settings (§8.6 rule 2).
CREATE TYPE setting_value_type AS ENUM ('string', 'boolean', 'integer', 'text_block', 'account_id');

-- Money.  Whole rial, always non-negative unless a column says otherwise (§7.7).
CREATE DOMAIN rial AS bigint;

-- Ledger / stock quantity.  Mirrors the legacy TBCDField p18 s3 (§7.3).
CREATE DOMAIN quantity AS numeric(18,3);

-- Percentage, exact.  Replaces the legacy AsFloat reads (§7.7).
CREATE DOMAIN percentage AS numeric(5,2) CHECK (VALUE >= 0 AND VALUE <= 100);
```

> **`Moein.M_ID` (source module) is deliberately NOT an enum.** §12.9 records that codes `15` and
> `35` are unidentified and the observed list is "certainly incomplete". It is modelled as
> `smallint` with a lookup table so unknown values migrate without loss. Narrow it later.

```sql
-- Lookup, not an enum, precisely because the value set is unconfirmed (§12.9).
CREATE TABLE journal_sources (
    id          smallint PRIMARY KEY,      -- legacy Moein.M_ID
    code        text     NOT NULL UNIQUE,
    label_fa    text     NOT NULL,
    label_en    text     NOT NULL,
    source_table text                      -- which table source_id points into, when known
);

INSERT INTO journal_sources (id, code, label_fa, label_en, source_table) VALUES
  ( 1, 'inventory_invoice',        'فاکتور انبار',            'Inventory invoice',        'inventory_invoices'),
  (21, 'cheque_received',          'چک دریافتی',              'Cheque received',          'cheques'),
  (22, 'cheque_bounced',           'برگشت چک از بانک',        'Cheque bounced',           'cheques'),
  (23, 'cheque_collected',         'وصول چک',                 'Cheque collected',         'cheques'),
  (24, 'cheque_returned',          'استرداد چک',              'Cheque returned to issuer','cheques'),
  (25, 'deposit_slip',             'فیش بانکی',               'Bank deposit slip',        'deposit_slips'),
  (26, 'cheque_payment_document',  'سند پرداخت چک',           'Cheque payment document',  'cheque_payment_documents'),
  (34, 'pistachio_receipt',        'خرید پسته',               'Pistachio purchase receipt', NULL),
  (41, 'petty_cash',               'تنخواه',                  'Petty cash',               'petty_cash_documents');
-- Codes 2–9, 15, 27–29, 35 exist in the data and are NOT yet identified (§12.9).
-- The migration must INSERT a placeholder row for every distinct M_ID found, or the FK fails.
```

---


---

[← 02-10-b-year-end-and-import.md](02-10-b-year-end-and-import.md) | [02-11-b-ddl-platform.md →](02-11-b-ddl-platform.md)
