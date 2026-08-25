# 01 — Roadmap: Full Phase / Step Checklist

46 steps across 8 phases, dependency-ordered. Each row is one step in the matching `phase-*.md`
file, where the full build instructions, spec references and manual test procedure live. This table
is the checklist — tick steps off here as they pass their manual test.

| # | Step | Goal | Key spec refs | Fixes / decisions |
|---|---|---|---|---|
| **Phase 0 — Foundations** | | | | |
| 0.1 | Repo scaffold & Docker Compose | Cargo workspace + Next.js app + Postgres, all running via `docker compose up` | `10-target-architecture.md` §5 | — |
| 0.2 | Database bootstrap & migrations | `sqlx migrate` wired, empty baseline migration, pooled connection, tenant-context transaction helper | `10-target-architecture.md` §2.4 | — |
| **Phase 1 — Platform, tenancy & auth** | | | | |
| 1.1 | Core schema: tenants, fiscal years, org, users + RLS | Every table gets `tenant_id` + row-level security from the first migration | `02-11-a/b`, A3 | A3 |
| 1.2 | Authentication | Argon2id + server-side sessions, login/logout | `08-03-authentication.md`; `10-...` §2.5 | C1 — full rebuild, no legacy behaviour ported |
| 1.3 | Authorization | Permission catalogue + enforced-server-side middleware | `08-04-authorization.md` | C1; B23, B24 |
| 1.4 | Audit log | `audit_log` on every mutation, incl. item changes | `08-05-audit-trail-change-log.md` | C1; A17 |
| 1.5 | Fiscal year management | Create/switch fiscal year, `Base.IsActive` as explicit admin action | `07-01-a/b`, A5, A6 | A5, A6; B21 (cancel-applies-change) |
| 1.6 | Frontend shell | Next.js RTL shell, nav tree skeleton, login page, protected routing | `08-01-the-complete-main-menu-tree.md`; `10-...` §3.2–3.3 | — |
| **Phase 2 — Accounting core** | | | | |
| 2.1 | Chart of accounts schema + API | 4-level `accounts` hierarchy, leaf-only posting | `03-01-a/b`, `03-02-a/b` | — |
| 2.2 | Chart of accounts UI | Tree editor (create/rename/recode/promote/demote/lock) | `03-12-a/b/c` (SNewu screen) | C5 — merges `S_KolU`/duplicate pickers |
| 2.3 | Voucher model + state machine | Header+lines, `0→1→2`, balance check only on issue | `03-03-a/b/c`, `03-04-voucher-validation-rules.md` | — |
| 2.4 | Voucher editor UI | Line grid, add/edit/delete, import, balance indicator | `03-12-a/b/c` (SanadEditU screen) | — |
| 2.5 | Automatic voucher generation engine | Generic engine other domains call to post their own vouchers | `03-06-a/b` | — |
| 2.7 | Period close / year-end | `NewFinalu` (close) → `EnteghalU` (carry-forward), order enforced | `03-09-a/b/c`, A7 | A7 |
| 2.8 | Accounting-core permissions | Wire the permission catalogue to every accounting-core route | `03-13-permissions.md` | — |
| **Phase 3 — Parties** | | | | |
| 3.1 | Party register schema + CRUD | Person/legal-entity record; creating one creates its leaf account node | `07-01-a/b`, `07-02-a/b`, `07-03`, `07-04-a/b` | — |
| 3.2 | Party current account (Jari) | Running balance view + `SahamdarConfig`, fix scratch-column corruption | `07-06-a/b`, `07-07` | B18 |
| 3.3 | Shareholder equity module | Share counts, nominal value, %, join/exit dates, profit allocation (new logic) | `07-05-shareholder-equity-profit-distribution.md`, A4 | A4 |
| 3.4 | Party UI screens | Person/company editors, party master list, bank accounts | `07-10-screen-by-screen-ui-specification.md` | B22 (hard-coded user id) |
| **Phase 4 — Treasury** | | | | |
| 4.1 | Cheque schema + state machine | Received/issued cheque lifecycle with unambiguous states | `06-01`, `06-02`, `06-03` | B11 |
| 4.2 | Cheque accounting integration | Every state transition posts correctly; delete actually deletes | `06-08-accounting-integration.md` | B10, B12, B13 |
| 4.3 | Cheque endorsement | Third-party transfer — feature never implemented in legacy | `06-04-endorsement-transfer-third-party.md` | B14 |
| 4.4 | Deposit slips + petty cash | `Fish` and `Tankhah` documents, each posting its own voucher | `06-06`, `06-07` | — |
| 4.5 | Treasury registers/filters UI | Received/issued registers, working filters, due-date aging | `06-11-a/b/c` | B15 |
| **Phase 5 — Inventory** | | | | |
| 5.1 | Item master + warehouses + UoM | Items, warehouses, units of measure | `05-01`, `05-02-a/b` | — |
| 5.2 | Invoice (Factor) documents | Purchase/sale/production/transfer document types | `05-03-a/b`, `05-04-a/b/c` | B7 |
| 5.3 | Stock quantity mathematics + stock card | Running stock-on-hand, item ledger card | `05-05-a/b`, `05-11-stock-card-and-balance.md` | B8, B9 |
| 5.4 | Costing & valuation | Costing method, worked-example verified | `05-06-a/b` | — |
| 5.5 | Pricing | Price list / pricing rules | `05-07-a/b` | — |
| 5.6 | Pistachio deduction calculator | Weight/grade deduction formula, now reachable in UI | `05-08-a/b/c` | B19 |
| 5.7 | Settlement (Tasfieh) | Settle invoices against cash/cheque receipts | `05-09-a/b` | — |
| 5.8 | Inventory → accounting integration | Purchase/sale/production/transfer post balanced vouchers | `05-10-a/b` | B1, B2 |
| 5.9 | Inventory screens | Item/invoice editors, warehouse settings, invoice list | `05-13-a/b/c` | — |
| **Phase 6 — Reporting** | | | | |
| 6.1 | Trial balances | 4-column and 6-column, posted-only | `04-02-a/b`, A8 | A8 |
| 6.2 | General & subsidiary ledgers | Daftar Kol / Daftar Moein, fixed opening-balance handling | `04-03-a/b/c` | B4, B5, B6 |
| 6.3 | Card Jari statement | Party running-account statement | `04-04-a/b` | — |
| 6.4 | Stock / warehouse reports | Movement, activity, pistachio-ops reports, no in-place table mutation | `04-01-a/b/c` | B3, B16, B25 |
| 6.5 | Print pipeline | Server-side PDF rendering, RTL-correct | `04-06-a/b` | — |
| 6.6 | Export pipeline | Excel/CSV, tax export posted-only | `04-07-export-pipeline.md` | B17 |
| 6.7 | Report permission gating | Every report route checked, no ungated menu items | `04-01`, `08-04` | B23, B24 |
| **Phase 7 — Hardening & cutover** | | | | |
| 7.1 | Constraint audit & enable | FKs, `NOT NULL`, `CHECK` — audit data, then enable | `02-13-a/b`, C2 | C2 |
| 7.2 | Migration readiness doc | Exact queries for A1/A9/A11–A16, ready to run once a live DB exists | `11-open-decisions.md` Group A | — |
| 7.3 | Backup/restore + new-fiscal-year wizard | Admin-triggered backup, restore, year creation | `08-08-backup-restore-new-company-import.md` | — |
| 7.4 | Production Docker/deploy | Multi-stage build, health checks, prod compose override | `10-target-architecture.md` §5 | — |
| 7.5 | Reconciliation test harness | Given same inputs, new system's trial balance/ledgers/stock match legacy — held ready for when migration runs | `10-target-architecture.md` §6 | — |

## Deferred pending live database access

Not scheduled into any phase above — see step 7.2 and `11-open-decisions.md`:

- **A1** — Jalali date storage format validation against real historical strings.
- **A9** — `CheckMaster` batch-vs-single-cheque row distribution.
- **A11–A16** — `Sahamdar_Edit` existence, `S_IS_*` column ownership, year-suffixed physical
  tables, `Kinds` table, 19 unreferenced stored procedures, `Sahamdar` DDL mismatch.

None of these block Phases 0–6: the new schema is being built clean, not migrated, per the decision
in `00-overview.md`.
