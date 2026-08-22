# 11 — Open decisions: everything awaiting your sign-off

Most items below are now decided (2026-08-18) — marked **DECIDED** with the ruling inline. A1 and
A9 remain narrowly blocked on **data from a populated database** (schema alone cannot resolve
them, no amount of Q&A does either); A2 was resolved 2026-08-19 by a schema-only dump. That same
dump surfaced seven new blocked items (A11-A17) and one new confirmed defect (B25) — see "What's
left" at the bottom. The specification documents describe the system **as-is**; this file collects
everything that needs a human answer before the rebuild can proceed, in three groups:

- **A — Blocking unknowns.** Not answerable from source. The rebuild cannot be correct until these
  are resolved, mostly by querying the live database.
- **B — Broken behaviour.** Defects confirmed in the live code. Each needs a ruling: replicate the
  bug, or fix it.
- **C — Proposed improvements.** Suggestions only. Default is "no" — port as-is.

Each item states where the full detail lives.

---

## Group A — Blocking unknowns (need a live database or a person who knows)

These cannot be answered by reading code. Most need one session against a production database.

### A1. What Jalali date format is physically stored? ⛔ narrowed further — schema-only dump reviewed, sample-data decode still blocked

**Ruling: new schema stores real `DATE`/`TIMESTAMP` columns (Gregorian, e.g. Postgres `date`),
full stop.** No Jalali string column anywhere in the new database. Jalali is a **display-only**
concern — converted from the real date at the presentation layer, on the fly, for Persian users.
This also kills the "two incompatible algorithms disagree" problem going forward: pick one
correct Gregorian↔Jalali library (e.g. `jalaali-js` or equivalent) for the UI layer, done, no
ambiguity possible since the stored value is unambiguous ISO.

**2026-08-19 update — `Full_Script_14050527.sql` reviewed (SQL Server schema-only dump: DDL,
procedure/function/trigger bodies, zero rows).** This moves the migration-decode question from
"cause unknown" to "cause known, sample data still needed":

- **A third algorithm, `dbo.Farsi_Date`, was found** — the one that actually ran server-side for
  every "today" default (`XNew` calls `Dbo.Farsi_Date(GetDate()+1)`, clamped into the fiscal year).
  Full body captured in `02-data-model/02-12-a.md` §12.1.
- It uses the **same naive-leap-year design** as the dead `TUtil.FarsiDate`/`DecodedateF`
  (`Utility.pas:413-434`) — a `%4`-on-loop-counter approximation, not the real Persian 33-year
  cycle — but is **not identical**: the epoch offset differs (`ADate-78` vs `ADate-80`), which
  shifts the computed date near month/year boundaries. `TDM.MiladiToShamsi` (`Dmu.pas:362-437`)
  remains structurally unrelated to both — it is the one algorithm of the three that computes a
  correct Persian calendar date.
- Since `TUtil.FarsiDate` has no call site (confirmed dead) and `dbo.Farsi_Date` is what `XNew`
  actually invoked, **`dbo.Farsi_Date` is now the best candidate for what produced the "today"
  defaults offered on data-entry screens** — but this only covers defaulted values. Hand-typed
  dates went through `Tools.TEditDate`/`TFullDate` (still closed-source, still unobtainable per
  A10) and could use a fourth, still-unverified algorithm.

**What's still blocked: importing the existing data.** This dump has **zero rows** (0 `INSERT`
statements) — it cannot show the actual stored value shapes, and cannot be used to fingerprint-test
`dbo.Farsi_Date` (or the two Delphi algorithms) against ground truth. The one-time migration script
still needs a **populated** database or export: sample date strings per column, plus a known
real-world date (or a range spanning a known Nowruz) to run `dbo.Farsi_Date` and both Delphi
algorithms against and see which one actually lines up with history.

→ `02-data-model.md §6`, §12 (`02-12-a.md` §12.1 has the full comparison table); `08-platform-and-security.md §9`

