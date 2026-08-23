# Migration readiness

**Purpose:** the day a populated legacy SQL Server database becomes reachable, whoever has access to
it — with no other context than this document — should be able to run every query below, copy the
output back, and that closes every item `specs/11-open-decisions.md` Group A still has open. This
document does not migrate anything itself; nothing here writes to the legacy database (every query
is read-only), and no ETL runs until every A-item below is resolved.

Cross-reference: `docs/phase-7-hardening-and-cutover.md §7.2`; `specs/11-open-decisions.md` Group A;
`specs/10-target-architecture.md §4`.

---

## 0. What's already settled — do not re-derive these

So a future reader doesn't spend a session re-answering questions that are already closed:

| Item | Status |
|---|---|
| A2 (DDL/procedure/trigger/constraint side) | **Closed 2026-08-19** by a schema-only dump (`Full_Script_14050527.sql`). All 49 procedures, 4 UDFs, 29 real tables' DDL, 0 FKs/0 named CHECKs/2 composite PKs, exactly 1 trigger — all captured. Only the *data-shaped* remainder of A2 (enum value sets, duplicate/orphan probes, row counts, collation) still needs a live session — folded into the queries below, not a separate pass. |
| A3 (single- vs multi-company) | **Decided:** multi-tenant, shared database, real `tenant_id` + RLS on every table. Already built (Phase 1.1). Nothing to migrate-decide. |
| A4 (shareholder equity in scope) | **Decided:** absorb into rebuild, real equity logic. Already built (Phase 3.3) as new logic — no legacy equivalent to migrate *from* for the equity math itself, only the underlying `Sahamdar` party rows (see A11/A16 below) and whatever's in the separate `Saham.Dbo` product at `\\pesteh\SahamData\` if that data still needs pulling in. |
| A5 (who sets `Base.IsActive`) | **Decided:** manual admin action, already built (Phase 1.5's fiscal-year close). No legacy write path existed to migrate. |
| A6 (new fiscal year setup) | **Decided:** chart of accounts stays global, no per-year copy/snapshot. Already built. |
| A7 (year-end ordering) | **Decided:** `NewFinalu` (close) must run before `EnteghalU` (carry-forward), enforced in code. Already built (Phase 2.7) — `books_closed_voucher_id` gate. |
| A8 (drafts in reports) | **Decided:** exclude drafts everywhere. Already built throughout Phase 6. |
| A10 (`Tools` unit source) | **Decided, unavailable.** A1 must be resolved purely from live-database evidence — no shortcut through the original Delphi library exists. |
| A17 (undocumented `Jens_Update` trigger / `AnbarJens_B`) | **Decided 2026-08-23: drop.** Not replicated — real audit logging (Phase 1.4) covers future needs. Nothing to migrate; `AnbarJens_B` itself is never read by the legacy app and does not need to come across. |

Everything below is genuinely still open and needs the live database.

---

## 1. A1 — What Jalali date format is physically stored?

**Known so far** (from the schema-only dump, `specs/02-data-model/02-12-a.md §12.1`): three
candidate algorithms exist — `dbo.Farsi_Date` (server-side, naive `%4` leap-year approximation,
epoch offset `ADate-78`), the dead `TUtil.FarsiDate`/`DecodedateF` (same naive design, offset
`ADate-80`, confirmed unreachable — skip), and `TDM.MiladiToShamsi` (structurally the one correct
Persian-calendar algorithm of the three). Hand-typed dates went through the closed-source
`Tools.TEditDate`/`TFullDate` (A10 — permanently unavailable), so a fourth, unverifiable algorithm
may be responsible for some historical values. The new schema (§0 above, already built) stores real
`date` columns — Jalali is display-only going forward. What's still needed is purely which algorithm
correctly decodes *historical* stored values, for the one-time migration.

```sql
-- 1a. Sample raw stored values for every date-shaped column across the tables the domain specs
--     name as carrying Jalali strings (S_Date, S_DateS, S_BDate, S_SDate, DM_Date, AF_Date, etc.).
--     Run per table; adjust the column list to match what §2 below's discovery query finds.
SELECT TOP 50 S_Date, S_DateS FROM DCheck ORDER BY S_SSN DESC;
SELECT TOP 50 DM_Date FROM DMoein ORDER BY DM_SSN DESC;
SELECT TOP 50 AF_Date FROM Anbar_Factor ORDER BY AF_SSN DESC;
SELECT TOP 50 S_BDate, S_SDate FROM Sahamdar ORDER BY S_SSN DESC;
-- ...repeat for every table §2's discovery query surfaces.

-- 1b. A known real-world anchor: find the earliest and latest voucher dates, then ask the business
--     "what date did you actually enter for the earliest/latest voucher in the system" to get one
--     verified (stored-string, real-Gregorian-date) pair to test every algorithm against.
SELECT MIN(DM_Date) AS earliest, MAX(DM_Date) AS latest FROM DMoein;

-- 1c. A Nowruz-boundary probe: Persian new year lands on Mar 20/21 Gregorian, and the three
--     algorithms' naive leap-year math is most likely to disagree near a year boundary. Pull every
--     row whose date string decodes (under any candidate algorithm) to within a few days of a known
--     Nowruz, and check them by hand against whatever supporting paperwork exists for that period.
SELECT DM_SSN, DM_Date FROM DMoein WHERE DM_Date LIKE '%/01/01' OR DM_Date LIKE '%/12/29'
  OR DM_Date LIKE '%/12/30';
```

Feed the sampled strings and the anchor date into all three candidate algorithms (reimplemented
from `specs/02-data-model/02-12-a.md §12.1`'s captured bodies) and see which one reproduces the
known-correct anchor and the Nowruz-boundary dates without drift. That's the migration's decoder.

---

## 2. A9 — Is `CheckMaster` one cheque or a payment batch?

Schema shape (header/detail, no PK either side) already favours "batch" — Phase 4.5 built
`cheque_payment_batches`/`cheque_payment_batch_lines` on that reading, documented as *inferred, not
confirmed*. This query is the confirmation:

```sql
SELECT CM_Count, COUNT(*) AS header_rows
FROM CheckMaster
GROUP BY CM_Count
ORDER BY CM_Count;

-- Cross-check: does every CheckDetail row's CD_CMSSN actually resolve, and does per-header line
-- count match the cached CM_Count?
SELECT cm.CM_SSN, cm.CM_Count, COUNT(cd.CD_CMSSN) AS actual_lines
FROM CheckMaster cm
LEFT JOIN CheckDetail cd ON cd.CD_CMSSN = cm.CM_SSN
GROUP BY cm.CM_SSN, cm.CM_Count
HAVING COUNT(cd.CD_CMSSN) <> cm.CM_Count;
```

If `CM_Count` is overwhelmingly 1, the batch model still works (a batch-of-one) but the UI/import
should default to single-cheque entry. If it's usually >1, the built batch model is already correct
as-is. Either way, the second query's mismatches (if any) tell the ETL where `CM_Count` cannot be
trusted and a batch's real line count must be recounted from `CheckDetail` directly.

---

## 3. A11 — Does `Sahamdar_Edit` exist?

`Sahamdar_Edit` is not among the 49 procedures in the schema-only dump. Confirm which is true on a
live server, and separately check the Delphi source:

```sql
-- 3a. Does it exist NOW, even if the schema-only dump predates it?
SELECT name, create_date, modify_date FROM sys.procedures WHERE name LIKE '%Sahamdar%';
```

Also: read `SahamdarEditU.pas`'s save handler directly — does it call a named procedure, or build
`INSERT`/`UPDATE` SQL inline in the Delphi data module? If inline, A11's original question ("does
party creation implicitly create a matching `Sarfasl` node") is answered "no, nothing does that
implicitly" and Phase 3.1's built party model (parties independent of accounts, linked by explicit
`party_id`, no implicit account creation) needs no change — it already assumed the safer answer.

---

## 4. A12 — Who maintains `Sarfasl.S_IS_Check`/`S_IS_Fish`/`S_IS_APArdakhti`/`S_IS_ADaryafti`?

```sql
-- 4a. Does any historical row hold a meaningful (non-zero/non-null) value at all?
SELECT COUNT(*) AS any_flag_set
FROM Sarfasl
WHERE S_IS_Check = 1 OR S_IS_Fish = 1 OR S_IS_APArdakhti = 1 OR S_IS_ADaryafti = 1;

-- 4b. Per-flag breakdown, if 4a is non-zero.
SELECT
  SUM(CASE WHEN S_IS_Check = 1 THEN 1 ELSE 0 END)      AS is_check,
  SUM(CASE WHEN S_IS_Fish = 1 THEN 1 ELSE 0 END)       AS is_fish,
  SUM(CASE WHEN S_IS_APArdakhti = 1 THEN 1 ELSE 0 END) AS is_ap,
  SUM(CASE WHEN S_IS_ADaryafti = 1 THEN 1 ELSE 0 END)  AS is_ar
FROM Sarfasl;

-- 4c. Is anything external writing them — a SQL Agent job, a linked server, a second front-end?
SELECT j.name AS job_name, s.step_name, s.command
FROM msdb.dbo.sysjobs j
JOIN msdb.dbo.sysjobsteps s ON s.job_id = j.job_id
WHERE s.command LIKE '%S_IS_Check%' OR s.command LIKE '%S_IS_Fish%'
   OR s.command LIKE '%S_IS_APArdakhti%' OR s.command LIKE '%S_IS_ADaryafti%'
   OR s.command LIKE '%Sarfasl%';
```

If 4a comes back 0 and 4c finds nothing, the columns are confirmed pure vestige — the rebuild's
existing decision to drop them (moved to `base_config`, Phase 2.1) needs no revisiting and no data
carries forward. If 4a is non-zero, whatever rows have flags set need a decision on where that
signal goes in the new schema before those specific rows migrate.

---

## 5. A13 — Year-suffixed physical tables (`Tah1403`, `mandeh_1404`, …)?

```sql
SELECT name FROM sys.tables
WHERE name LIKE '%1403' OR name LIKE '%1404' OR name LIKE '%1405' OR name LIKE '%1406'
   OR name LIKE 'Tah%' OR name LIKE 'mandeh_%'
ORDER BY name;

-- For each match, get row count and a same-shape comparison against its presumed parent table
-- (Tah1403 vs Moein; mandeh_1404 vs Anbar_FactorD) to confirm the "structural clone" pattern holds
-- generally and isn't specific to the two already found.
SELECT COUNT(*) FROM Tah1403;       -- compare shape/columns against Moein
SELECT COUNT(*) FROM mandeh_1404;   -- compare shape/columns against Anbar_FactorD
```

Also ask the business directly: is a year-end "freeze a copy of the ledger/stock movement table"
step a known manual practice? If confirmed and recurring, the migration's table-discovery step must
enumerate by this naming pattern across every fiscal year found, not just read the tables the domain
specs name — a larger scope than assumed anywhere in `specs/`.

---

## 6. A14 — What happened to the `Kinds` table?

No query can distinguish "never existed" from "existed once, dropped" from "lives in a sibling
database" — this one needs a direct question to the business/operator, not a probe. If confirmed
absent and never existed, Phase 5.1's `pistachio_grades` table (seeded from the 7-value enumeration
recovered from source comments, not from a live `Kinds` table) needs no reconciliation. If a `Kinds`
table (or equivalent) is found on a live server, diff its rows against the 7-value enumeration before
trusting the seed data used since Phase 5.1.

