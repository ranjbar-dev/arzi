# 10 — Target architecture: Rust + React + PostgreSQL + Docker

This document maps the specified system onto the target stack. It is a starting position, not a
mandate — where it makes a choice, it says why, so the choice can be overridden.

Two constraints govern everything here:

1. **Business logic is preserved exactly.** The legacy code is a logic reference. Its architecture,
   patterns and naming are not.
2. **Several architectural facts are still unresolved** — above all the tenancy model (see
   `11-open-decisions.md` A3). Where a decision depends on one of those, this document says so
   rather than assuming.

---

## 1. Shape of the system

The legacy application is a fat client: 126 forms talking directly to SQL Server, with business
rules split between event handlers and ~30 stored procedures. Every workstation holds database
credentials.

The replacement is conventional and boring on purpose:

```
Browser (React SPA)
      │  HTTPS, JSON
      ▼
Rust API server  ──────►  PostgreSQL
      │                        ▲
      └── background jobs ─────┘
```

No microservices. One deployable API binary, one database, one static frontend bundle. The domain
is a single accounting system with tightly coupled modules — vouchers, inventory and treasury all
post to the same ledger inside the same transaction. Splitting them across services would mean
distributed transactions over a problem that fits in one Postgres instance comfortably.

---

## 2. Backend

### 2.1 Crate choices

| Concern | Choice | Why |
|---|---|---|
| HTTP | `axum` | Tower ecosystem, straightforward extractors, widely used. |
| Async runtime | `tokio` | Required by the above. |
| Database | `sqlx` | Compile-time-checked SQL against a live schema. This system is SQL-heavy — dozens of reports are essentially one query each. An ORM would fight that; `sqlx` embraces it. |
| Migrations | `sqlx::migrate!` | Already present, no extra tool. |
| Money | `rust_decimal` | See §2.3. |
| Jalali dates | `ptime` or equivalent — **pending A1** | Cannot be chosen until the stored format is known. |
| Serialisation | `serde` with `rename_all = "camelCase"` | Rust stays snake_case; the API is camelCase. |
| Auth | `argon2` + JWT or server-side sessions | See §2.5. |
| Errors | `thiserror` in the domain, one `IntoResponse` at the boundary | Keeps HTTP concerns out of the logic. |
| Tests | built-in `#[tokio::test]` + `sqlx::test` | See §6. |

Deliberately **not** included: a DI framework, a generic repository trait layer, an event bus, a
CQRS split. None of them earn their keep in a single-database line-of-business system, and each one
adds a layer someone has to read at 3am.

### 2.2 Module layout

Mirrors the specification documents, so a question about the code has an obvious document and vice
versa:

```
src/
  main.rs           server wiring
  db.rs             pool, transaction helpers
  auth/             authentication, authorization, sessions
  accounting/       accounts, vouchers, period close      → docs 03
  inventory/        items, invoices, stock, costing       → docs 05
  treasury/         cheques, deposit slips, petty cash    → docs 06
  parties/          party register, current accounts      → docs 07
  reporting/        queries behind every report           → docs 04
  platform/         settings, fiscal years, audit log     → docs 08
  jalali.rs         calendar conversion (pending A1)
```

Each module: `mod.rs` (routes), `model.rs` (types), `logic.rs` (rules), `queries.rs` (SQL).
Cross-module calls go through the module's public functions, not its queries.

### 2.3 Money

The legacy system uses 64-bit integers for amounts in rials, with no decimal component and no
currency column — despite the product being named *arzi* ("foreign currency"), which appears to be
vestigial.

**Recommendation: keep integer rials** (`i64` in Rust, `bigint` in PostgreSQL). It matches the
existing data exactly, avoids rounding drift, and the domain has no sub-rial amounts. Use
`rust_decimal` only where a computation genuinely needs fractions — unit costs and the pistachio
deduction percentages — and round to whole rials at the persistence boundary, matching the legacy
rounding points documented in `05-inventory.md §6`.

Do **not** introduce a floating-point type anywhere in the money path.

### 2.4 Transactions

Every operation that posts to the ledger — issuing a voucher, confirming an invoice, clearing a
cheque — happens in **one** database transaction covering both the source document and its
generated voucher lines. The legacy system did not do this consistently, which is how several of
the Group B defects produce half-written state.

Take the transaction at the top of the request handler, pass `&mut Transaction` down, commit once.
No nested transaction helpers, no unit-of-work abstraction.

The same helper issues `SET LOCAL app.tenant_id = $1` as its first statement, with `$1` taken from
the authenticated session (never from a client-supplied header or body field) — this is what the
row-level-security policies in `02-data-model.md` §11 key off, so tenant isolation is enforced by
Postgres itself, not just by application code remembering to filter.