### A2. Stored-procedure bodies and table DDL ⛔ narrowed — DDL/schema side DECIDED, data-profiling side still needs a live session

**2026-08-19: `Full_Script_14050527.sql` (SQL Server schema-only dump) reviewed end to end.**
6 of the 11 artefact categories in `02-data-model/02-12-b.md` §12.17 are now delivered:

- All 49 stored procedures captured, including `Sarfasl_ADD` (duplicate + parent-posting checks),
  `Sarfasl_Deep` (**confirmed: it deletes, guarded by a children/postings check — not report-only**),
  `Anbar_AddToFactor` (confirmed idempotent per `@factor` via `if not exists(...) insert`, so
  re-saving does not double voucher lines), `Anbar_CardJensi` (running-balance via a
  `SUM(...) WHERE P.SSN <= #R.SSN` correlated pattern), `Taraz_6Sotooni`, `KolState`, `Select_Kol`,
  `Active_Set` (resolves `S_Active = (S_Child=0 AND S_Mo>0)`), `MoeinAdd`, `XNew`, and 19 more
  procedures the source tree never named at all (see below).
- All 4 UDFs captured: `Make_L`/`Make_R` (mirror-image zero-padded code formatters), `NoTo3`
  (thousands-grouping only — **not** "spells the number in words" as `06-10-a.md` previously
  guessed; corrected there), and `Farsi_Date` — a function not previously known to exist (§A1).
- Full DDL for all 29 real application tables (10 more `CREATE TABLE`s in the dump are disposable
  developer scratch tables, not part of the schema).
- Constraints/indexes/FKs: **zero FKs, zero named CHECKs, two enforced composite PKs** (`Sarfasl`,
  `SahamdarConfig`), three single-column indexes (all on `Sarfasl`'s `S_IS_*` columns — see new
  item below), confirming §11's "add constraints the legacy system lacks" premise (C2) rather than
  contradicting it.
- Triggers: **exactly one**, previously undocumented — `Jens_Update` on `Anbar_Jens`
  (`AFTER DELETE, UPDATE`), archiving the pre-change row into `AnbarJens_B`. Does not consume
  `@@IDENTITY` (see new item below) — the specific hazard A2 was worried about does not materialize
  anywhere in this dump.
- Collation: no per-column `COLLATE` overrides anywhere (schema-level fact); server default
  collation itself is still unknown (needs a live connection, not exportable in a DDL script).