```sql
-- Run anyway, in case a differently-cased or differently-named sibling exists.
SELECT name FROM sys.tables WHERE name LIKE '%Kind%';
```

---

## 7. A15 — Nineteen stored procedures with no call site in the Delphi source

```sql
SELECT j.name AS job_name, s.step_name, s.command
FROM msdb.dbo.sysjobs j
JOIN msdb.dbo.sysjobsteps s ON s.job_id = j.job_id
WHERE s.command LIKE '%Anbar_AddToFactor2%' OR s.command LIKE '%Anbar_Report2%'
   OR s.command LIKE '%DMoein_Make_Update%' OR s.command LIKE '%Jari_Remind%'
   OR s.command LIKE '%Kol_Taraz%'          OR s.command LIKE '%Make_Directory%'
   OR s.command LIKE '%MakeSanad_CheckBank%' OR s.command LIKE '%MakeSanad_Fishvarizi%'
   OR s.command LIKE '%Moein_Delete_SSN1%'  OR s.command LIKE '%Moein_Set_Tx%'
   OR s.command LIKE '%Moein_sumAsnad%'     OR s.command LIKE '%Moein_Taraz%'
   OR s.command LIKE '%notify_users%'       OR s.command LIKE '%Pol_Select%'
   OR s.command LIKE '%Taraz_6Sotoni%';

-- Also check for a second front-end / linked-server caller by usage stats (SQL Server keeps these
-- only since last service restart, so treat "0" as inconclusive, not proof of dead).
SELECT OBJECT_NAME(object_id) AS proc_name, last_execution_time, execution_count
FROM sys.dm_exec_procedure_stats
WHERE OBJECT_NAME(object_id) IN (
  'Anbar_AddToFactor2','Anbar_Report2','DMoein_Make_Update','Jari_Remind','Kol_Taraz1','Kol_Taraz2',
  'Make_Directory','MakeSanad_CheckBank','MakeSanad_Fishvarizi','Moein_Delete_SSN1','Moein_Set_Tx',
  'Moein_sumAsnad','Moein_Taraz1','Moein_Taraz2','Moein_Taraz2_Head','Moein_Taraz3','notify_users',
  'Pol_Select','Taraz_6Sotoni'
);
```