### 2.5 Authentication and authorization

Nothing from the legacy design transfers (see `08-platform-and-security.md`). The replacement:

- **Passwords**: Argon2id. No plaintext, ever. No username enumeration on the login screen.
- **Sessions**: server-side sessions in Postgres, or JWTs with short expiry plus a refresh token.
  Server-side sessions are simpler to revoke and this system needs revocation more than it needs
  statelessness.
- **Authorization**: enforced in the API handler, every time, before any query runs. The legacy
  permission matrix (~85 permissions, documented in `08-platform-and-security.md §4`) is the
  starting point for the permission set, but the *enforcement* is entirely new — presentation-level
  checks are additionally applied in React purely for UX, never as the control.
- **Credentials**: environment variables or a secret store. Never a file the client can read.
- **Audit trail**: an `audit_log` table written on every mutation. The legacy system has none;
  this is new and, in an accounting system, not optional.

### 2.6 What happens to the stored procedures

Roughly 30 procedures exist. Their bodies are not in the repository (blocking item A2). Once dumped,
the disposition is per-procedure:

- **Pure reporting queries** (the trial balances, stock cards, ledger views) → plain SQL in the
  relevant `queries.rs`, so the logic is visible and testable in Rust.
- **Procedures encoding business rules** (`Sarfasl_ADD`, `Anbar_AddToFactor`, `Sarfasl_Deep`) →
  reimplemented as Rust functions with tests, not ported as procedures. Rules belong where they can
  be unit-tested.
- **Numbering procedures** (`XNEW`, `B_SelectSerial`) → PostgreSQL sequences or
  `INSERT … RETURNING`, eliminating the race conditions documented in `02-data-model.md §5`.

No PL/pgSQL unless a genuine set-based operation demands it.

---

## 3. Frontend

### 3.1 Stack

| Concern | Choice | Why |
|---|---|---|
| Build | Vite | Fast, unremarkable. |
| Language | TypeScript | Non-negotiable in a system with this many field-level rules. |
| Routing | React Router | The app is a navigation tree of ~80 screens. |
| Server state | TanStack Query | Nearly all state in this app *is* server state. |
| Client state | React state / context | There is very little genuinely client-side state. Do not add Redux for it. |
| Forms | React Hook Form + Zod | Validation rules are numerous, specific, and must mirror the server exactly. |
| Tables | TanStack Table + virtualisation | Ledgers and stock cards return tens of thousands of rows. |
| i18n | `react-i18next` | See §3.3. |

### 3.2 Navigation

The legacy shell is a 6-tab ribbon of speed buttons with **zero keyboard shortcuts** — the complete
tree is extracted in `08-platform-and-security.md §1`. That tree is the navigation specification.

Two notes for the rebuild:

- Users of a data-entry system this dense will want keyboard navigation. The legacy app had none,
  so adding it is a change, not a port — but a cheap and welcome one. Flagged in `11-open-decisions.md`.
- 14 units are dead and 5 menu items are ungated. Do not port either without a ruling.

### 3.3 Persian, RTL and digits

This is a Persian application and the UI must remain Persian.

- **Direction**: `dir="rtl"` at the document root; use CSS logical properties (`margin-inline-start`,
  not `margin-left`) throughout so the layout is direction-agnostic.
- **Strings**: every user-visible string becomes a translation key. The Persian text captured
  verbatim in the specification documents is the initial `fa` locale content. **No Persian in
  identifiers, ever.**
- **Digits**: the legacy reports render Persian-Indic digits (`۰۱۲۳`). Presentation-layer formatting
  only — store and compute with ASCII digits.
- **Dates**: the UI shows Jalali dates; the database stores what A1 decides. Conversion happens in
  one place on each side, never inline.

### 3.4 Reports and printing

The legacy system embeds FastReport templates inline in `.dfm` files (which is why one form file is
5.9 MB). There is no equivalent, and none is needed.

- **On-screen**: virtualised tables, server-side pagination and aggregation. The server sends
  computed totals — never recompute a trial balance in the browser.
- **Print/PDF**: server-side rendering. The layouts are dense, RTL, and must be reproducible
  byte-for-byte for tax purposes; browser print CSS is not a reliable substrate for that.
- **Excel/CSV**: server-side generation (`rust_xlsxwriter`, `csv`). The legacy exports are
  documented column-by-column in `04-reporting.md §7`, including the tax-authority export whose
  draft-inclusion is an open question (A8).

---

## 4. Database

Full DDL is in `02-data-model.md §11`, with every constraint tagged `[AS-IS]`, `[NEW]` or
`[VERIFY]`. Points that are architecture rather than schema:

- **Fiscal-year scoping.** `COID` becomes `fiscal_year_id`, a real foreign key to `fiscal_years`.
  Master data (accounts, parties) stays global, matching current behaviour.
- **Tenancy.** Decided (A3): multi-tenant, shared database. `tenant_id` is on *every* table,
  including what used to be described as global master data (`accounts`, `parties`, `items`,
  `warehouses`, `users`, `app_settings`, `organization`), enforced with row-level security
  (`ENABLE`/`FORCE ROW LEVEL SECURITY` + a policy against `current_setting('app.tenant_id')` —
  full DDL in `02-data-model.md` §11). The application's Postgres role does not own these tables and
  does not have `BYPASSRLS`, so RLS cannot be silently bypassed by ordinary application code. This
  was the one decision that was expensive to defer — retrofitting tenancy is a schema-wide change —
  which is why it is applied from the first DDL draft rather than added later.
- **Dates.** Store a real `date` column, with a `legacy_*_jalali` shadow column preserving the
  original string for reconciliation during and after migration. The shadow columns can be dropped
  once the migration is proven.
- **Money.** `bigint` rials, as §2.3.
- **Audit columns.** `created_at`, `updated_at`, `created_by`, `updated_by` uniformly. The legacy
  system has these only sporadically and hard-codes the user in places.
- **Denormalised copies** (`StateName`, `BedName`, `BesName`, `DMoein` totals) are candidates for
  removal — see `11-open-decisions.md` C3. They currently drift.

### Migration

Not writable until A1 and A2 are answered. When it is:

1. Dump the live schema and procedure bodies.
2. Determine the Jalali storage format; validate the conversion against a full-table scan.
3. ETL into the new schema with `[NEW]` constraints **disabled**.
4. Run the data-integrity audit for the Group B defects — out-of-balance vouchers, orphaned
   invoices, duplicate stock lines, contradictory cheque history.
5. Remediate, then enable constraints one at a time.
6. Reconcile: every trial balance, ledger and stock balance must match the legacy system's output
   for the same parameters, or the difference must be explained by an approved Group B fix.

Step 6 is the acceptance test for the whole migration.

---

## 5. Docker

```
docker-compose.yml
  api        Rust binary, multi-stage build (cargo-chef → distroless)
  web        static bundle behind nginx (or served by the API)
  db         postgres:17, named volume
```

Multi-stage builds with `cargo-chef` for dependency-layer caching. Config strictly through
environment variables. Health checks on `api` and `db`. One `docker-compose.yml` for local
development; production overrides in a second file rather than a templating layer.

---

## 6. Testing

The legacy project has **zero automated tests** — and `test.dpr`, despite its name, is a licence-key
generator. Everything here is new.

- **Unit tests** for every business rule, particularly: voucher balancing, the period-close
  algorithm, stock quantity mathematics, costing, and the pistachio deduction formulas. These are
  the rules that must not drift; the specification documents give worked arithmetic to assert
  against.
- **Integration tests** against **SQLite or an in-memory database**, per your requirement. Note the
  constraint honestly: `sqlx` supports both, but any query using PostgreSQL-specific syntax will not
  run on SQLite. Two workable approaches:
  1. Keep the SQL portable in the modules that matter, and accept that a handful of
     PostgreSQL-specific reporting queries are tested only against Postgres.
  2. Use `sqlx::test`, which spins up a real disposable Postgres database per test — genuinely fast
     and removes the dialect problem entirely.

  Recommendation: **option 2 for the test suite, with SQLite kept for local development
  convenience** if that is what the requirement was aiming at. Say the word if you want option 1
  and portable SQL becomes a hard constraint on every query.
- **Reconciliation tests** — the highest-value tests in this project. Given the same fiscal year and
  parameters, the new trial balance, ledgers and stock balances must equal the legacy system's
  output. This is what proves "logic preserved exactly", and it is worth building the harness for it
  early.

---

## 7. What is deliberately not in this architecture

Listed so the omissions read as decisions rather than oversights:

- No microservices, message queue, or event sourcing.
- No GraphQL — the client is one first-party SPA over a well-understood REST surface.
- No ORM — the domain is SQL-shaped.
- No Redux — there is almost no client-side state.
- No caching layer initially. Add one when a report is measurably slow, informed by real query
  plans, not in anticipation.
- No FastReport equivalent — server-side PDF rendering covers it.
- No multi-currency — the legacy system has none despite its name. If it is genuinely wanted, that
  is a new feature and needs specifying, not porting.

Each of these is cheap to add later if measurement justifies it, and expensive to remove once it is
load-bearing.