**Ruling: DECIDED for the DDL/procedure/trigger/constraint side — this part of A2 is closed.** What
remains is exclusively **data-shaped** and this dump cannot supply it (zero `INSERT` statements,
confirmed): enumeration value sets (§12.9), duplicate/orphan probes (§12.11), row counts/volume
(§12.16), and value-shape checks for dates (§12.1/§12.2). These are re-scoped, smaller asks for the
next live session — see `02-data-model/02-12-b.md` §12.17 for the precise remaining list. Also
still needed: server-level collation, and confirmation of whether `sys.views` and `sys.jobs` hold
anything (this dump's export scope did not include either).

→ `02-data-model.md §12` (`02-12-a.md`, `02-12-b.md`) has the full resolution, findings, and the
narrowed remaining artefact list.

### A3. Is the system single-company or multi-company? ⛔ shapes the whole architecture — DECIDED

**Ruling: multi-tenant, shared database.** Real `tenant_id` on every table, row-level security.
Every table in the DDL carries this in its uniqueness strategy.

`CO_ID` is a fiscal year. There is no tenancy model today. Multi-company was emulated by adding
`Base` rows, with the chart of accounts and party register shared globally across all of them and
no isolation whatsoever — that emulation is replaced by real tenancy.

**2026-08-19 correction:** this ruling was recorded here but had not been carried into the DDL —
`02-data-model.md §11` still described `accounts`, `parties`, `items`, `warehouses`,
`units_of_measure`, `product_grades`, `users`, `app_settings` and `organization` as global, with
`organization` literally constrained to one row for the whole product (`CHECK (id = 1)`) and
`users.username` / `fiscal_years.year` unique product-wide instead of per client. As specced, the
schema could serve exactly one client. Now fixed: every table in §11 carries `tenant_id`, enforced
with `ENABLE`/`FORCE ROW LEVEL SECURITY` and a policy against `current_setting('app.tenant_id')`,
set per-transaction in the Rust layer from the authenticated session (`10-target-architecture.md`
§2.4). This also resolves the two related open proposals in `08-platform-and-security.md` §14:
B7 (permissions are tenant-scoped via `user_id`, the permission catalogue itself stays global) and
E2 (tenants are now modelled explicitly via the new `tenants` table, rather than overloading
`Base.Co_ID`).

→ `07-parties-and-shareholders.md §1`; `02-data-model.md §11` (`02-data-model/02-11-a` through `02-11-h`)

### A4. Is shareholder equity in scope? — DECIDED

**Ruling: absorb into rebuild.** Build real equity logic — share counts, nominal values,
percentages, join/exit dates, profit allocation — migrating data out of `Saham.Dbo`
(`\\pesteh\SahamData\`) into the new system rather than reading it live.

`Sahamdar` is currently a person/legal-entity register, not a shareholder register: no equity
logic exists in this codebase today.

→ `07-parties-and-shareholders.md §5`, Q13

### A5. Who sets `Base.IsActive`? — DECIDED

**Ruling: manual admin action.** New system gets an explicit "close/archive year" action that
sets `IsActive = false`; an admin/accountant triggers it deliberately at year-end. Not automatic,
not dropped.

Nothing in the current codebase ever writes this flag.

→ `03-accounting-core.md`, Q3; `08-platform-and-security.md`, Q5

### A6. How is a new fiscal year actually set up? — DECIDED

**Ruling: keep chart of accounts global.** Matches current (if silent) behaviour — new-year
creation does not copy the chart of accounts (`MakeNewU.pas:129-150` stays commented-out logic,
not resurrected). No per-year snapshot.

### A7. Year-end ordering is unenforced — DECIDED

**Ruling: `NewFinalu` is the authoritative closing form, `EnteghalU` the carry-forward that must
run after it — enforce that order in code.** `FinalU` is superseded and dropped (see also C5,
merge/retire duplicate closing forms).

Previously: `EnteghalU` carried forward *every* account with a balance including profit-and-loss
accounts, silently assuming `NewFinalu` ran first, with no enforcement and three forms live
simultaneously.

→ `03-accounting-core.md §9`, Q4

### A8. Should unposted drafts appear in reports? — DECIDED

**Ruling: exclude drafts everywhere.** All reports and the tax-authority export filter to
posted-only. This resolves B17 (draft leakage into tax filings) as a side effect — no separate fix
needed there.

→ `04-reporting.md §3`, Q6, Q9

### A9. Is `CheckMaster` one cheque or a payment batch? ⛔ narrowed — DDL corroborates "batch", row distribution STILL BLOCKED, needs a populated DB

**2026-08-19: `Full_Script_14050527.sql` reviewed.** The DDL shape supports the existing "batch"
reading: `CheckMaster` (one bank-account, one amount/count pair per header row) plus `CheckDetail`
(N payee lines per header, linked by `CD_CMSSN`, unenforced — no FK, no index either side) is a
textbook header/detail structure, which a "one row per physical cheque" design would not need.
Neither table has a primary key at all, consistent with nothing in this schema being enforced by
default. **This is schema-shape evidence, not a data answer** — the dump has zero rows, so it
cannot say whether `CM_Count` is usually 1 (a batch-of-one, behaving like a single cheque in
practice) or usually >1 (a real batch). **Not a policy call — still settled only by**
`SELECT CM_Count, COUNT(*) … GROUP BY CM_Count` **against a populated database.** Do not treat this
as DECIDED on the strength of the DDL alone.

→ `06-treasury.md §12` (Q7); `06-03-received-versus-issued-cheques.md §3.2`

### A10. Can the `Tools` unit be obtained? — DECIDED (answer: no)

**Confirmed unavailable.** Source cannot be obtained. A1 (Jalali format) must be resolved purely
by querying the live DB and testing conversions against known dates — no shortcut via the
original library.

### A11. Does `Sahamdar_Edit` exist? ⛔ NEW, needs DB session — party CRUD's actual mechanism is now in question

`06-*`/`07-*` assumed party CRUD goes through a stored procedure named `Sahamdar_Edit` (the sole
gap in `02-data-model/02-12-a.md`'s original §12.3 procedure list). `Full_Script_14050527.sql`
contains 49 procedures and **`Sahamdar_Edit` is not one of them.** Either this dump predates its
creation, it was renamed, or party CRUD happens through inline `INSERT`/`UPDATE` in the Delphi data
module with no server-side procedure at all — which would also settle the original question this
item existed to answer ("does it create the matching `Sarfasl` node") in favour of "no, because
there's no procedure to do it in." Needs either a fresh dump or a direct read of the relevant
Delphi unit (`SahamdarEditU.pas`) to see whether it calls a named procedure or assembles SQL inline.

→ `02-data-model/02-12-a.md §12.3` item 8

### A12. Who or what maintains `Sarfasl.S_IS_Check` / `S_IS_Fish` / `S_IS_ADaryafti` / `S_IS_APArdakhti`? ⛔ NEW, needs DB session

These four columns are real (confirmed by DDL) and **three of the four are individually indexed** —
a deliberate DBA action, not an accident. Yet **no procedure in the 49-procedure dump writes to any
of them**, and the client-side assignments are already known to be commented out
(`Sarfasl_TakmilU.pas:76-83`). Indexed-but-unwritten-by-anything-in-this-dump is a stronger and
stranger signal than "the app doesn't write it" alone: something else — a SQL Agent job, a
different front-end, a manual process, or a since-removed feature whose index survived its
removal — may still be responsible, or the indexes are pure vestige. `sys.jobs`/`sysjobsteps` and
`SELECT COUNT(*) FROM Sarfasl WHERE S_IS_Check=1 OR S_IS_Fish=1 OR S_IS_APArdakhti=1 OR
S_IS_ADaryafti=1` would settle whether any historical row even holds a meaningful value. §11 still
defaults to dropping these columns (per the existing §12.10 item 7 reasoning); this item is about
whether that's safe to do without checking with the operator first.

→ `02-data-model/02-12-a.md §12.3` item 7, `§12.6`; `02-data-model/02-12-b.md §12.10` item 7

### A13. Does the legacy database use one physical table per fiscal year for some data? ⛔ NEW, needs DB session — changes the migration's data-discovery scope

The dump contains `Tah1403` (a byte-for-byte structural clone of `Moein`, including the same
extended-property comments) and `mandeh_1404` (a near-clone of `Anbar_FactorD`) — **year-suffixed
physical tables**, not `CO_ID`/`AFD_COID`-scoped rows inside the shared `Moein`/`Anbar_FactorD`
tables. If this is a recurring pattern (one such table per fiscal year, for some subset of the
schema), the migration's "find all the data" step needs to enumerate year-suffixed tables by
pattern, not just read the tables named in `02-*`'s DDL — a materially different and larger data
discovery scope than assumed anywhere in the current documents. `SELECT name FROM sys.tables WHERE
name LIKE '%1403' OR name LIKE '%1404' OR name LIKE 'Tah%' OR name LIKE 'mandeh_%';` on the live
server, plus asking the business directly whether this is a known year-end archival practice, would
settle it.

→ `02-data-model/02-12-a.md §12.5`; `02-data-model/02-12-b.md §12.16`

### A14. What happened to the `Kinds` table? ⛔ NEW, needs DB session

`02-data-model/02-12-a.md`'s original §12.5 expected `Kinds` to exist (confidence B/C in the
master table list, §2.2) alongside `TCheck`, `Anbar_Vahed`, `TolidMaster`, and others. All of those
were confirmed present except `Kinds` and `TolidMaster` — and `TolidMaster` was already suspected
to be a placeholder, so its absence was expected. `Kinds`'s absence is not expected by anything in
the current documentation. Either it was dropped, renamed, never existed and the confidence-B/C
listing was itself wrong, or it lives in a sibling database. Needs a direct question to the
business/operator, since the dump alone cannot distinguish "never existed" from "existed once."

→ `02-data-model/02-12-a.md §12.5`; `02-data-model/02-12-b.md §12.14`

### A15. Nineteen stored procedures with no call site anywhere in the Delphi source — dead, or called from elsewhere? ⛔ NEW, needs DB session

`Full_Script_14050527.sql` has 49 procedures; 30 match names already known from the app (one of
those, `Sahamdar_Edit`, is itself missing — A11). The remaining 19 —
`Anbar_AddToFactor2`, `Anbar_Report2`, `DMoein_Make_Update`, `Jari_Remind`, `Kol_Taraz1`,
`Kol_Taraz2`, `Make_Directory`, `MakeSanad_CheckBank`, `MakeSanad_Fishvarizi`, `Moein_Delete_SSN1`,
`Moein_Set_Tx`, `Moein_sumAsnad`, `Moein_Taraz1`, `Moein_Taraz2`, `Moein_Taraz2_Head`,
`Moein_Taraz3`, `notify_users`, `Pol_Select`, `Taraz_6Sotoni` (the orphaned single-`o` twin of the
real `Taraz_6Sotooni`) — **do not appear anywhere in the `.pas`/`.dfm` source tree** (grepped
individually). Three (`notify_users`, `Make_Directory`, `Pol_Select`) look like generic DBA/
integration utilities unrelated to the app proper. Several others (`Kol_Taraz1`/`Kol_Taraz2`,
`Moein_Taraz1`/`2`/`3`) read like earlier drafts of procedures that *are* called
(`Taraz_6Sotooni`). `DMoein_Make_Update` (dated `1403/02/24` in its own header comment) looks like
it could be the intended fix for `06-treasury.md` defect B13 (cheque collection never builds its
`DMoein` header) but under a different name than any call site references. **Before deciding what
to port (Group C default is port-as-is; these were never in scope for that because nothing was
known to call them), confirm none of them is invoked by a SQL Agent job or a second front-end** —
`sys.jobs`/`sysjobsteps` from the live server, not in this schema-only dump.

→ `02-data-model/02-12-a.md §12.3`

### A16. The dump's `Sahamdar` DDL doesn't match `02-11-c.md`'s `parties` design — which is right? ⛔ NEW, needs a fresh dump or source recheck

`02-data-model/02-11-c-ddl-parties-and-accounts.md`'s `parties` table (derived from `Sahamdar`) maps
five legacy columns — `S_Shanas`, `S_Phone`, `S_Siba`, `S_ShabaNo`, `S_Aks` — that **do not appear
anywhere in `Full_Script_14050527.sql`'s `Sahamdar` DDL.** The dump's actual columns include two
the target design never mapped at all: `S_Melli varchar(20)` and `S_keshavarzi varchar(20)`
(possibly a national/legal-entity ID and an agricultural-union membership number respectively —
unconfirmed). Several types also differ from what was assumed: `S_BDate`/`S_SDate` are
`varchar(10)` (long-form Jalali), not the `char(8)` short-form both `02-11-c.md` and
`02-data-model/02-12-a.md §12.1` assumed; `S_IDNO` is `bigint`, not `int`; `S_CodeMelli`/
`S_CodePosti` are `varchar(12)`, not `char(10)`. **Do not silently reconcile either direction** —
this could mean the dump predates a schema change (the five missing columns were added later), or
it could mean the target design's assumptions (built from Delphi field metadata, not DDL) were
simply wrong. A fresh dump against the current production schema, or a direct re-check of
`SahamdarEditU.pas`'s full field list, settles it.

→ `02-data-model/02-11-c-ddl-parties-and-accounts.md`

### A17. An undocumented trigger silently audits `Anbar_Jens` changes — should the rebuild replicate this? ⛔ NEW, needs a ruling (not blocked on data — behaviour is fully known)

`Jens_Update` (`AFTER DELETE, UPDATE` on `Anbar_Jens`) copies the pre-change row into `AnbarJens_B`
on every edit or delete of an inventory item — a server-side change-history mechanism that exists
today, silently, and that **no `02-*` or `05-inventory.md` document currently mentions.**
`AnbarJens_B` is confirmed never read by the Delphi application anywhere (grepped) — it is a
write-only audit trail nobody in the app consumes, though an operator could query it directly in
SSMS. This is not blocked on further data — the trigger's full body is captured and its behaviour
is completely known. What needs a ruling: **does the rebuild need an equivalent item-change-history
table**, given one has apparently been silently relied upon (or at least silently produced) in
production, or is this dead weight safe to drop like the `S_IS_*` columns (A12)?

→ `02-data-model/02-12-a.md §12.7`

---

## Group B — Confirmed defects: replicate, or fix? — DECIDED: fix all 24, B25 NEW (needs the same sign-off)

**Ruling: fix every defect below, including the ones that change visible behaviour** (B4, B5, B6,
B11, B12, B14, B15, B17, B19 — ledger opening-balance handling, cheque state codes, report
permissions, endorsement, pistachio deduction calculator). None are replicated as-is.

**2026-08-19: B25 added**, found by reading `Anbar_ReportKharidForoosh`'s body in
`Full_Script_14050527.sql` — confirmed via the procedure text itself, not inferred from caller
code. It almost certainly falls under the same "fix every defect" ruling as B1-B24, but is listed
separately because it postdates that ruling and deserves the same explicit sign-off the other 24
got, per this file's own standard.

Where a defect has corrupted historical data (see "Data already affected?" column), fixing the
code is not sufficient — each of those needs a data-remediation pass, not just a code fix.

Each of these is live in production code. The specification documents them as-is.

| # | Defect | Where | Data already affected? |
|---|---|---|---|
| B1 | **Purchase and opening-stock vouchers do not balance.** Input VAT disabled behind `if false then`; discount posted to the wrong side. Out by `2·Kasr − Maliat`. No balance check exists in `MakeSanadU`. | `MakeSanadU.pas:289,424` | Yes — every affected voucher |
| B2 | **Production and transfer documents generate no accounting entry at all** (`' Not implemented yet. '`). | `05-inventory.md §10` | Yes — missing entries |
| B3 | **A report rewrites the movement table**: `UPDATE Anbar_FactorD SET AFD_Customer = (…)` with **no `WHERE`**, every run. | `Anbar_Amalkard.pas:168,189,215` | Yes, silently |
| B4 | **Consolidated ledger double-counts its opening balance** — the opening leg omits `M_kind=1`, the movement leg includes it. | `TMoein`, `04-reporting.md §3` | Display only |
| B5 | **`BedBes` splits the opening period at `<= D1`** while every other ledger uses `< D1` — a silent one-day reconciliation break. | `04-reporting.md §3` | Display only |
| B6 | **Daftar Kol shows nothing** until someone manually runs "ساخت روزنامه", and those rows are written in draft state. | `04-reporting.md §3` | Design question |
| B7 | **Invoice counterparty validation is unreachable** — `if not S_Bed.tag=0` parses as `(not Tag)=0`. Invoices save with `AF_Customer=0`. | `AnbarFactorU.pas:579` | Yes — orphaned invoices |
| B8 | **Un-posting deletes `M_Id in (32,33,35)` but posting creates `31..39`** — opening-stock lines are never removed; a document can be posted twice. | `05-inventory.md §5` | Yes — duplicate lines |
| B9 | **`M_Id=34` orphans are permanently unremovable** — the reverse-voucher handler has an empty body and `SodoorSanadU` has no branch for `FM_ID=14`. | `05-inventory.md §5.3` | Yes |
| B10 | **Cheque bounce leaves history contradicting the master row** — `DCheck2` records state 2 while the master goes to state 1. | `CheckBargashtu.pas:209,214` | Yes |
| B11 | **Bounced and never-deposited cheques share state code 1**, distinguished only by free text. State 3 is unreachable. | `06-treasury.md §2` | Yes — unrecoverable distinction |
| B12 | **Cheque delete is a no-op** — bare `Exit;` above the real code. | `CheckListDU.pas:457` | No |
| B13 | **Cheque collection never builds its voucher header** — the only screen missing `DMoein_Make`. | `06-treasury.md §2` | Yes |
| B14 | **Cheque endorsement does not exist** despite having columns (`S_Z*`, zero reads and writes). | `06-treasury.md §4` | No |
| B15 | **Every cheque list filter is unreachable**; the due-date aging query never runs. | `06-treasury.md §11` | No |
| B16 | **`RoyatJU` drops and recreates a permanent table** (`temp_RJ_<userId>`) on every run. | `04-reporting.md §1` | Operationally risky |
| B17 | **Tax-authority Excel export includes unposted drafts** and mislabels the voucher-number column as `ردیف`. | `ToExcelDaraeiU`, `04-reporting.md §7` | Yes — filings |
| B18 | **`SahamdarConfig.SC_Tik` is a globally mutated scratch column** — genuine multi-user corruption. | `07-parties-and-shareholders.md`, Q16 | Yes |
| B19 | **The pistachio deduction calculator is unreachable** — its panel is `Visible=False` and never shown; its Save button has no handler. The formula is documented as the domain spec regardless. | `05-inventory.md §8` | Feature never shipped |
| B20 | **Hidden privilege escalation**: Ctrl+Alt drag one button onto another re-enables the disabled new-company action. | `Mainu.pas:501-532` | Security |
| B21 | **Cancel applies the change** on the fiscal-year switcher. | `ChangesU.pas:78` | Yes |
| B22 | **Hard-coded `userId = 68`** in party save paths. | `07-parties-and-shareholders.md` | Yes — wrong attribution |
| B23 | **Any user can redesign or load report layouts** — `pbEdit`/`pbLoad` are reachable from every preview. | `04-reporting.md §6` | Security |
| B24 | **Five report menu items have no permission check at all** (`Report4/5/6/8`, `_Report9`); `CardJariU.Report2` is ungated while `Report1` is gated. | `04-reporting.md §1` | Security |
| B25 | **`Anbar_ReportKharidForoosh` (purchase/sales report) has no fiscal-year parameter and no `AFD_Coid` predicate anywhere in its body** — confirmed by reading the stored procedure text: it filters only on `AFD_Date` range and `AFD_Type`, so it returns rows from every fiscal year that falls in the date range, not just the current one. | `Full_Script_14050527.sql`; `02-data-model/02-12-b.md §12.12` | Yes — every report run to date, if run across a date range spanning more than one fiscal year |

B1, B2, B7, B8, B9, B10, B13 and B18 are data-integrity defects needing a documented
data-remediation pass alongside the code fix. B3, B16, B20, B21, B22, B23, B24 and B25 are outright
hazards. B4, B5, B6, B11, B12, B14, B15, B17 and B19 are behaviour changes users will *notice* —
now approved as part of the "fix all 24" ruling above. **B25 needs its own explicit sign-off** (see
note above the table) but is written up the same way for when that happens.

---

## Group C — Proposed improvements — DECIDED: all approved

Roughly 90 individual suggestions are quarantined in the `PROPOSED IMPROVEMENTS` sections of the
domain documents (22 in `02`, 36 in `04`, plus sets in `03`, `05`, `06`, `07`, `08`). They are not
repeated here in full. The themes, all now approved:

### C1. Security — DECIDED: full rebuild

Password hashing (Argon2id), server-side authorization on every endpoint rather than
presentation-only checks, credentials in environment/secret storage rather than a client-readable
file, an audit trail, and session management. All five pieces approved — the current design has
no equivalent to port, so nothing here was a "change" to weigh.

### C2. Schema integrity — DECIDED: add all constraints, audit data first

Foreign keys, `NOT NULL`s, and `CHECK` constraints the legacy system never enforced — including
the riskiest ones (invoice total identity, cheque-number uniqueness) previously flagged `[NEW]`
and commented out. Run a data audit pass first to find what historical data would violate each
constraint, then enable all of them.

### C3. Denormalisation cleanup — DECIDED: derive, don't store

The legacy schema stores denormalised copies (`StateName`, `BedName`, `BesName`, `DMoein` header
totals) that can and do drift from their source. These are computed at read time instead of
stored. Changes nothing users see; removes a class of silent inconsistency (see also B4/B5,
consolidated-ledger opening-balance bugs in the same area).

### C4. Dead code not carried forward — DECIDED: drop

14 units plus one orphan form are unreferenced or non-compiling — not ported. Listed with
evidence in `09-unit-index.md §3`.

### C5. Merge candidates — DECIDED: merge all

`Sarfasl_SelectU` vs `SelectSarfasl`; `DMoein` vs `TMoein`; the three closing forms
(`EnteghalU` / `FinalU` / `NewFinalu`, see A7 — `NewFinalu` + `EnteghalU` retained, `FinalU`
dropped). Each pair is consolidated into one implementation. Users will notice fewer near-identical
menu entries; behaviour stays equivalent.

---

## What's left

All policy decisions are made (2026-08-18): A3–A8, A10, Group B (B1-B24), Group C. See rulings
inline above.

**2026-08-19: a schema-only SQL Server dump (`Full_Script_14050527.sql` — DDL, procedure/function/
trigger bodies, zero data rows) was reviewed against every open question in `02-data-model.md §12`
and `06-treasury.md §12`.** This closed **A2** (DDL/procedure side) outright, narrowed **A1** and
**A9** further, and surfaced seven new items (A11-A17) plus one new confirmed defect (B25) that need
the same sign-off the rest of this file already got.

Two items remain genuinely blocked on a **populated** database — schema alone cannot resolve them,
no further Q&A does either:

1. **A1** — narrowed twice now: the new schema is settled (real date columns, Jalali is
   display-only), and the server-side conversion algorithm actually in use (`dbo.Farsi_Date`) is
   now known. What's left is testing it (and the two Delphi algorithms) against real historical
   date strings — needs rows, this dump has none.
2. **A9** — still exactly one query: `SELECT CM_Count, COUNT(*) … GROUP BY CM_Count` against a
   populated `CheckMaster`. The DDL shape corroborates "batch" but cannot supply the distribution.

Seven items are newly blocked, surfaced by this session, not present in the original three:

3. **A11** — does `Sahamdar_Edit` exist under this or another name?
4. **A12** — who/what maintains the three indexed-but-unwritten `Sarfasl.S_IS_*` columns?
5. **A13** — does the legacy DB use one physical table per fiscal year for some data
   (`Tah1403`, `mandeh_1404`), and if so, for how much of the schema?
6. **A14** — does `Kinds` exist anywhere, or was its confidence-B/C listing wrong?
7. **A15** — are any of the 19 unreferenced-by-source procedures invoked by a job or another
   front-end?
8. **A16** — does the dump's `Sahamdar` DDL (missing 5 assumed columns, 2 unmapped extra ones,
   several type mismatches) reflect current production, or does `02-11-c.md`'s `parties` design
   need correcting — needs a fresh dump or a source recheck, not a policy call.
9. **A17** — not blocked on data, but needs a ruling: replicate the silently-discovered
   `Anbar_Jens` change-history trigger, or drop it?

Once a populated database session happens for items 1-2, and the remaining items get either a data
answer (3-8) or a ruling (9, and B25), the schema and migration can proceed using the rulings
already recorded here plus whatever these resolve to.
