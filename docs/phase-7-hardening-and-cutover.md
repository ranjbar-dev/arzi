# Phase 7 — Hardening & Cutover

By this point the system is functionally complete and self-consistent — but it was built clean, not
migrated (per the decision in `00-overview.md`). This phase adds the constraints the legacy never
had, prepares (but does not execute) the eventual legacy-data migration, and gets the system
deployable.

---

## 7.1 Constraint audit & enable

**Goal:** every constraint proposed across the domain specs' "PROPOSED IMPROVEMENTS" sections — all
approved per Group C in `11-open-decisions.md` — is actually present in the schema by the end of this
step. Since this is a fresh build (no legacy data to violate a constraint), most of these should
already hold from how earlier phases were specified; this step is the audit that confirms it, not a
first attempt.

**Build**

- Audit against `02-13-a/b-improvements-integrity-and-keys.md` / `-security-and-audit.md`,
  confirming each is genuinely enforced by the database, not just assumed by application code:
  - `UNIQUE (tenant_id, level1_code, level2_code, level3_code, level4_code)` on `accounts` — added
    in Phase 2.1; confirm it survived.
  - `UNIQUE (fiscal_year_id, voucher_number)` and the equivalent for invoice numbers — the counter
    allocator from Phase 2.3/5.2 should make this automatic; confirm the constraint exists as a
    backstop, not just the allocator's good behaviour.
  - Voucher balance rule enforced at the database boundary in addition to the Phase 2.5 engine's
    application-level check — a `CHECK` or trigger asserting `status = 'draft' OR total_debit =
    total_credit`, so the rule holds even against a hypothetical future write path that bypasses the
    engine.
  - `debit = 0 OR credit = 0` and non-negativity on every voucher line.
  - Real foreign keys everywhere a legacy column was a bare, unenforced integer reference (account
    references, party references, cheque/deposit-slip source links, etc.) — confirm none were
    quietly left as a plain `bigint` with no `REFERENCES`.
  - `accounts.parent_id` + `level` (per `02-13-a.md` §13.6) — confirm the hierarchy has a real
    parent pointer, not just the four-segment positional encoding, if that was the design adopted in
    Phase 2.1.
  - `accounts.party_id` (per §13.7) — confirm the party↔account link from Phase 3.1 is a real FK,
    not a positional convention.
  - No denormalised `*_name`/`*_code`/`*_state_name` columns anywhere (per §13.11) — confirm every
    display value is derived by join/enum at read time. This should already hold throughout, since
    every phase's build notes said "derive, don't store" (C3) — this is the final sweep confirming it.
- Since there's no legacy data to break these constraints, there's no "audit data first" step to run
  for real — but write the audit query set anyway (row-count-of-violations-if-any, per constraint),
  because Phase 7.2's eventual migration will need exactly these queries to validate incoming legacy
  data before it's loaded.

**Spec refs:** `02-13-a/b-improvements-integrity-and-keys.md`, `-security-and-audit.md`; C2 in
`11-open-decisions.md`.

**Manual test**

1. For each constraint in the audit list, attempt an operation that should violate it directly via
   the database (bypassing the API) — confirm the database itself rejects it, not just the
   application layer.
2. Run the audit query set (violation counts) against the current, freshly-built database → confirm
   zero violations everywhere (expected, since nothing has migrated legacy data in yet).
3. Confirm the audit query set is saved somewhere reusable — it becomes step 3 of the migration
   procedure in 7.2.

**Done when:** every approved Group C constraint is present and enforced at the database level, and
the violation-audit queries exist and are ready for future migration use.

---

## 7.2 Migration readiness doc

**Goal:** a precise, ready-to-run procedure for the day a populated legacy database becomes
available — not an attempt to migrate now. This step produces a document, not code that runs against
production.

**Build**

Write `docs/migration-readiness.md` (or similar) containing, verbatim where possible, the exact
queries `11-open-decisions.md` already specifies for each still-blocked item:

- **A1 — Jalali date format.** The queries needed: sample date strings per column, plus a known
  real-world date (or a range spanning a known Nowruz) to test `dbo.Farsi_Date` and both Delphi
  algorithms against. Document that the new schema stores real `date` columns and Jalali is
  display-only (already decided) — what's still needed is validating *which legacy algorithm* to
  trust when converting historical string dates during import.
- **A9 — `CheckMaster` batch semantics.** `SELECT CM_Count, COUNT(*) ... GROUP BY CM_Count` against a
  populated `CheckMaster` — determines whether issued-cheque batches are usually size 1 (behaves like
  a single cheque) or genuinely multi-line, which affects how Phase 4's issued-cheque model should
  map legacy rows.
