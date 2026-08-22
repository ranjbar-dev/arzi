_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 12.9 Enumerations whose full value set is unknown

**Still open — needs data.** `Full_Script_14050527.sql` is schema-only: **zero `INSERT` statements**
in the whole file, confirmed by `grep -c "^INSERT"` → `0`. None of the queries in this section can
be run against it; every row below is exactly as open as before. The DDL does resolve one adjacent
structural question: `02-data-model/02-12-a.md` §12.3 item 4 (`Anbar_AddToFactor`'s body) confirms
`Anbar_Jens.AJ_ID` really is a foreign key into `Anbar_Config.AC_ID` (looked up and dereferenced in
the procedure), so the "is `AJ_ID` really `Anbar_Config`'s key" half of the `Anbar_Jens.AJ_ID` row
below is answered — only the actual warehouse-id value set still needs data.

**P1.** The source shows which values are *written*; it never shows the full set that *exists*
(§3.5 item 10). Every one of these becomes a PostgreSQL `CHECK` or enum in §11, and a value outside
the assumed set will fail the migration.

| Column | Values known from source | Question | Query |
|---|---|---|---|
| `Moein.M_ID` (source module) | 1–9, 15, 21, 22, 23, 24, 25, 26, 27–29, 34, 35, 41 (§2.7) — 15 and 35 **unidentified** | what are 15 and 35? what else exists? | `SELECT M_ID, COUNT(*) FROM Moein GROUP BY M_ID ORDER BY 1;` |
| `Moein.M_Tx` / `DMoein.DM_Tx` | 0, 1, 2 (§9.7) | does `3` exist? `Taraz_6Sotooni.@Sabt` defaults to `3` | `SELECT M_Tx, COUNT(*) FROM Moein GROUP BY M_Tx;` |
| `Moein.M_Kind` / `DM_Kind` | 1, 2 (§2.7) | any third journal kind? | `SELECT M_Kind, COUNT(*) FROM Moein GROUP BY M_Kind;` |
| `DCheck.S_State` | 1, 2, 4, 5 written; **3 never written** and state 1 is overloaded (`06-treasury.md` §2.1) | do state-3 rows exist historically? | `SELECT S_State, S_StateName, COUNT(*) FROM DCheck GROUP BY S_State, S_StateName;` |
| `DCheck2.S_State` | records `2` for a bounce while the master row goes to `1` — a **known disagreement** (`06-treasury.md` §2.1) | how many rows disagree with their master? | join `DCheck2` to `DCheck` on `S_Link`, compare latest event's `S_State` |
| `DFish.S_State` (deposit channel, **not** a lifecycle) | 1–4 | the label for each | `SELECT S_State, S_StateName, COUNT(*) FROM DFish GROUP BY S_State, S_StateName;` |
| `Anbar_Factor.AF_Type` | 1 = `رسید انبار` (goods receipt), 2 = `حواله انبار` (goods issue); the family `'1,2,3,4,5,6,7,8,9'` is filtered at `AnbarFactorU.pas:593` | 3–9 are never labelled in source | `SELECT AF_Type, COUNT(*) FROM Anbar_Factor GROUP BY AF_Type;` |
| `Sahamdar.S_Kind` | 1 = natural person, 2 = legal entity; **only `1` is ever written** (`SahamdarEditU.pas:290`) | do type-2 rows exist? | `SELECT S_Kind, COUNT(*) FROM Sahamdar GROUP BY S_Kind;` |
| `Sahamdar.S_MaliatState` | a combo-box `ItemIndex` written raw (`SahamdarEditU.pas:309`); the meaning of each integer lives **only in the `.dfm` item list** (§2.6) | recover the item list from `SahamdarEditU.dfm` **and** confirm no other value exists | `SELECT S_MaliatState, COUNT(*) FROM Sahamdar GROUP BY S_MaliatState;` |
| `Base_Config.BC_ID` (system-account role) | 11, 13, 14, 15 (§2.4); `11`'s meaning is unknown | the full role set | `SELECT BC_ID, BC_Name, COUNT(*) FROM Base_Config GROUP BY BC_ID, BC_Name;` |
| `DCheck.S_linkPrg` / `DFish.S_LinkPRG` | 0 = manual, 1 = goods invoice; **no other value is handled** by the UI (`CheckListDU.pas:289-292`) | do other values exist in data? | `SELECT S_linkPrg, COUNT(*) FROM DCheck GROUP BY S_linkPrg;` |
| `Anbar_Jens.AJ_ID` (warehouse) | design-time default `93` — a real warehouse number (§3.1) | the warehouse set, and whether `AJ_ID` really is `Anbar_Config`'s key | `SELECT AJ_ID, COUNT(*) FROM Anbar_Jens GROUP BY AJ_ID;` |
| `Pass_Config.P_ID` | 1100–2125 (`08-platform-and-security.md` §4.2) | the full permission-id list and each id's caption | `SELECT DISTINCT P_ID, P_DESC FROM Pass_Config ORDER BY P_ID;` |

### 12.10 Column-identity questions in the accounting core

**P1 — 6 of 7 fully or mostly resolved by DDL/procedure bodies (schema, not data — this dump
answers all of these without needing rows).**

1. **`Article` vs `M_Article` — RESOLVED.** `Moein`'s full column list from the dump:
   `M_SSN, M_COID, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, Article, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2,
   M_Id, M_Link, M_User, M_Kind, M_Code, M_time`. **Only `Article` exists.** There is no
   `M_Article` column. `SanadEditU.dfm`'s field name is a naming inconsistency in that one form,
   not a second physical column — nothing to migrate twice, no stale-column risk.
2. **`Moein.M_Ted` type drift — RESOLVED.** The physical column is `M_Ted numeric(18, 3)`. This
   matches the `TBCDField(18,3)` binding; `TSingleField` and `TStringField(10)` in other forms are
   just different (looser or narrower) Delphi field-metadata declarations pointed at the same
   `numeric(18,3)` column — SQL Server will implicitly convert on either side, so no data-loss risk
   from the column itself, only from whichever narrower Delphi binding actually round-trips it
   somewhere. §11's `quantity` domain (`numeric(18,3)`) is exactly right as specified.
3. **`Moein.M_L` / `M_R` / `M_Name` / `M_CR` / `M_CodeStr` — RESOLVED: none are physical columns on
   `Moein`.** They are absent from `Moein`'s DDL. Every procedure that returns them
   (`Moein_Taraz1`, `Moein_Taraz2`, `Moein_sumAsnad`, `Moein_View_Daftar`, `Sarfasl_Seek_SSN`,
   `Sarfasl_view`, §12.3) computes `M_L`/`M_R` at query time via `Dbo.Make_L`/`Dbo.Make_R` (§12.4)
   or joins `Sarfasl.S_Name` for `M_Name`/`CodeName`. **§11 should not create these as columns on
   the `journal_lines`/`moein` table at all** — they are always derived, never stored, so there is
   no "denormalised copy with no maintenance" to decide about; there is nothing there to migrate.
4. **`DMoein.DM_Atf`** — DDL confirms `DM_Atf int NOT NULL DEFAULT (0)` (i.e. it is always present,
   never null, defaults to zero when not supplied). The *purpose* (cross-reference number vs
   attachment count) still needs the value distribution — **still open, needs data.**
5. **`DMoein.DM_CUser`/`DM_CDate` vs `DM_MUser`/`DM_MDate` swap — strongly corroborated, not yet
   100% proven.** DDL: `DM_MUser int` (nullable, no default), `DM_MDate datetime NOT NULL DEFAULT
   (getdate())`, `DM_CUser int NOT NULL DEFAULT (0)`, `DM_CDate datetime` (nullable, no default).
   The unreferenced-by-app procedure `DMoein_Make_Update` (§12.3, `02-12-a.md`) makes an `INSERT`
   that **explicitly sets `DM_MUser=@userid, DM_MDate=GetDate()` and leaves `DM_CUser=0,
   DM_CDate=NULL`** — i.e. on creation, the **M-prefixed columns are populated and the C-prefixed
   columns are left at their empty defaults**, exactly matching the "swapped" claim from
   `Dmu.pas:831-835`. This is strong, DDL-plus-procedure-body evidence for the swap, from a
   procedure that (per §12.3) has no confirmed live call site — so it corroborates the *design
   intent* rather than proving what every historical row actually contains.
   `SELECT COUNT(*) FROM DMoein WHERE DM_MDate > DM_CDate;` from §12.10's original text is still the
   test that would prove it against real history — **still open for historical confirmation, but no
   longer a guess about which convention the schema intends.**
6. **`Sarfasl.S_Active` semantics — RESOLVED.** See `02-12-a.md` §12.3 item 7 (`Active_Set`'s body):
   `S_Active = 1` if and only if `S_Child = 0 AND S_Mo > 0` — a leaf account (no children) below the
   Kol level. Kol-level accounts (`S_Mo = 0`) are never active regardless of `S_Child`.
7. **`Sarfasl.S_IS_Check` / `S_IS_Fish` / `S_IS_APArdakhti` / `S_IS_ADaryafti` — RESOLVED they
   exist; still open whether they hold meaningful data.** All four columns are present in the DDL
   (`tinyint`, three of the four indexed — see `02-12-a.md` §12.6). **No procedure in this dump
   writes to any of them**, matching the commented-out client-side assignments. Three being
   individually indexed despite being unwritten by anything in this dump is itself a new open
   question (something outside this dump's scope may still write them) — tracked in
   `11-open-decisions.md`. §11's decision to drop them stands on stronger evidence now (nothing
   server-side maintains them either), but "do historical rows hold meaningful values already"
   still needs a data query: `SELECT COUNT(*) FROM Sarfasl WHERE S_IS_Check=1 OR S_IS_Fish=1 OR
   S_IS_APArdakhti=1 OR S_IS_ADaryafti=1;`

### 12.11 Uniqueness that the migration will discover the hard way

**Still open — needs data.** This dump has zero rows; none of these duplicate/orphan probes can run
against it. One structural fact is now confirmed rather than inferred: **the DDL enforces no
uniqueness at all on `Sahamdar.S_CodeMelli`, `PassWord.UserName`, `AF_Factor`, or `DM_Sanad`** — no
unique index or constraint exists anywhere in the dump besides the two composite primary keys noted
in `02-12-a.md` §12.6 (`Sarfasl`, `SahamdarConfig`). So every duplicate probe below is checking a
constraint the database is fully capable of violating today; none of them are "confirming a
constraint that's probably already effectively unique in practice" — there is no historical
protection of any kind.

**P0 for the migration script.** §5.6 R8 and §11 propose real unique constraints. Existing
duplicates will **block** the migration and must be reconciled with the business owner *before*
cutover (§5.7). Run every one of these and expect non-empty results:

```sql
SELECT S_Ko,S_Mo,S_Ta1,S_Ta2, COUNT(*) FROM Sarfasl
  GROUP BY S_Ko,S_Mo,S_Ta1,S_Ta2 HAVING COUNT(*) > 1;              -- accounts natural key
SELECT DM_Coid, DM_Sanad, COUNT(*) FROM DMoein
  GROUP BY DM_Coid, DM_Sanad HAVING COUNT(*) > 1;                   -- voucher number per year
SELECT AF_COID, AF_Factor, COUNT(*) FROM Anbar_Factor
  GROUP BY AF_COID, AF_Factor HAVING COUNT(*) > 1;                  -- invoice number per year
SELECT S_Card, COUNT(*) FROM Sahamdar GROUP BY S_Card HAVING COUNT(*) > 1;
SELECT S_CodeMelli, COUNT(*) FROM Sahamdar
  WHERE S_CodeMelli IS NOT NULL AND LTRIM(RTRIM(S_CodeMelli)) <> ''
  GROUP BY S_CodeMelli HAVING COUNT(*) > 1;
SELECT UserName, COUNT(*) FROM PassWord GROUP BY UserName HAVING COUNT(*) > 1;
```

Equally: **orphans**, because no FK was ever enforced.

```sql
SELECT COUNT(*) FROM Moein  m LEFT JOIN Sarfasl s ON s.S_SSN = m.M_Code   WHERE s.S_SSN IS NULL;
SELECT COUNT(*) FROM Moein  m WHERE m.M_Code = 0;    -- known: the file importer writes 0 (§10.6)
SELECT COUNT(*) FROM Moein  m LEFT JOIN DMoein d
  ON d.DM_Coid = m.M_COID AND d.DM_Sanad = m.M_Sanad WHERE d.DM_SSN IS NULL;
SELECT COUNT(*) FROM DCheck2 e LEFT JOIN DCheck c ON c.S_SSN = e.S_Link WHERE c.S_SSN IS NULL;
SELECT COUNT(*) FROM Moein  m LEFT JOIN PassWord p ON p.UserCode = m.M_User WHERE p.UserCode IS NULL;
SELECT COUNT(*) FROM Moein  m LEFT JOIN Base b ON b.CO_ID = m.M_COID WHERE b.CO_ID IS NULL;
```

And the **denormalisation drift** checks already listed in §7.7 (recompute `DM_TBed`/`DM_TBes`,
`AF_Total`, `CM_Mab`, `TM_Mab`, `DM_Count`, `CM_Count`, `TM_Count`, `S_Child`) — every one of these
is maintained by application code with no constraint behind it, and every mismatch is a business
decision, not a technical one.

### 12.12 Fiscal-year scoping gaps

**Resolved by the procedure bodies (`02-12-a.md` §12.3), with one confirmed real bug.**

- **`Asnad_View`** takes `@CO int` and filters `M_COID=@CO` throughout — correctly scoped.
- **`Moein_ChapSanad`** takes `@Co int` and filters `@Co=M_CoID` throughout — correctly scoped **at
  the procedure level**. `RoozViewU.dfm`'s caller passing only `@Sanad` (no `@Co`) is therefore a
  **caller-side bug** (either the form supplies a hard-coded/wrong `@Co`, or the call fails) — not a
  missing filter inside the procedure as originally suspected. Worth a source-level check of exactly
  what `RoozViewU.dfm` passes for `@Co`, but the procedure itself is not the leak.
- **`Anbar_ReportKharidForoosh` — confirmed real cross-year leak.** Its signature is
  `@D1 varchar(10), @D2 varchar(10), @Type int` — **no fiscal-year parameter at all**, and the body
  confirms it: every query filters `Anbar_FactorD` by `AFD_Date` range and `AFD_Type` only, with
  **no `AFD_Coid` predicate anywhere in the procedure**. Since `AF_Date`/`AFD_Date` are Jalali
  strings that are not guaranteed globally unique across fiscal years in the same way a real `date`
  column would be, and nothing else scopes the query, **this procedure genuinely returns rows from
  every fiscal year that falls in the date range**, not just the caller's current one. This is a
  confirmed defect, not a hypothesis — recommend adding to `11-open-decisions.md` Group B (defects
  to fix) rather than leaving it as an open question.
- **`Sarfasl_view`** takes `@Co int` but the body (`Select * From Sarfasl order by S_ko, S_mo,
  S_ta1, S_ta2` — no `@Co` predicate anywhere) **never uses it.** Confirmed: the parameter is
  ignored, not used to join `Base` for display widths or anything else. Matches the fact that
  `Sarfasl` genuinely has no year column (§1.4) — the parameter is vestigial.

Also unresolved from §1: **whether the `Anbar`, `Saham` and `Rppc_Solution` catalogs live on the
same physical server** as the main database (§1.5). This decides whether the integration is a
cross-database join, a linked server, or a network call — and therefore whether it can be a
PostgreSQL FDW or must be an HTTP client. `SELECT name FROM sys.databases;` on the connection in
`CS2`, plus `SELECT * FROM sys.servers;` for linked servers. **Still open** — this is a single-database
schema export; it carries no `sys.databases`/`sys.servers` server-level metadata, so it cannot say
what else lives on the same instance. One data point in its favour, though: `02-12-a.md` §12.5
found `Pol_Select` (an app-adjacent, unreferenced-by-app-code procedure) with a **commented-out**
cross-database join to `Account.dbo.AcnAccounts` — i.e. whoever wrote it once assumed a same-server,
same-instance cross-database join was possible, which is suggestive (not proof) that `Anbar`/`Saham`
may be sibling databases on the same server rather than a remote system.

### 12.13 The `BN_*` bank table — does it exist?

**RESOLVED — it does not exist.** All 39 `CREATE TABLE` statements in the dump were enumerated
(`02-12-a.md` §12.5); none is named `BN_*` or has anything resembling `BN_SSN`, `BN_Name`,
`BN_BankCode`, `BN_AsnadCode`, `BN_Shaba`, `BN_Check`, or `BN_Fish`. `BankTanzim.dfm`'s inert form
describes a table that is not in this database. Whether it was dropped, never created, or lives in
one of the other catalogs (`Anbar`/`Saham`/`Rppc_Solution`, §12.12) cannot be told from this dump —
but for the database this dump is from, **the answer is definitively "does not exist"**. §11's bank
modelling is entirely new, with no legacy starting point to migrate from — confirmed, not just
inferred from the form being inert.

### 12.14 `TolidMaster` is probably not a table this system uses

**RESOLVED — confirmed absent.** None of the 39 `CREATE TABLE` statements is `TolidMaster` (or any
case variant). The "proposed conclusion" is correct: it is a design-time placeholder that was never
a real table in this database, and there is no row count to check because there is no table. §11
should not model it at all. **New, related finding:** `Kinds` — one of the other tables `02-12-a.md`
§12.5 expected to find (listed at confidence B/C in §2.2) — is **also absent** from all 39
`CREATE TABLE` statements, under every case/spelling searched. Unlike `TolidMaster`, nothing in this
document previously suspected `Kinds` was a placeholder; its disappearance is a genuine open
question, not a confirmation — see `11-open-decisions.md`.

`Moadian`'s **full DDL is now captured** (`02-12-a.md` §12.5): `M_SSN` identity, `M_ID tinyint`,
`M_Link int NOT NULL`, `M_Date datetime`, `M_UserID int`, `M_CodeMelli varchar(15)`, `M_Inty
tinyint`, `M_Tob tinyint`, `M_Name varchar(100)`, `M_Factorinno varchar(20)`,
`M_TAXID/M_Status/M_REFID/M_UID/M_ERROR varchar(512)`, `M_OK tinyint`, `M_CodePosti varchar(15)` —
19 columns, not the 2 (`M_link`, `M_id`) previously known from `AnbarListU.pas:537`. The extra
columns (`M_TAXID`, `M_Status`, `M_REFID`, `M_UID`, `M_ERROR`, `M_OK`) read like the request/response
bookkeeping for an e-invoice submission API call — a materially richer entity than "a correlated
`COUNT(*)`" suggested. The tax-submission feature can now be fully specified from this DDL; no
further dump is needed for its shape (only for what values `M_ID`/`M_Status` etc. actually take,
§12.9).

### 12.15 Platform and settings

**P1/P2 — 2 of 6 resolved by DDL, 1 narrowed.**

1. **`Base.IsActive` is written by no screen** (§2.3, §8.3) yet `Dmu.pas:1008-1014` blocks all
   posting when it is not `1`. **Column confirmed**: `IsActive tinyint`, nullable, no default —
   consistent with "no screen writes it" (a column with a real default would suggest at least
   table-creation-time initialization; this one has none). How is a year archived today — by hand
   in SSMS? **Still open — this is a business/operations question, not a schema question**; no
   amount of DDL settles who flips this flag. `SELECT CO_ID, IsActive, FromDate, ToDate FROM Base
   ORDER BY CO_ID;` still needs data.
2. **`Anbar_Config`'s primary key — RESOLVED.** `AC_ID int NOT NULL PRIMARY KEY`. See
   `02-12-a.md` §12.5 for the full column list. `warehouses` can be keyed on this directly.
3. **`Sahamdar_Show` takes `@Id`, `Sahamdar_Seek` takes `@S_card` — RESOLVED, and the doc's own
   assumption was wrong.** The DDL settles it directly: `Sahamdar`'s primary key is
   `CONSTRAINT PRIMARY KEY (S_Card)` — **`S_Card`, not `S_SSN`.** `S_SSN` is a plain
   `IDENTITY(1,1)` column with no key on it at all (§12.6). §2.6's assumption ("`S_SSN` with
   `S_Card` unique") is backwards: `S_Card` is the enforced key, `S_SSN` is the redundant
   surrogate. `Sahamdar_Show`'s `@Id` parameter is unused in its body (`Select * From Sahamdar
   order by -S_Card` — `@Id` never appears) — it is vestigial, matching the pattern already found
   for `Sarfasl_view`'s `@Co` (§12.12). §11's `parties`/`shareholders` primary-key choice should
   follow `S_Card`, not a fresh surrogate mapped from `S_SSN`.
4. **Do multiple organisations' letterheads actually differ per year?** Still open — needs data,
   this dump has none. `Base`'s columns are otherwise fully confirmed (§12.5-adjacent: `CO_ID` PK,
   `No_Ko/No_Mo/No_Ta1/No_Ta2 bigint`, `Co_Name/Co_Sub/Co_Address varchar(100)`, `Co_Tel/Co_Fax
   varchar(20)`, `Fromdate/todate varchar(10)`, `IsActive tinyint`, `M_Shenaseh varchar(6)`,
   `M_PrivateKey/M_PublicKey varchar(2048)` — the last two storing **cryptographic key material in
   plain `varchar`**, worth flagging for the security review independent of this question).
5. **`arzi.local.ini` in production.** Still open — a schema-only DDL/procedure dump carries no
   connection-string or `.ini` content. Needs the live server or the deployed config file directly.
6. **`Tanzim.T_Int`** — DDL confirms the physical column is `T_Int bigint` (not text) sitting beside
   `T_Str varchar(512)` and `T_Desc varchar(50)`, with `T_ID int NOT NULL PRIMARY KEY`. The claim
   that it's written as the string `'0'` is therefore an implicit string→bigint conversion at
   insert time, not a type mismatch — consistent with, not contradicting, "genuinely unused".
   Whether any row holds a non-zero value still needs data: `SELECT DISTINCT T_Int FROM Tanzim;`

### 12.16 Volume and profiling data needed to size the target

**Still fully open — needs data.** `Full_Script_14050527.sql` has no rows (`sys.partitions.rows`
equivalents are not exportable from a schema-only script) — none of the sizing questions below can
be answered from it. One new, mildly concerning data point for capacity planning: §12.5
(`02-12-a.md`) found `mandeh_1404` and `Tah1403` — **physical per-fiscal-year table copies**, not
just `CO_ID`-scoped rows in a shared table. If this pattern repeats for every fiscal year, the row
counts and index/partitioning decisions in §11 need to account for a table-per-year growth pattern
in at least some parts of the schema, not only row growth within stable tables — worth confirming
via `SELECT name FROM sys.tables WHERE name LIKE '%1403' OR name LIKE '%1404' OR name LIKE
'Tah%' OR name LIKE 'mandeh_%';` on the live server to see how many year-suffixed tables actually
exist.

**P2**, but cheap and needed for index and partitioning decisions in §11:

```sql
SELECT t.name, SUM(p.rows) AS rows
FROM sys.tables t JOIN sys.partitions p ON p.object_id = t.object_id AND p.index_id IN (0,1)
WHERE t.is_ms_shipped = 0 GROUP BY t.name ORDER BY rows DESC;

SELECT MIN(CO_ID), MAX(CO_ID), COUNT(*) FROM Base;          -- how many fiscal years
SELECT M_COID, COUNT(*) FROM Moein GROUP BY M_COID ORDER BY 1;  -- lines per year
SELECT MAX(M_Bed), MAX(M_Bes), MAX(AF_Total) FROM ...;      -- do any exceed int32 (§7.7 check 2)
```

### 12.17 Summary — what a single database session must produce

**Status after `Full_Script_14050527.sql`: 6 of 11 artefact categories delivered (schema side);
5 remain, all of them data-shaped — this dump is schema-only (0 `INSERT` statements) and cannot
supply them.** One session against a **populated** production restore (not just a schema export)
is still needed, now scoped down to exactly the following:

1. ~~`sys.procedures` + `OBJECT_DEFINITION`~~ — **DONE.** 49 procedures captured, all named-in-doc
   ones present except `Sahamdar_Edit` (§12.3), plus 19 previously-unknown extras (§12.3).
2. ~~`sys.objects type IN ('FN','IF','TF')` + `OBJECT_DEFINITION`~~ — **DONE.** 4 functions captured
   (`Farsi_Date`, `Make_L`, `Make_R`, `NoTo3`) — `Farsi_Date` was not previously known (§12.4).
3. ~~`sys.columns` / `sys.types` / `sys.default_constraints` for every table~~ — **DONE.** 29 real
   tables, full DDL (§12.5). `Kinds` and `TolidMaster` confirmed absent; `mandeh_1404`, `Tah1403`,
   `POL_Namad` confirmed present but previously undocumented.
4. ~~`sys.key_constraints`, `sys.foreign_keys`, `sys.check_constraints`, `sys.indexes`~~ — **DONE.**
   Zero FKs, zero CHECKs, two composite PKs (`Sarfasl`, `SahamdarConfig`), three single-column
   indexes (all on `Sarfasl`) (§12.6).
5. ~~`sys.triggers`, `sys.views`~~ — **DONE for triggers**: exactly one (`Jens_Update` on
   `Anbar_Jens`, §12.7). **Views not captured** — the dump contains no `CREATE VIEW` statements, so
   either none exist or the export tool that produced this file was scoped to tables/procedures/
   functions/triggers only. Worth a direct `SELECT * FROM sys.views;` to be certain, since a script
   that dumps triggers but not views may simply not have been asked for views.
6. **Partially done.** Per-column `collation_name` is confirmed absent (no `COLLATE` overrides
   anywhere, §12.8) and `varchar`/`nvarchar` split is fully known (§12.8). **`sys.databases`'s
   server-level default collation and `sys.servers` (linked servers) are still needed** — schema
   exports of this kind do not carry server-level metadata.
7. **Still fully open.** The enumeration profiles in §12.9 need rows; none exist in this dump.
8. **Still fully open.** The duplicate/orphan/drift probes in §12.11 need rows.
9. **Partially done.** The physical type/length/algorithm side of §12.1 and the column-existence
   side of §12.2 are resolved; the *value-shape* probes (are stored strings actually zero-padded
   `YYYY/MM/DD`, do outliers exist) still need rows.
10. **Still fully open.** Row counts and value maxima (§12.16) need rows.
11. **Still fully open.** The same for the `Anbar`, `Saham` and `Rppc_Solution` catalogs — this
    dump is a single database's schema, not a server-wide catalog listing.

**What a follow-up session needs to bring, precisely:** a populated (not schema-only) restore or
export, so items 7-11 above (and the value-shape half of 1 and 2) can finally be answered — plus,
ideally, whatever produced this dump re-run with **views** included, and one `sys.databases` /
`sys.servers` query for item 6's remaining half.


---

[← 02-12-a-open-questions-dates-and-procedures.md](02-12-a-open-questions-dates-and-procedures.md) | [02-13-a-improvements-integrity-and-keys.md →](02-13-a-improvements-integrity-and-keys.md)