If any of these turn out live (a SQL Agent job or nonzero recent `execution_count`), read that
procedure's body before deciding whether the rebuild needs an equivalent — particularly
`DMoein_Make_Update`, flagged as a plausible (if unconfirmed) fix for B13 (cheque collection never
builds its voucher header, already fixed structurally in Phase 4.2 regardless of what this query
finds).

---

## 8. A16 — `Sahamdar` DDL mismatch (dump vs. `specs/02-data-model/02-11-c-ddl-parties-and-accounts.md`)

```sql
-- Full current column list, types and nullability — compare directly against 02-11-c.md's mapping
-- table and against api/migrations/0009_parties.sql's actual `parties` columns.
SELECT COLUMN_NAME, DATA_TYPE, CHARACTER_MAXIMUM_LENGTH, IS_NULLABLE
FROM INFORMATION_SCHEMA.COLUMNS
WHERE TABLE_NAME = 'Sahamdar'
ORDER BY ORDINAL_POSITION;

-- Specifically: do S_Shanas / S_Phone / S_Siba / S_ShabaNo / S_Aks exist on THIS server (the
-- schema-only dump's Sahamdar DDL didn't have them)? And do S_Melli / S_keshavarzi exist (the dump
-- HAD these, unmapped by anything built so far)?
SELECT COUNT(*) AS has_shanas   FROM sys.columns WHERE object_id = OBJECT_ID('Sahamdar') AND name = 'S_Shanas';
SELECT COUNT(*) AS has_melli    FROM sys.columns WHERE object_id = OBJECT_ID('Sahamdar') AND name = 'S_Melli';
SELECT COUNT(*) AS has_keshavarzi FROM sys.columns WHERE object_id = OBJECT_ID('Sahamdar') AND name = 'S_keshavarzi';

-- Sample values for the two unmapped-if-present columns, to judge their real business meaning
-- before deciding where (if anywhere) they map onto Phase 3.1's built `parties` table.
SELECT TOP 20 S_SSN, S_Melli, S_keshavarzi FROM Sahamdar WHERE S_Melli IS NOT NULL OR S_keshavarzi IS NOT NULL;
```

