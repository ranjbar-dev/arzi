_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## PROPOSED IMPROVEMENTS (needs user approval)

*Everything below is a proposal. Nothing here has been agreed. None of it changes the
business logic, which is preserved exactly per the brief; these address security,
correctness and operability of the platform layer only.*

### A. Security — critical (recommend treating as non-negotiable)

| # | Proposal | Replaces |
|---|---|---|
| A1 | Store passwords with **Argon2id** (`argon2` crate), per-user salt, tuned work factor. Never store, log or transmit the plaintext. | Plaintext `Password` column (`Admin.pas:289`, `ChangePasswordU.pas:67`) |
| A2 | Compare with a **constant-time** verify, and make comparison **case- and whitespace-sensitive**. | `UpperCase(Trim(...))` compare (`GetPassu.pas:85-88`) |
| A3 | Remove the **username dropdown**. Users type their own identifier. | `Select * From PassWord Where Enabled = 1` bound to a combo (`GetPassu.dfm:143-146`) |
| A4 | Add **failed-attempt tracking**, progressive delay and account lockout, with an admin unlock path and an audit entry per event. | Nothing (`GetPassu.pas:88-93`) |
| A5 | Move the database credential **out of client reach entirely**. The Rust backend holds it in an env var / secrets manager; the browser never sees a connection string. | `CS2` in a shared ini file, obfuscated with a compile-time key (`Dmu.pas:726-737`, `INI.pas:43-170`) |
| A6 | Enforce **every** permission server-side, per command and per query, with the UI merely reflecting the server's answer. | Presentation-only `.Enabled`/`.Visible` gating (~120 call sites) |
| A7 | Eliminate all **string-concatenated SQL**. Use `sqlx` compile-time-checked queries or parameterised statements exclusively. | ~200 sites incl. `Dmu.pas:1557`, `Mainu.pas:408-411`, `GetPassu.pas:72` |
| A8 | Delete every **hard-coded secret**: the archive password (`Backup_U.pas:141`), the lab PIN (`Lab.pas:125`), the generator password (`testmainU.pas:93`), the licence bypass constant (`testmainU.pas:231`), the cipher key (`INI.pas:15`). | — |
| A9 | Remove the **Ctrl+Alt drag privilege escalation** (`Mainu.pas:501-532`). New-company creation becomes an ordinary, audited, permissioned action. | — |
| A10 | Forbid creating a user **without** a password. Require an invite/first-login flow. | `Admin.pas:176-181` |
| A11 | Sessions: short-lived tokens, idle timeout, explicit logout, one identity per request, session listing and revocation for admins. | Global mutable `DM.UserId` / `DM.Admin` (`GetPassu.pas:94-97`) |
| A12 | Enforce a password policy (length, no reuse of the last N, optional expiry) and require re-authentication for sensitive actions. | Three trivial checks (`ChangePasswordU.pas:48-65`) |

### B. Authorization model

| # | Proposal |
|---|---|
| B1 | Replace numeric ids with **stable string keys** (`accounting.document.post`, `warehouse.invoice.delete`, …), keeping a legacy-id column for the migration. |
| B2 | Introduce **roles** (a named set of permissions) with users assigned to roles. Keep per-user overrides if the customer needs them. Today every user is configured individually across ~85 checkboxes. |
| B3 | Make `Supervisor` a role, and split its implicit **lock-bypass** into its own explicit permission. |
| B4 | Save permissions in a **single transaction**, and record a diff in the audit log. |
| B5 | Load a user's full permission set **once per session** into a bitset/`HashSet`, not one query per check (`Dmu.pas:1552-1562`; `Admin.pas:225-228` currently issues ~85 queries per grid-row change). |
| B6 | Resolve the dead/ambiguous permissions from §13 Q6–Q10 and either implement or delete them. |
| B7 | Decide whether permissions are tenant-scoped. |

### C. Audit trail

| # | Proposal |
|---|---|
| C1 | Add an **append-only** `audit_log` (actor, timestamp, action, entity type, entity id, tenant, before/after JSON, source IP, session id), insert-only at the database level. |
| C2 | Cover the events that are recorded **nowhere** today: login success/failure, logout, lockout, password change, user create/enable/disable/rename, permission grant/revoke, document lock/unlock, deletes, settings changes, fiscal-year create/open/close, backup, export. |
| C3 | Capture **before** values, not just the last editor id. |
| C4 | Give admins a searchable, filterable audit viewer with export. |
| C5 | Keep `M_User`/`DM_MUser`/`DM_CUser` as denormalised convenience columns for reports, but treat the audit log as the record of truth. |

### D. Settings

