# 02 — Data Model, Persistence and Shared Infrastructure

> Specification document for the rebuild of **arzi** (legacy Delphi/VCL + MS SQL Server + ADO)
> onto **Rust / React / PostgreSQL / Docker**.
> The legacy code is a **logic reference only**. Naming below is proposed clean English;
> every legacy Persian/Finglish identifier is preserved alongside it.
>
> Cross-references (do not duplicate): `docs/01-glossary.md` (term map + naming conventions),
> `docs/03-accounting-core.md`, `docs/07-parties-and-shareholders.md`,
> `docs/08-platform-and-security.md`, `docs/09-unit-index.md`.

---

This document has been split into per-section files under [`02-data-model/`](02-data-model/00-index.md)
for readability. Existing cross-references of the form "`02-data-model.md §N`" resolve here: find
§N below and open the linked file(s). See also [`02-data-model/00-index.md`](02-data-model/00-index.md).

| Section | Title | File | Approx lines |
|---|---|---|---|
| §1 | Connection and runtime topology | [02-01-connection-and-runtime-topology.md](02-data-model/02-01-connection-and-runtime-topology.md) | 242 |
| §2 | Table inventory (2.0–2.5: derivation, naming, master list, `Base`, `Base_Config`, `Sarfasl`) | [02-02-a-table-inventory-overview.md](02-data-model/02-02-a-table-inventory-overview.md) | 229 |
| §2 | Table inventory (2.6–2.8: `Sahamdar`, `Moein`, `DMoein`) | [02-02-b-table-inventory-parties-vouchers.md](02-data-model/02-02-b-table-inventory-parties-vouchers.md) | 164 |
| §3 | Stored procedures (3.0 scope + 3.1 part 1) | [02-03-a-stored-procedures-overview.md](02-data-model/02-03-a-stored-procedures-overview.md) | 256 |
| §3 | Stored procedures (3.1 part 2, continued) | [02-03-b-stored-procedures-continued.md](02-data-model/02-03-b-stored-procedures-continued.md) | 241 |
| §3 | Stored procedures (3.2–3.5: functions, classification, cross-cutting, dump list) | [02-03-c-stored-procedures-functions-and-summary.md](02-data-model/02-03-c-stored-procedures-functions-and-summary.md) | 127 |
| §4 | Ad-hoc SQL inventory (4.0–4.2: scope, schema-mutating SQL, connection/startup) | [02-04-a-adhoc-sql-schema-and-startup.md](02-data-model/02-04-a-adhoc-sql-schema-and-startup.md) | 227 |
| §4 | Ad-hoc SQL inventory (4.3–4.4: accounting-core SQL, chart-of-accounts/party lookups) | [02-04-b-adhoc-sql-accounting-and-lookups.md](02-data-model/02-04-b-adhoc-sql-accounting-and-lookups.md) | 252 |
| §4 | Ad-hoc SQL inventory (4.5: design-time dataset SQL) | [02-04-c-adhoc-sql-design-time-datasets.md](02-data-model/02-04-c-adhoc-sql-design-time-datasets.md) | 264 |
| §4 | Ad-hoc SQL inventory (4.6–4.9: new fiscal year, backup, no-SQL units, summary) | [02-04-d-adhoc-sql-misc-and-summary.md](02-data-model/02-04-d-adhoc-sql-misc-and-summary.md) | 171 |
| §5 | Keys, identity and document numbering | [02-05-keys-identity-and-document-numbering.md](02-data-model/02-05-keys-identity-and-document-numbering.md) | 263 |
| §6 | Date handling (6.1–6.4: summary, "today", the two Jalali algorithms, input validation) | [02-06-a-date-handling-storage-and-algorithms.md](02-data-model/02-06-a-date-handling-storage-and-algorithms.md) | 213 |
| §6 | Date handling (6.5–6.8: string-comparison arithmetic, `DateS`, UI, proposed model) | [02-06-b-date-handling-arithmetic-and-model.md](02-data-model/02-06-b-date-handling-arithmetic-and-model.md) | 108 |
| §7 | Money and amount handling | [02-07-money-and-amount-handling.md](02-data-model/02-07-money-and-amount-handling.md) | 220 |
| §8 | Configuration and settings storage (8.1–8.2: ini file, `Tanzim` table) | [02-08-a-configuration-ini-and-tanzim.md](02-data-model/02-08-a-configuration-ini-and-tanzim.md) | 197 |
| §8 | Configuration and settings storage (8.3–8.6: `Base`, `Anbar_Config`, registry, proposed model) | [02-08-b-configuration-base-and-registry.md](02-data-model/02-08-b-configuration-base-and-registry.md) | 117 |
| §9 | Concurrency, locking and transactions | [02-09-concurrency-locking-and-transactions.md](02-data-model/02-09-concurrency-locking-and-transactions.md) | 304 |
| §10 | Backup, restore, new-year creation and import (10.1–10.4) | [02-10-a-backup-and-restore.md](02-data-model/02-10-a-backup-and-restore.md) | 180 |
| §10 | Backup, restore, new-year creation and import (10.5–10.8: year-end close, import, proposed model) | [02-10-b-year-end-and-import.md](02-data-model/02-10-b-year-end-and-import.md) | 204 |
| §11 | Proposed PostgreSQL DDL (11.0–11.1: how to read this section, extensions/enums/domains) | [02-11-a-ddl-overview-and-extensions.md](02-data-model/02-11-a-ddl-overview-and-extensions.md) | 223 |
| §11 | Proposed PostgreSQL DDL (11.2: platform — fiscal years, organisation, settings, users) | [02-11-b-ddl-platform.md](02-data-model/02-11-b-ddl-platform.md) | 265 |
| §11 | Proposed PostgreSQL DDL (11.3: parties and the chart of accounts) | [02-11-c-ddl-parties-and-accounts.md](02-data-model/02-11-c-ddl-parties-and-accounts.md) | 179 |
| §11 | Proposed PostgreSQL DDL (11.4: accounting core — vouchers and voucher lines) | [02-11-d-ddl-accounting-core.md](02-data-model/02-11-d-ddl-accounting-core.md) | 180 |
| §11 | Proposed PostgreSQL DDL (11.5: treasury, part 1 — cheques, cheque_events, deposit_slips) | [02-11-e-ddl-treasury-1.md](02-data-model/02-11-e-ddl-treasury-1.md) | 174 |
| §11 | Proposed PostgreSQL DDL (11.5: treasury, part 2 — cheque/petty-cash payment docs, cheque_types) | [02-11-f-ddl-treasury-2.md](02-data-model/02-11-f-ddl-treasury-2.md) | 159 |
| §11 | Proposed PostgreSQL DDL (11.6: inventory) | [02-11-g-ddl-inventory.md](02-data-model/02-11-g-ddl-inventory.md) | 239 |
| §11 | Proposed PostgreSQL DDL (11.7–11.8: compliance/integrations, deferred/cross-cutting objects) | [02-11-h-ddl-compliance-and-deferred.md](02-data-model/02-11-h-ddl-compliance-and-deferred.md) | 112 |
| §12 | Open questions (12.1–12.8: Jalali storage, `S_DateS`, procedures/functions/DDL to dump, constraints, triggers) | [02-12-a-open-questions-dates-and-procedures.md](02-data-model/02-12-a-open-questions-dates-and-procedures.md) | 203 |
| §12 | Open questions (12.9–12.17: collation, enumerations, uniqueness, fiscal-year gaps, `BN_*`, volume) | [02-12-b-open-questions-schema-and-volume.md](02-data-model/02-12-b-open-questions-schema-and-volume.md) | 193 |
| §13 | PROPOSED IMPROVEMENTS — needs user approval (13.1–13.12: integrity, uniqueness, FKs, natural keys) | [02-13-a-improvements-integrity-and-keys.md](02-data-model/02-13-a-improvements-integrity-and-keys.md) | 204 |
| §13 | PROPOSED IMPROVEMENTS — needs user approval (13.13–13.23: banks/cheques, rounding, security, audit) | [02-13-b-improvements-security-and-audit.md](02-data-model/02-13-b-improvements-security-and-audit.md) | 165 |
| §14 | Naming map (14.1–14.9: tables, column patterns, `Base`, `Base_Config`, `Sarfasl`, `Sahamdar`, `Moein`, `DMoein`, `DCheck`) | [02-14-a-naming-map-tables-and-columns.md](02-data-model/02-14-a-naming-map-tables-and-columns.md) | 232 |
| §14 | Naming map (14.10–14.20: remaining tables, settings, procedures → Rust, data-module members) | [02-14-b-naming-map-procedures-and-modules.md](02-data-model/02-14-b-naming-map-procedures-and-modules.md) | 267 |