If `S_Melli`/`S_keshavarzi` are confirmed live and populated, they most likely map onto
`parties.national_id` (already built) and a currently-unmapped agricultural-union-membership field
respectively — but confirm the actual values look like a national ID / union number before assuming
that, per this item's own "do not silently reconcile either direction" instruction. If
`S_Shanas`/`S_Phone`/`S_Siba`/`S_ShabaNo`/`S_Aks` are confirmed genuinely absent on a live server
(not just missing from the one schema-only dump), Phase 3.1's decision to drop them from `parties`
(`api/migrations/0009_parties.sql`'s header comment) is confirmed correct and needs no revisiting.

---

## 9. The six-step migration procedure (`specs/10-target-architecture.md §4`), restated with query cross-references

Not runnable until every item in §§1-8 above comes back answered — a partial answer (e.g. A1
resolved but A13 still open) means step 3 below cannot start safely, since the ETL's table-discovery
scope is still unknown.

1. **Dump the live schema and procedure bodies.** — Superseded in part: already have this
   (`Full_Script_14050527.sql`, 2026-08-19, DDL/procedure/trigger/constraint side, A2). Re-dump only
   if §7's job-search or §8's column check finds the live schema has since diverged from that dump.
2. **Determine the Jalali storage format; validate the conversion against a full-table scan.** —
   §1 above (A1). Do this FIRST — every date column in every other table's data depends on it.
3. **ETL into the new schema with `[NEW]` constraints disabled.** — Table-discovery scope depends on
   §5 (A13, year-suffixed tables) and §6 (A14, `Kinds`) being answered first; the batch-vs-single
   mapping for issued cheques depends on §2 (A9); the exact `parties` column mapping depends on §8
   (A16); whether party creation needs a synthetic matching-account step depends on §3 (A11).
4. **Run the Phase 7.1 audit queries** (`docs/constraint-audit-queries.sql`) **against the imported
   data**, `[NEW]` constraints still disabled — every query in that file returns a violation count;
   nonzero counts here are expected on first run against real historical data, unlike the zero counts
   confirmed against this fresh-built database in Phase 7.1.
5. **Remediate, then enable constraints one at a time** — for each nonzero count from step 4, fix or
   exclude the offending rows, then `ALTER TABLE ... VALIDATE CONSTRAINT` (or add it fresh) before
   moving to the next constraint. Cheque-number uniqueness stays deliberately unenforced per Phase
   7.1's own finding (needs the §13.13 banks model, never built) unless that decision is revisited
   separately.
6. **Reconcile**: every trial balance, ledger and stock balance must match the legacy system's output
   for the same parameters, using the reconciliation harness Phase 7.5 builds — or the difference
   must be traceable to an approved Group B fix (`specs/11-open-decisions.md` Group B's 25-item
   table, all approved for fixing, not replication). This is the acceptance test for the whole
   migration — do not sign off on steps 1-5 alone.