- **A11 — Does `Sahamdar_Edit` exist?** Either a fresh procedure dump or a direct read of
  `SahamdarEditU.pas` to determine whether party CRUD went through a named procedure or inline SQL —
  affects whether Phase 3's party-creation migration needs to replicate stored-procedure side effects.
- **A12 — Who maintains `Sarfasl.S_IS_*`?** `SELECT COUNT(*) FROM Sarfasl WHERE S_IS_Check=1 OR
  S_IS_Fish=1 OR S_IS_APArdakhti=1 OR S_IS_ADaryafti=1` plus a check of `sys.jobs`/`sysjobsteps` —
  determines whether these columns (already dropped from the rebuild's schema per Phase 2.1) hold any
  historically meaningful data that needs preserving elsewhere before a legacy database is decommissioned.
- **A13 — Year-suffixed physical tables.** `SELECT name FROM sys.tables WHERE name LIKE '%1403' OR
  name LIKE '%1404' OR name LIKE 'Tah%' OR name LIKE 'mandeh_%'` plus a direct question to the
  business about whether this is a known year-end archival practice — determines whether the
  migration's data-discovery scope is larger than the tables named in the domain specs.
- **A14 — Does `Kinds` exist?** Direct query/question to the operator — resolves whether
  `pistachio_grades` (Phase 5.1) needs to seed from a real table or purely from the 7-value
  enumeration recovered from source comments.
- **A15 — 19 unreferenced stored procedures.** Check `sys.jobs`/`sysjobsteps` on the live server for
  whether any are invoked by a SQL Agent job or a second front-end before deciding whether any need a
  rebuild equivalent.
- **A16 — `Sahamdar` DDL mismatch.** A fresh schema dump or a source re-check to resolve which of the
  two conflicting column lists (the dump's vs. the domain spec's) reflects current production —
  affects the exact field mapping used when migrating `Sahamdar` rows into Phase 3's `parties` table.
- **A17 — already ruled (Add it, per today's decision)** — no further data needed; note in this doc
  that it's closed, so a future reader doesn't re-open it.
- The 6-step migration procedure from `10-target-architecture.md` §4 (dump schema + procedure bodies
  → determine Jalali format → ETL with `[NEW]` constraints disabled → run the Phase 7.1 audit queries
  against the imported data → remediate and enable constraints one at a time → reconcile every trial
  balance/ledger/stock balance against the legacy system's output for the same parameters). Restate
  it here as the concrete checklist, cross-referenced to the specific queries above.

**Spec refs:** `11-open-decisions.md` Group A (A1, A9, A11–A17); `10-target-architecture.md` §4.

**Manual test**

1. Review the resulting document with a fresh eye (or have someone else review it) — confirm every
   listed query is copy-paste-ready against a real SQL Server connection, with no placeholder left
   unfilled.
2. Confirm the document explicitly states what is *not* needed anymore (A2 — already closed; A3–A8,
   A10 — already decided) so a future reader doesn't waste time re-deriving settled questions.

**Done when:** a person with SQL Server access to the legacy database, and no other context, could
follow this document top to bottom and produce the exact answers `11-open-decisions.md` is still
waiting on.

---

## 7.3 Backup/restore + new-fiscal-year wizard

**Goal:** real backup/restore (the legacy had backup but explicitly **no restore feature at all**,
`08-08.md` §8.2's closing line) and a safe, validated new-fiscal-year creation flow (the legacy's
was gated behind a hidden Ctrl+Alt drag gesture and copied the chart-of-accounts block was commented
out, `08-08.md` §8.3).

**Build**

- **Backup**: scheduled + on-demand `pg_dump`-based backup, with retention/rotation, stored somewhere
  durable (object storage, not a client-writable path — fixing the legacy's "client-side `FileExists`
  check is meaningless for a remote server" defect, `08-08.md` §8.1). Runs as a real background job,
  not smuggled into every login (`08-08.md` §8.1: the legacy's auto-backup ran unconditionally from
  `Reload` on every login, before the licence check, with a process-lifetime "already ran" flag as
  its only guard).
- **Restore**: an actual, tested restore path — closing the "no unit reads an `.ABS` archive back"
  gap (`08-08.md` §8.2) entirely. Test restoring into a scratch database as part of this step, not
  just taking backups on faith.
- **New fiscal year wizard**: reachable through a normal, permission-gated admin action — not a
  hidden gesture (`08-08.md` §8.3's `Ctrl+Alt` drag finding is B20, already ruled "fix"). Validates
  the new year's id/dates properly (the legacy validated neither, `08-08.md` §8.3's "not validated"
  list) and does **not** blindly copy the previous year's row string-for-string, which in the legacy
  carried forward stale `IsActive` and account-SSN references that "point at chart-of-accounts rows
  that do not exist in the new year" — the rebuild sets every field explicitly. Chart of accounts
  stays global (A6, already decided) — so there's no "clone the chart of accounts" step to leave
  half-implemented; document explicitly that this is intentional, not an oversight, unlike the
  legacy's commented-out clone block.

**Spec refs:** `08-08-backup-restore-new-company-import.md`; B20 in `11-open-decisions.md`; A6 in
`11-open-decisions.md`.

**Manual test**

1. Trigger a backup on demand → confirm it completes and the artifact lands in durable storage, not a
   path only meaningful on one machine.
2. Restore that backup into a scratch database → confirm the data matches (this is the direct fix for
   the legacy's complete absence of a restore path — actually prove it works).
3. Create a new fiscal year through the wizard as an admin → confirm proper validation (bad
   date ranges, duplicate year ids rejected) and confirm the resulting year has no stale references
   to the prior year's accounts.
4. As a non-admin, attempt to reach the new-fiscal-year action → rejected via a normal permission
   check, not because a UI gesture is undiscoverable (direct B20 test — the legacy's "security" here
   was obscurity, not a real permission gate on the actual action).

**Done when:** backup and restore have both been exercised end-to-end at least once, and new-year
creation is a normal, validated, permission-gated action.

---

## 7.4 Production Docker/deploy

**Goal:** deployable, per `10-target-architecture.md` §5.

**Build**

- Multi-stage Rust build with `cargo-chef` for dependency-layer caching → distroless final image.
- Static frontend bundle behind nginx (or served by the API — pick one and document why).
- `postgres:17` with a named volume in the base compose file; a second, production-override compose
  file for anything environment-specific (secrets sourcing, resource limits, TLS termination) — one
  `docker-compose.yml` for local dev, an override for prod, not a templating layer.
- Health checks on `api` and `db` (extending the `/health` endpoint from Phase 0.1 to check DB
  connectivity, migration status, and any other liveness signal worth having).
- All configuration via environment variables — confirm nothing from Phase 1's credential handling
  regressed into a file-based secret (the legacy's entire security model was built on exactly that
  mistake, per `00-overview.md` fact 4 — this step is a final check that the rebuild never went there).

**Spec refs:** `10-target-architecture.md` §5.

**Manual test**

1. Build the production images from a clean checkout → confirm both build successfully and are
   reasonably sized (distroless final stage, no build toolchain baked in).
2. Run the production compose file locally with production-like environment variables → confirm the
   full stack comes up healthy.
3. Kill and restart the `db` container → confirm `api`'s health check correctly reflects the outage
   and recovers once `db` is back.
4. Grep the built images and the compose files for any hard-coded credential or connection string →
   confirm none exists.

**Done when:** a clean checkout can be built and deployed via the documented compose files with no
manual steps beyond setting environment variables.

---

## 7.5 Reconciliation test harness

**Goal:** the harness that will prove "logic preserved exactly" once a real migration happens — built
and ready now, even though there's no legacy data to run it against yet.

**Build**

- Per `10-target-architecture.md` §6's explicit call-out that this is "the highest-value tests in
  this project" and "worth building the harness for it early": given the same fiscal year and report
  parameters, assert the new system's trial balance, ledgers, and stock balances equal a reference
  output.
- Since there's no live legacy database to compare against yet, build the harness against **known
  worked examples already embedded in the specs** — every worked-arithmetic example cited throughout
  this plan (Phase 3.2's party-balance example, Phase 5.4's average-cost example, Phase 5.6's two
  pistachio-deduction examples) becomes a permanent regression test in this harness, not just a
  manual-test step that gets run once and forgotten.
- Structure the harness so that, once a populated legacy database and the migration from 7.2 exist,
  plugging in real legacy report output as the reference is a matter of adding fixtures — not
  rewriting the harness.

**Spec refs:** `10-target-architecture.md` §6.

**Manual test**

1. Run the harness against the current system → confirm every worked example from Phases 2–6 passes
   as an automated regression test, not just something that was manually verified once during that
   phase's development.
2. Deliberately introduce a wrong result in one calculation (e.g. break the pistachio deduction
   formula) → confirm the harness catches it and fails clearly.
3. Revert the deliberate break → confirm the harness passes again.

**Done when:** every worked-arithmetic example from this entire plan is a permanent, automated
regression test, and the harness is structured to accept real legacy comparison data the moment it
becomes available.