| # | Proposal |
|---|---|
| D1 | **One** settings store. Machine/deployment config in env vars; tenant and user preferences in PostgreSQL. Retire the ini file entirely. |
| D2 | Remove the hard-coded `D:\BACKUP` and `\\pesteh\SahamData\` paths (`Dmu.pas:711`, `:759`). |
| D3 | Give `Base.IsActive` a real UI: an audited open/close-period action. |
| D4 | Decide the scope of `Tanzim` print parameters (§13 Q14) and key them accordingly. |
| D5 | Replace feature-gating-by-database-presence (`Dmu.pas:765-777`) with explicit per-tenant feature flags. |
| D6 | Validate every setting on write (dates, paths, digit widths) — today `TanzimU` writes most fields with no validation at all. |

### E. Licensing

| # | Proposal |
|---|---|
| E1 | Drop hardware node-locking entirely. Replace with a server-side subscription/entitlement check evaluated per request and **failing closed**. |
| E2 | Model tenants explicitly (organisation → companies → fiscal years) instead of overloading `Base.Co_ID` as both company and year. |
| E3 | Express seat limits, feature tiers and usage caps as entitlements, enforced in the backend. |
| E4 | Delete `test.dpr`, `testmainU.pas` and `LockUnit.pas`. |

### F. Backup / restore / operations

| # | Proposal |
|---|---|
| F1 | Move backups to **infrastructure** (managed PostgreSQL snapshots + WAL archiving). Remove the in-app `BACKUP DATABASE` (`Mainu.pas:393-414`). |
| F2 | If an in-app export is still wanted, make it a **logical export** (`pg_dump`/`COPY`) that is complete — including blobs like `ARM` — and verified. |
| F3 | Build an actual **restore** path with a documented, rehearsed runbook. There is none today. |
| F4 | Make fiscal-year creation copy the **chart of accounts** (or ask explicitly), remap `C1081`/`C1082`, validate inputs, and run in a transaction. |
| F5 | Make import a first-class, reachable feature with real format validation, a dry-run preview and a row-level error report. It is currently unreachable (`Mainu.pas:863-865`). |
| F6 | Report every error to the user. `DoBackup` and `Backup_U` currently swallow or ignore failures. |

### G. Concurrency & correctness

| # | Proposal |
|---|---|
| G1 | Add optimistic concurrency (a `version` column or `xmin`) on every editable entity; surface a real conflict to the UI instead of silently overwriting. |
| G2 | Wrap multi-statement operations in explicit transactions with an appropriate isolation level: `DMoein_Make`, `Dmoein_UpdateMab`, permission save, fiscal-year creation. |
| G3 | Use database-generated identities (sequences/identity columns) instead of client-side `max(x)+1` (`Admin.pas:175`, `Dmu.pas:1247`, `:1258`). |
| G4 | Keep the advisory document/account locks as a *business* feature (they are meaningful to accountants), but back them with real transactional guarantees and audit every toggle. |
| G5 | Make the `TX` state machine a server-enforced transition table, not a set of button-visibility rules. |
| G6 | Push list refresh to clients (SSE/WebSocket) or at minimum refetch on focus, so stale grids are not the default. |

### H. Application shell / UX

| # | Proposal |
|---|---|
| H1 | Turn the ribbon into a proper **route-based navigation** with URLs, deep links, browser history and back/forward. |
| H2 | Add **keyboard shortcuts** — there are literally none today — and full keyboard navigation. |
| H3 | Ship an accessible React app (focus management, ARIA, contrast) with proper **RTL** support via CSS logical properties rather than per-control `BiDiMode`. |
| H4 | Lazy-load routes. Nothing needs a 47-step splash screen (`arzi.dpr:147-294`). |
| H5 | Hide, rather than merely disable, actions the user cannot perform; and explain *why* when disabling is the better UX. |
| H6 | Fix the mislabelled controls found here: `B_Pass` opens the **fiscal-year** selector (`Mainu.pas:421`); `C1122`'s caption `کزارش` is a typo for `گزارش`; `Sytem1..5` for "System"; `IsEnabel` for `IsEnabled`; `Unkhnown` for `Unknown` (`Dmu.pas:1186`); the invoice/receipt permission-caption mismatch (§1.4). |
| H7 | Remove all dead UI: `Sarfasl_Add`, `SMoein5`, `AR_Chap2`, `AR_Kholaseh`, `_Report9`, `B_Bardasht`, `Bank_Tanzim`, `B_Enteghal2`, `B_CloseMoein`, and the six hidden developer buttons (§1.8). |
| H8 | Preserve `Memo1`'s `M_ID` → source-table mapping (`Mainu.dfm:4319-4380`) as real documentation and, ideally, as a typed enum in the domain model. |

### I. Code health / build

| # | Proposal |
|---|---|
| I1 | Consolidate the triplicated utilities (`ElfHash`, the cipher suite, `inttostr3`, `N23`/`Str2String`, `IsInteger`, the Shaba/card validators) into one implementation each, choosing the variant production actually calls. |
| I2 | Establish real automated tests. There are **none** today — `test.dpr` is a key generator, not a test suite. Priority: golden-file tests for the Jalali conversion and the number-to-Persian-words output. |
| I3 | Delete the empty shells: `GetCodeStringU.pas`, `DateFrameU.pas`, `TanzimPU.pas`, plus dead code `EncryptText`/`DecryptText` and `IntToRoman`. |
| I4 | Normalise all source to UTF-8. Several files are Windows-1256 (`GetPassu.pas`, `SayMessage.pas`, `Lib.inc`, `MakeNewU.pas`, `Get_Serial.pas`, `Lab.pas`), which is why their Persian message strings appear as mojibake — some of those messages are **unrecoverable from the source alone** and must be re-elicited from the customer. |
| I5 | Use SQLite (or `sqlx`'s test transactions) for the test suite per the brief, with the schema generated from the same migrations as PostgreSQL. |

---

Prev: [13. Open questions](08-13-open-questions.md) · Next: [index](00-index.md)
