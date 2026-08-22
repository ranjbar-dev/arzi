_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 12. Open questions

Everything below is a fact about the data layer that **could not be determined from the source
tree**. Each item states the question, why it matters, and the exact artefact or query that would
answer it. Nothing here is a matter of opinion — these are gaps, not choices. (Choices are §13.)

Priority key: **P0** blocks writing the migration; **P1** blocks implementing a module; **P2**
affects fidelity but can be deferred behind a flag.

---

### 12.1 The blocking question — how are Jalali dates physically stored?

**P0. Partially resolved by `Full_Script_14050527.sql` (schema-only dump, SQL Server, no data rows).
The physical type/length and the server-side algorithm are now known. The historical value shapes
and which algorithm produced them are still unknown — no data in this dump.**

**What we know.** Every date column in the schema is a character column holding a Jalali date
string (§6.1), and the application compares them lexicographically rather than with date arithmetic
(§6.5). Two Jalali conversion algorithms exist in the Delphi source — `TUtil.DecodedateF` /
`TUtil.FarsiDate` (`Utility.pas:413-442`, §6.3.1) and `TDM.MiladiToShamsi` (`Dmu.pas:362-437`,
§6.3.2) — they **disagree** (§6.3.3), and **neither has a single call site** (repo-wide grep:
`FarsiDate` → `Utility.pas:47,435` only; `MiladiToShamsi` → `Dmu.pas:140,362` only). Both are dead
code compiled into the binary.

**What we do not know, and cannot know from source.** The treasury and accounting screens obtain
and render their dates through `Tools.TEditDate` / `TFullDate` — a **third-party VCL control whose
source is not in this repository** (§6.7). The conversion therefore happens inside a binary. The
only server-side conversion, `XNew` (§3.1), is a stored procedure whose body is likewise absent
(§3.0). Consequently:

- We cannot prove the stored strings are `'YYYY/MM/DD'` for every column (only that the
  `TStringField.Size = 10` declarations *imply* it, and that `Sahamdar.S_BDate`/`S_SDate` and
  several SP parameters are **size 8**, i.e. `'YY/MM/DD'` — §3.4).
- We cannot prove which Jalali↔Gregorian algorithm produced the historical values, so we cannot
  choose the algorithm that round-trips them.
- We cannot rule out that some columns hold Gregorian dates, or free text (see 12.2).

**Artefacts that answer it.** Run against a production or recently restored database:

```sql
-- (a) the physical type and length of every date-shaped column
SELECT t.name AS tbl, c.name AS col, ty.name AS type, c.max_length, c.is_nullable, c.collation_name
FROM sys.columns c
  JOIN sys.tables t  ON t.object_id = c.object_id
  JOIN sys.types  ty ON ty.user_type_id = c.user_type_id
WHERE c.name LIKE '%Date%' OR c.name LIKE '%DateS%'
ORDER BY t.name, c.name;

-- (b) the actual value shapes present, per column (repeat per column)
SELECT LEN(M_Date) AS len, COUNT(*) AS n, MIN(M_Date) AS lo, MAX(M_Date) AS hi
FROM Moein GROUP BY LEN(M_Date) ORDER BY n DESC;

SELECT TOP 200 DISTINCT M_Date FROM Moein
WHERE M_Date NOT LIKE '[0-9][0-9][0-9][0-9]/[0-9][0-9]/[0-9][0-9]';

-- (c) the conversion the server actually performs
SELECT OBJECT_DEFINITION(OBJECT_ID('dbo.XNew'));

-- (d) a ground-truth pair: run XNew and record the answer against the known Gregorian date
EXEC XNew @COID = 1403;   -- record CurrentDate alongside the real date the query was run
```

Additionally required and **not obtainable from this repository**: the compiled `Tools` package or
its source (the vendor's `TEditDate`/`TFullDate`), or, failing that, a set of at least 40
(Gregorian date entered → Jalali string stored) observations spanning leap years, so the control's
algorithm can be identified by fingerprint. The candidate fingerprints are tabulated in §6.3.3.

**Resolved by the dump — query (a):** every date-shaped column is `varchar`, never a real date/time
type (except `Anbar_Jens.AJ_DateTime datetime` and `Moadian.M_Date datetime`, which are unrelated
audit timestamps, and `Moein.M_time`/`DMoein.DM_MDate datetime DEFAULT (getdate())`, also audit
timestamps not business dates). Business-date columns are `varchar(10)` almost everywhere
(`M_Date`, `AF_Date`, `S_Date`, `CM_Date`, `DM_Date`, `R_Date`…) with the `S_DateS`/`TM_Date`
exceptions in §12.2. No column is `char` — all are `varchar`, so short values are not
space-padded.

**Resolved by the dump — query (c): `XNew`'s body is captured in full.** It does **not** convert
dates directly — it delegates to a fourth, previously-unknown function:

```sql
CREATE PROCEDURE [dbo].[XNew] @COID int
AS Begin
    Declare @NextFactor int
    Set @NextFactor = isnull( (Select Max(AF_Factor) From Anbar_Factor Where @COID=AF_COID ) , 0) + 1
    Declare @NextSanad int
    Set @NextSanad = isnull( (Select Max(M_Sanad) From Moein Where @COID=M_COID ) , 0) + 1
    Declare @FDate Varchar(10)
    Set @FDate =  ( Select FromDate From Base Where CO_ID=@COID )
    Declare @TDate varchar(10)
    Set @TDate =  ( Select ToDate From Base Where CO_ID=@COID )
    Declare @CDate varchar(10)
    Set @CDate =  Dbo.Farsi_Date( GetDate() +1)
    if @CDate > @TDate  Set @CDate = @Tdate
    if @Cdate < @FDate  Set @CDate = @FDate
    Select @NextFactor As NextFactor, @NextSanad As NextSanad, @FDate As FromDate, @TDate As ToDate, @CDate As CurrentDate
End;
```

So `XNew` is mainly a next-number allocator (invoice/voucher counters, §5.7); the only date logic in
it is `@CDate = dbo.Farsi_Date(GetDate()+1)`, clamped into the fiscal year's `[FromDate, ToDate]`
range. **`dbo.Farsi_Date` is a fourth Gregorian→Jalali implementation, not previously known from the
source tree**, and it is the one that actually ran, server-side, for every "today" default offered
on a data-entry screen:

```sql
CREATE FUNCTION [dbo].[Farsi_Date] (@Date DateTime)
RETURNS Varchar(10)  AS
BEGIN
	Declare @FD Float       Set @FD =  Cast( @Date As Float )
	Declare @ADate int     Set @ADate = Floor( @FD )
	Declare @Ayear int     Set @Ayear = 1279
	Declare @Amonth int  Set @Amonth = 1
	Declare @Aday int      Set @Aday = 1
	Declare @R int            Set @R = @ADate - 78
	Declare @I int             Set @I = 1
	While @I < 400  Begin
		if @I - 4*Round((@I/4),0) = 0  Set @R = @r - 366 Else Set @R = @R - 365
		Set @AYear = @AYear + 1
		if @R < 365 Break
		Set @I = @I + 1
	End
	Set @I = 1
	While @I < 7  Begin
		if @R > 31  Begin
			Set @AMonth = @AMonth + 1
			Set @R = @R - 31
		End Else Begin
			Set @ADay = @R
			Break
		End
		Set @I = @I + 1
	End
	if @I = 7 Begin
		Set @I = 1
		While @I < 7  Begin
			if @R > 30  Begin
				Set @AMonth = @AMonth + 1
				Set @R = @R - 30
			End Else Begin
				Set @ADay = @R
				Break
			End
			Set @I = @I + 1
		End
	End
Return	 Cast(@AYear as varchar(4))+'/' +
	 Right( '0'+Ltrim( Cast(@Amonth as varchar(4))) , 2 ) + '/' +
	Right( '0'+Ltrim( Cast(@ADay as varchar(4))) , 2 )
END;
```

**Comparison against the two client-side algorithms (now possible — both are in this repository):**

| | `dbo.Farsi_Date` (SQL, server) | `TUtil.FarsiDate`/`DecodedateF` (`Utility.pas:413-434`) | `TDM.MiladiToShamsi` (`Dmu.pas:362-437`) |
|---|---|---|---|
| Epoch anchor | `@ADate - 78`, start year 1279 | `ADate - 80`, start year 1279 | proper Gregorian day-of-year decomposition, no epoch offset |
| Leap rule | `%4` on the **loop counter**, not the Persian year — a naive approximation, not the real 33-year Persian cycle | identical `%4`-on-counter approximation | real: `IsLeapYear` on the Gregorian year feeding a correct Persian day-count split (79/186/etc. day boundaries) |
| Output format | `YYYY/MM/DD` (4-digit year) | `YY/MM/DD` (`Ayear mod 100`, 2-digit year) | `YYYY/MM/DD` |
| Relationship | — | **same algorithm family as `dbo.Farsi_Date`** (identical leap approximation, identical month-split loop) but **not identical**: the epoch offset differs by 2 (`-78` vs `-80`), which shifts the computed date near month/year boundaries | **structurally unrelated** — a different, more correct method |

**What this settles:** the "two algorithms disagree" problem in the source is now a **three-algorithm**
problem, and the two that share the same naive-leap-year design (`dbo.Farsi_Date` and
`TUtil.FarsiDate`) are close relatives but provably not the same function — a value produced by one
will not always match the other, especially near a month or leap-year boundary. Since
`TUtil.FarsiDate` has no call site (confirmed dead, §6.3.3) and `dbo.Farsi_Date` is the one actually
invoked by `XNew` on every "today" default, **`dbo.Farsi_Date`'s naive-%4-leap, `ADate-78`-epoch
algorithm is the best current candidate for what the server itself produced** — but this only covers
values defaulted from `XNew`. Values typed by hand through `Tools.TEditDate`/`TFullDate` (still
closed-source, still unobtainable, §6.7) could still use a fourth, different algorithm; that
component's behaviour remains genuinely unverifiable without either its source or ≥40
Gregorian→Jalali observations from a live, populated database.

**Still open — needs data, not just schema:** query (b) (actual value shapes per column: are they
consistently zero-padded 10-char strings, or do outliers exist), and query (d) (running `XNew`
against a known real date to fingerprint-test `dbo.Farsi_Date` against ground truth) both require
row data. This dump has none (0 `INSERT` statements — see §12.17 note). **Until a populated database
is available, §11 stores business dates as `date` (Gregorian) with a shadow
`legacy_date_jalali text` column (§6.8), and the migration decode step is unwritable — narrowed from
"unwritable, cause unknown" to "unwritable, blocked only on sample data", since the schema and the
server's own algorithm are now known.**

---

### 12.2 `DCheck.S_DateS` — a 50-character "date" column

**RESOLVED by the dump.** `DCheck`'s columns are, in full:
`S_SSN int IDENTITY, S_COID int, S_State int, S_StateName varchar(50), S_CheckNo varchar(15),
S_Sanad int, S_Date varchar(10), S_DateS varchar(50), S_Mab bigint, S_Desc varchar(200),
S_BesSSN int, S_BesCR varchar(50), S_BesName varchar(100), S_BedSSN int, S_BedCR varchar(50),
S_BedName varchar(50), S_UserID int, S_linkPrg int NOT NULL DEFAULT (0), S_LinkSSN int NOT NULL
DEFAULT (0), S_Zssn int, S_ZCR varchar(50), S_ZName varchar(100)`. This confirms the `Size = 50`
`TStringField` binding — `S_DateS` really is `varchar(50)` in the database, not a Delphi
metadata artefact, and sits beside a genuine `S_Date varchar(10)`. **There is no third column**
— no `S_Dates` exists anywhere in the DDL. `CheckListDU.pas:329` and `FishListD.pas:229`'s
`Order By S_Dates` therefore resolves to `S_DateS` purely through SQL Server's default
case-insensitive collation (confirmed absent server/column-level collation override, §12.8) — it
is the same column, not a missing fourth spelling, and not a bug. `DFish` has the identical
`S_DateS varchar(50)` column (resolving `06-treasury.md §12` Q3 — yes, it physically exists).
**Still open:** why a date is 50 characters wide, and what it actually contains, both require row
data this dump does not have — the dump proves the column is real and correctly named, not what is
stored in it.

**P0 for treasury (narrowed).** `S_DateS` is declared `TStringField Size = 50` while `S_Date` beside it is
`Size = 10` (`Dmu.dfm:943-950`, §6.6). Treasury calls it the cheque **due date** (`سررسید`,
`06-treasury.md` §1.1) and it is the sole sort key of the cheque list (`Order By S_Dates`,
`CheckListDU.pas:329`) and the sole basis of the ageing filter. A 50-character column is not a
date. Furthermore `CheckListDU.pas:329` and `FishListD.pas:231` order by **`S_Dates`** — a *third*
spelling that differs from both `S_Date` and `S_DateS`.

**Answered by:**

```sql
SELECT c.name, ty.name, c.max_length FROM sys.columns c
  JOIN sys.types ty ON ty.user_type_id = c.user_type_id
WHERE c.object_id = OBJECT_ID('DCheck') AND c.name LIKE 'S_Date%';

SELECT LEN(S_DateS) AS len, COUNT(*) n FROM DCheck GROUP BY LEN(S_DateS) ORDER BY n DESC;
SELECT TOP 100 S_DateS FROM DCheck WHERE LEN(LTRIM(RTRIM(S_DateS))) <> 10;
```

**Resolved:** `S_Dates` is not a separate column (see above) — no extra column is needed in §11's
`cheques` table on this account.

---

### 12.3 Stored-procedure bodies that must be dumped

**RESOLVED — all bodies captured.** `Full_Script_14050527.sql` is a schema-only dump (DDL, procedure
and function bodies, one trigger; **zero `INSERT` statements** — no data). It contains 49
`CREATE PROCEDURE` statements. Of the ~31 procedures named in the table below, **30 are present
verbatim; one, `Sahamdar_Edit` (item 8), is absent from the dump** — either this dump predates its
creation, it was renamed, or party CRUD really does happen through inline `INSERT`/`UPDATE` in the
Delphi data module rather than a stored procedure. This is now an open question in its own right —
see `11-open-decisions.md` new item.

Key findings from the captured bodies, resolving the "why it matters" column for each row:

| # | Procedure | Resolution |
|---|---|---|
| 1 | `MoeinAdd` | **No special parsing exists.** `@bed`/`@bes varchar(20)`, `@ted varchar(15)` are only `isnull()`-defaulted to `0`, then passed straight into the `bigint`/`bigint`/`numeric(18,3)` columns via implicit conversion — whatever string the client sends must already be a plain numeric literal (no thousands separators tolerated). Validates the account exists in `Sarfasl`, that a `Mo` is given when `kind=1`, that `Ta1` is given if the parent has children, and that `Article` (narration) is non-empty — all four checks return a `(ErrorNo, ErrorDesc)` pair rather than raising. `M_Code` is resolved as `Sarfasl.S_SSN` of the matched `(Ko,Mo,Ta1,Ta2)` row — confirms the orphan-check join in §12.11 (`Moein.M_Code = Sarfasl.S_SSN`) is the intended relationship. `@State = 0` inserts a new line; any other value is treated as the `M_SSN` of an existing line to `UPDATE` (edit-in-place). Does **not** touch `DMoein` — header totals are a separate step (`DMoein_Make_Update`, an unreferenced-by-app extra, see below). |
| 2 | `MakeSanad_CheckDaryafti` | Full body captured. Guards: cheque must exist and be `S_State = 1` ("در صندوق"); refuses if a draft (`M_TX=0`) already posted under `M_Id=21` for this cheque (must be finalized first); deletes any prior `M_Id=21` lines for this `S_SSN`, then inserts exactly 2 `Moein` rows (debit `S_BedSSN`, credit `S_ZSSN`) carrying a narration built from `dbo.Noto3` and `S_DateS`. Confirms the "2 lines per cheque receipt" claim in `06-treasury.md §3.3` at the SQL level, not just the Delphi level. |
| 3 | `XNew` | Full body in §12.1 — mainly a next-number allocator; only incidentally computes a Jalali "today" via `dbo.Farsi_Date`. |
| 4 | `Anbar_AddToFactor` | Full body captured (~115 lines). Resolves invoices to up to 4 `Moein` postings per line (goods/counterparty, warehouse-config account, tax, discount), each guarded by `if not exists(...) insert ... update`, i.e. **idempotent by design against re-running with the same `@factor`** — re-saving does not double the posting, provided `M_link=@factor` stays constant. `@Code1`/`@Code2`/`@Code4` (goods/tax/discount accounts) are looked up from `Anbar_Config` keyed by the item's `Anbar_Jens.AJ_ID` (warehouse) — confirming `AJ_ID` really is `Anbar_Config.AC_ID`'s foreign key, answering part of §12.9's `Anbar_Jens.AJ_ID` question at the schema-relationship level (the value *set* still needs data). |
| 5 | `Sarfasl_ADD` | Full body captured. Rejects on duplicate `(Ko,Mo,Ta1,Ta2)` — matches the enforced composite `PRIMARY KEY (S_Ko,S_Mo,S_Ta1,S_Ta2)` found in the DDL (§12.5/§12.6). Also rejects if the **parent** code already has `Moein` postings (`m_kind=1`) — you cannot add a child under an account that has already been posted to directly. New rows are inserted with `S_Active = 1, S_Child = 0` hardcoded, then `NeedUpdate=1` is set on the parent and `Active_Set` is invoked inline — the real `S_Active`/`S_Child`/`FullName`/`LineName`/`M_L`/`M_R` values are computed by that recompute pass, not by `Sarfasl_ADD` itself. |
| 6 | `Sarfasl_Deep` | Full body captured. **Answered: it deletes, guarded.** Counts children (any row with a longer matching key prefix) and counts `Moein` rows already posted at that exact key. If children exist (`@C>1`) or postings exist (`@D>0`) it refuses with a Persian error message and does **not** delete. Only when both are zero does it run `DELETE Sarfasl WHERE S_ko=@K and S_mo=@M and S_ta1=@t1 and S_ta2=@t2`. It does not touch `Moein`, `DMoein`, or any `Anbar_*` table — the referential check is one level only (does this exact node have postings), not a cascade check across the whole subtree's descendants' postings beyond what `@C`/`@D` capture. |
| 7 | `Active_Set` | Full body captured (~85 lines, one batch `UPDATE ... WHERE NeedUpdate=1` per derived column, run over **every** row with `NeedUpdate=1`, not scoped to one account). **Resolves what "active" means:** `S_Active=1` iff `S_Child=0 AND S_Mo>0` — i.e. a leaf account below the Kol level with no children under it. `S_Child` is a count of *direct* children at the next level down, minus 1 for a self-match quirk in the correlated subquery (`(SELECT COUNT(*) ... ) - 1`). Also rebuilds `FullName` (newline-joined ancestor names) and `LineName` (`/`-joined ancestor names) and `M_L`/`M_R` (zero-padded `Ko[-Mo[-Ta1[-Ta2]]]` code strings, left-biased — see UDF `Make_L`/`Make_R` below). **`Active_Set` never writes `S_IS_Check`, `S_IS_Fish`, `S_IS_APArdakhti`, or `S_IS_ADaryafti`** — and no other procedure in this dump does either (grepped). Those four columns exist in the DDL, three of them are indexed (§12.6), but nothing server-side maintains them, matching the client-side finding that their assignments are commented out. This is now a live open question, not just a source-code inference — see `11-open-decisions.md` new item. |
| 8 | `Sahamdar_Edit` | **Absent from the dump.** New open question (above). |
| 9 | `Moein_All` | Full body captured. Straightforward: sums `M_bed - M_bes` (and the reverse) per `(Ko,Mo,Ta1,Ta2)` for `M_kind=1`, clamps negatives to 0, drops zero rows. This is **not the same computation** as the inline `#R` query at `EnteghalU.dfm:330-349` referenced in the original question — that comparison still needs the `EnteghalU.dfm` query re-read side-by-side with this body; flagged, not fully closed (a source-vs-source diff, not a dump question). |
| 10 | `Anbar_Mandeh` | Full body captured. `AFD_Type` 1/2/3/4 map to in/out/bounced-out/bounced-in (`Noin`/`NoOut`/`NoBOut`/`NoBIn`), `avin`/`avout` are unit-cost averages (`mabin/noin`), confirming these are **unit of measure quantities aggregated by movement type**, not a second warehouse dimension — resolves the "two units of measure or two warehouses" question in the original table row. `Remi` (remaining stock) = `noin - noout + nobout - nobin`. |
| 11 | `Anbar_CardJensi` | Full body captured. Running balance (`Sumr`) is a correlated `SUM(AFD_In-AFD_Out) WHERE P.SSN <= #R.SSN` over an identity-numbered temp table ordered by date/factor — a cumulative running total, standard stock-card semantics. |
| 12 | `Taraz4Setooni` | Full body captured. `@St`: 1=Kol level, 2=Moein level, 3=Tafzil level (matches the header comment exactly). `@Ki`: 1=Moein-sourced, 2=Rooznameh-sourced. `@Sabt`: 1/2/3 select `M_Tx=1`, `M_Tx=2`, or `M_Tx>0` (both) respectively — **`@Sabt=3` is confirmed to be an "any non-draft state" filter sentinel on the query parameter, not a claim that `M_Tx` itself takes the value 3 anywhere in the data.** This directly resolves the `[VERIFY]` flag in `02-11-a.md` about `voucher_status` only needing 3 enum values. |
| 13 | `Taraz_6Sotooni` | Full body captured — **and a new finding: there are two distinct, differently-spelled procedures, not one procedure referenced inconsistently by case.** `Taraz_6Sotooni` (double-`o`, params `@D1,@D2,@kind,@Coid,@Level,@Sabt`) is the one actually referenced by the Delphi source (`Dmu.pas`, `Mainu.pas`) and computes opening (`Bed1`/`Bes1`, before `@D1`) vs period (`Bed2`/`Bes2`, `@D1..@D2`) balances per account, same `@Sabt` sentinel semantics as `Taraz4Setooni`. **`Taraz_6Sotoni`** (single-`o`, params `@FromDate,@ToDate,@St,@Ki,@Sabt,@Co` — the signature this open-questions table originally assumed) **is a separate, orphaned procedure with no call site anywhere in the `.pas`/`.dfm` tree** (grepped). The two are not case-variants of each other; they are unrelated procedures that happen to differ by one letter. `02-11-a.md`/`02-11-h.md` and `02-14-b-naming-map-procedures-and-modules.md` should key off `Taraz_6Sotooni` (double-o) only. |
| 14 | `Moein_View_Daftar` | Full body captured. Running balance via the same identity-temp-table pattern as `Anbar_CardJensi`; an opening-balance pseudo-row is prepended (`Article = 'گردش تا تاریخ '+@D1`) summing everything before `@D1`. |
| 15–27 | `MoeinViewSanad`, `MoeinTotalSanad`, `Moein_ChapSanad`, `Asnad_View`, `KolState`, `Sarfasl_view`, `Sarfasl_Seek_SSN`, `Sarfasl_Seek_Name`, `Select_Kol`, `Select_moein`, `Select_Taf1`, `Select_Taf2`, `Sahamdar_Seek`, `Sahamdar_Show`, `Anbar_AjnasView`, `Anbar_PrintFactor`, `Anbar_ReportKharidForoosh` | All 17 present verbatim, all simple `SELECT`/aggregate reporting queries confirming the column lists and filter shapes already assumed in `04-reporting.md`. `Asnad_View`, and `Moein_ChapSanad` (as called from `RoozViewU.dfm` with only `@Sanad`) confirmed to take **no fiscal-year parameter in the procedure itself** either — `Moein_ChapSanad` does receive `@Co` and filters `@Co=M_CoID` throughout, so **that one is fiscal-year-scoped correctly at the SQL level**; the `RoozViewU.dfm` caller passing only `@Sanad` (§12.12) is a caller-side gap, not a missing filter inside the procedure. `Asnad_View` takes `@CO` and filters on it too — also correctly scoped. **This resolves §12.12's first paragraph**: both procedures already accept and apply a fiscal-year filter; if cross-year leakage happens it is because a caller omits the parameter, not because the procedure ignores it. `Sarfasl_view` takes `@Co` but never uses it in the body (`Select * From Sarfasl order by ...` — no `@Co` predicate at all) — **confirmed unused parameter**, resolving the "is it ignored" half of §12.12's second paragraph. |

**Extra procedures — present in the dump but not named in this table, and not referenced anywhere in
the `.pas`/`.dfm` source tree** (grepped individually; none matched outside `Full_Script_14050527.sql`):

- `notify_users` — a well-known, publicly-published generic SQL Server admin script (header credits
  "Narayana Vyas Kondreddi, 2000") that `net send`s a message to every connected session via
  `xp_cmdshell`. Not Arzi-specific; almost certainly a DBA utility left on the server, not application
  code.
- `Make_Directory` — wraps `xp_fileexist`/`xp_create_subdir`. Generic filesystem utility, not
  referenced by the app.
- `Pol_Select` — reads `Pol_Namad` (a table also not documented elsewhere in `02-*`, see §12.5 new
  finding) and has commented-out joins to `Account.dbo.AcnAccounts`, i.e. **a different database**
  named `Account` on the same server. This looks like a stock-market ("نماد" = ticker symbol)
  integration or import staging table for a *separate* system, not part of Arzi's own schema.
- `Anbar_AddToFactor2`, `MakeSanad_Fishvarizi` — both are stub bodies (`Select 1 as E` / `Select 0 as e, 'ok' as d`
  with no logic at all) — placeholders, never finished, not called.
- `Kol_Taraz1`, `Kol_Taraz2`, `Moein_Taraz1`, `Moein_Taraz2`, `Moein_Taraz2_Head`, `Moein_Taraz3`,
  `Taraz_6Sotoni` (single-o, see item 13 above), `Jari_Remind`, `Anbar_Report2`,
  `MakeSanad_CheckBank`, `Moein_sumAsnad`, `Moein_Set_Tx`, `Moein_Delete_SSN1`, `DMoein_Make_Update` —
  all fully-formed, apparently-working trial-balance/ledger/cheque/DMoein-maintenance procedures with
  **no call site found anywhere in the Delphi source**. Several look like superseded or in-progress
  replacements for procedures that *are* called (`Kol_Taraz1`/`Kol_Taraz2` read like earlier drafts of
  `Taraz_6Sotooni`; `DMoein_Make_Update`, dated `1403/02/24` per its header comment, looks like the
  intended header-totals maintainer described in `06-treasury.md` B13 but under a different name than
  any call site references — `DMoein_Make`, not `DMoein_Make_Update`). Flagged as a new open item
  rather than assumed dead, since a job scheduler, a different front-end, or a not-yet-wired-in screen
  could still call any of these by name. See `11-open-decisions.md` new item.

**Not resolved by this dump:** whether any of the above extras are invoked by a SQL Agent job, a
linked application, or manually — that requires `sys.jobs`/`sysjobsteps` from the live server, which
this schema-only dump does not include.

The procedures, **listed by name**, with what each blocks (original table, kept for reference):

| # | Procedure | Blocks | Why the body is required |
|---|---|---|---|
| 1 | `MoeinAdd` | the whole accounting core | parses `@bed`/`@bes` from `varchar(20)` and `@ted` from `varchar(15)`; resolves `M_Code`; almost certainly maintains `DMoein` totals. The server-side parse rule *is* the money semantics (§3.1, §7). |
| 2 | `MakeSanad_CheckDaryafti` | treasury posting | the entire debit/credit generation for a received cheque (`CheckDaryaftU.pas:356`). |
| 3 | `XNew` | every date | the live Gregorian→Jalali conversion (12.1). |
| 4 | `Anbar_AddToFactor` | inventory invoicing | invoice-header roll-up of `AF_Mab`/`AF_Kasr`/`AF_Maliat`/`AF_Total` and the line count. |
| 5 | `Sarfasl_ADD` | chart of accounts | maintains `S_Child`, `FullName`, `M_L`/`M_R`. |
| 6 | `Sarfasl_Deep` | account deletion | the referential-integrity checks the schema does not enforce. Also: **does it delete, or only report?** The component is named `Sp_Del` (`ListSarfaslu.dfm:249`) but the name means "depth". |
| 7 | `Active_Set` | account maintenance | which derived column(s) it rewrites — `S_Active`, `S_Child`, or both (§3.1). |
| 8 | `Sahamdar_Edit` | party CRUD | insert-vs-update semantics, and whether it creates the matching `Sarfasl` node (callers call `Sarfasl_Add` separately, which suggests not — §3.1). |
| 9 | `Moein_All` | year-end close | the definition of "closing balance of every account", to be **diffed against** the inline `#R` query at `EnteghalU.dfm:330-349`, which computes apparently the same thing differently (§3.1). Two implementations, one truth. |
| 10 | `Anbar_Mandeh` | stock valuation | the valuation rule, and what the two parallel quantity/value pairs (`R1/R2`, `TedIn1/2`, `Mabin1/2`, `Phiin1/2`) actually are — two units of measure or two warehouses (§3.1). |
| 11 | `Anbar_CardJensi` | stock card | the running-balance rule. |
| 12 | `Taraz4Setooni` | trial balance | which `M_Kind`/`M_Tx` rows count, and the meaning of `@St` and `@Ki`. |
| 13 | `Taraz_6Sotooni` | trial balance | the opening-balance rule, and confirmation that `@Sabt = 3` is the "all states" sentinel (`M_Tx ∈ {0,1,2}`, §9.7). |
| 14 | `Moein_View_Daftar` | subsidiary ledger | running-balance rule. |
| 15–27 | `MoeinViewSanad`, `MoeinTotalSanad`, `Moein_ChapSanad`, `Asnad_View`, `KolState`, `Sarfasl_view`, `Sarfasl_Seek_SSN`, `Sarfasl_Seek_Name`, `Select_Kol`, `Select_moein`, `Select_Taf1`, `Select_Taf2`, `Sahamdar_Seek`, `Sahamdar_Show`, `Anbar_AjnasView`, `Anbar_PrintFactor`, `Anbar_ReportKharidForoosh` | reporting endpoints | column lists and filter semantics; low risk but needed for report parity. |

Also dump **every procedure not referenced by this codebase** — jobs and admin scripts may be
maintaining columns the application only reads (`S_Child`, `FullName`, `M_L`, `M_R` are prime
suspects, since the client-side maintenance for all four is commented out, `Dmu.pas:274-296`).
**Resolved above:** `S_Child`, `FullName`, `M_L`, `M_R` (and `LineName`) are in fact maintained
server-side — by `Active_Set` (item 7), not by any client code — so the "who maintains these"
question is fully closed. `S_IS_Check`/`S_IS_Fish`/`S_IS_APArdakhti`/`S_IS_ADaryafti` remain
genuinely unmaintained by anything in this dump (see item 7).

### 12.4 User-defined functions that must be dumped

**RESOLVED — all bodies captured**, plus one function neither this table nor any `02-*`/`06-*`
document previously named: `dbo.Farsi_Date` (§12.1). §3.2 and §3.5 item 2.

| Function | Used at | Resolution |
|---|---|---|
| `dbo.Noto3(bigint)` | `CheckDaryaftU.pas:324`, inside the cheque narration | **Body captured — pure thousands-grouping, no rounding, no explicit negative-number handling.** `CAST(@INP AS varchar(20))` then four `IF Len(@Re)>N` steps insert a comma every 3 digits from the right (at lengths 3, 7, 11, 15 — i.e. groups of 3). No rounding logic exists because the input is already `bigint` (whole rial, no fractional part to round). No sign-aware branch: a negative value's leading `-` is just an extra character, which shifts the grouping boundary by one position relative to a same-magnitude positive value — a real, minor formatting quirk, not a crash. Whether `TDM.inttoStr3` (`Dmu.pas:859-867`) handles the sign the same way is a source-vs-source comparison still worth doing, but is no longer a dump question. |
| `dbo.Make_L(@coid,@ko,@mo,@ta1,@ta2)` | still read by `SanadMoeinu.dfm`, `TajmiU.dfm`, `RoyatJU.dfm` for `ORDER BY` | **Body captured.** Builds a zero-padded `Ko[-Mo[-Ta1[-Ta2]]]` string, left-to-right, padding each segment to `Base.No_Ko`/`No_Mo`/`No_Ta1`/`No_Ta2` width (looked up per `@Co`, up to 3 leading zeros added). `Make_R` builds the same segments **right-to-left** (`@S = @S + '-' + @K` instead of `@K + '-' + @S`), i.e. `Make_L` and `Make_R` are mirror images of each other, not independent formatters — worth noting `Moein_sumAsnad`'s body (an unreferenced extra, §12.3) assigns `M_R = Make_L(...)` and `M_L = Make_R(...)`, i.e. **swapped**, in that one procedure only; every other caller assigns them the "expected" way round. Since `Moein_sumAsnad` itself has no call site, this swap has no live effect but is worth a code-comment if that procedure is ever revived. |
| `dbo.Make_R(...)` | `ORDER BY`, and persisted as a UI preference `[Sarfasl_Select] MRL=M_R` (§8.1.2) | Body captured — see above, mirror of `Make_L`. |

### 12.5 Table DDL that must be dumped

**RESOLVED for every table that exists — with a significant inventory correction.**
`Full_Script_14050527.sql` contains 39 `CREATE TABLE` statements. Ten of them are developer scratch
tables with no relationship to the application schema — `temp_KS_48790`, `temp_KS_68`,
`temp_RJ_48790`, `temp_RJ_52918`, `temp_RJ_68`, `temp_RJ_70031`, `temp_RJ_86797`, `temp_RJ_86798`,
`temp1`, `TMP2` — left behind on the server from ad-hoc queries (their names embed what look like
user/session IDs). Excluding those, **29 real application tables** are confirmed, full DDL captured
for all of them:

`Anbar_Config`, `Anbar_Factor`, `Anbar_FactorD`, `Anbar_Jens`, `Anbar_Vahed`, `AnbarJens_B`, `Base`,
`Base_Config`, `CheckDetail`, `CheckMaster`, `DCheck`, `DCheck2`, `DFish`, `DMoein`, `mandeh_1404`,
`Moadian`, `Moein`, `Pass_Config`, `PassWord`, `POL_Namad`, `Rooznameh`, `Sahamdar`,
`SahamdarConfig`, `SahamdarInfo`, `Sarfasl`, `Tah1403`, `TankhahDetail`, `TankhahMaster`, `Tanzim`,
`TCheck`.

- **`TCheck`** — full DDL captured (`S_SSN` identity, `S_COID`, `S_State`, `S_StateName`,
  `S_CheckNo`, `S_Sanad`, `S_Date varchar(10)`, `S_DateS varchar(50)`, `S_Mab`, `S_Desc`,
  `S_BankSSN`/`S_BankCR`/`S_BankName`, `S_BedSSN varchar(200)`/`S_BedCR`/`S_BedName`,
  `S_Asnadssn`/`S_AsnadCR`/`S_AsnadName`, `S_UserID`). An extended property on `S_State` gives the
  full code list directly from the database, no data query needed: `1=check naghdi` (cash cheque),
  `2=check moedi`, `3=bardasht naghdi` (cash withdrawal), `4=bardasht ba kart` (card withdrawal),
  `11=daryaft check` (cheque received), `12=variz ba fish ya naghdi` (deposit by slip/cash),
  `13=variz ba kartkhan` (deposit by card reader) — **this reads like a general cash/bank movement
  type table, not a cheque-specific one**, resolving `06-treasury.md §12` Q21 (yes, real table) with
  a scope correction worth a look before modelling it as part of `cheques`.
- **`Anbar_Vahed`** — full DDL: `AV_SSN` identity, `AV_Code int NOT NULL PRIMARY KEY`,
  `AV_Name varchar(50)`.
- **`Kinds`** — **does not exist.** Not one of the 39 `CREATE TABLE` statements, under any
  case/spelling searched. Either it was dropped before this dump was taken, it never existed and
  was a documentation guess, or it lives in a different database. New open question — see
  `11-open-decisions.md`.
- **`TolidMaster`** — **confirmed absent**, resolving §12.14: the "proposed conclusion" there
  (design-time placeholder, not real schema) is correct — there is no such table to query a row
  count from.
- **`Moadian`** — full DDL captured: `M_SSN` identity, `M_ID tinyint`, `M_Link int NOT NULL`,
  `M_Date datetime`, `M_UserID int`, `M_CodeMelli varchar(15)`, `M_Inty tinyint`, `M_Tob tinyint`,
  `M_Name varchar(100)`, `M_Factorinno varchar(20)`, `M_TAXID/M_Status/M_REFID/M_UID/M_ERROR
  varchar(512)`, `M_OK tinyint`, `M_CodePosti varchar(15)` — a tax-authority (`مودیان`) e-invoice
  submission log, far more detailed than the two columns (`M_link`, `M_id`) previously known from
  `AnbarListU.pas:537`. No PK.
- **`Pass_Config`** — full DDL: `P_User int, P_ID int, P_Desc varchar(30)`. No PK, no defaults —
  fully known now (the P_ID→caption mapping itself is data, §12.9, still open).
- **`Anbar_Config`** — **resolved**: primary key is `AC_ID int NOT NULL PRIMARY KEY`. Full column
  list: `AC_Name varchar(50)`, `AC_Kharid/AC_Foroosh/AC_BKharid/AC_BForoosh/AC_Maliat int DEFAULT
  (0)`, `AC_Kasr int` (no default), `AC_DMaliat varchar(5) DEFAULT ('6.0')`.
- **`CheckDetail`'s and `TankhahDetail`'s identity columns** — **resolved: `CD_SSN` and `TD_SSN`
  respectively, both `int IDENTITY(1,1)`, and — new finding — neither is declared `PRIMARY KEY` in
  the DDL.** Same for `CheckMaster.CM_SSN` and `TankhahMaster.TM_SSN`: identity, but not a primary
  key. None of these four master/detail tables has any enforced key at all — see §12.6.
- **`BN_*` bank table** — **confirmed absent.** No table named `BN_*` or otherwise holding those
  columns exists in the 39 `CREATE TABLE` statements. Resolves `06-treasury.md §12` Q1 and §12.13
  below definitively: the `BankTanzim` form's grid columns describe a table that is not in this
  database. Either it was dropped, never created, or lives elsewhere — no evidence for any of those
  three from this dump alone.

**Three tables exist that no `02-*` or `06-*` document currently names at all** — new findings,
not previously flagged as open questions:

- **`mandeh_1404`** and **`Tah1403`** — both structurally near-identical to `Anbar_FactorD` and
  `Moein` respectively, but named after specific Persian fiscal years (1404, 1403). `Tah1403`'s
  columns are an exact copy of `Moein`'s (`M_SSN` PK identity, `M_COID`, `M_Sanad`, `M_Date`,
  `M_Bed`, `M_Bes`, `M_Ted`, `Article`, `M_Tx`, `M_Ko`, `M_Mo`, `M_Ta1`, `M_Ta2`, `M_Id`, `M_Link`,
  `M_User`, `M_Kind`, `M_Code`, `M_time`), carrying the same extended-property comments on `M_Id`
  and `M_Link` as `Moein` itself. `mandeh_1404`'s columns match `Anbar_FactorD` minus `AFD_VahedC`.
  **This means the legacy database uses per-fiscal-year physical table copies for at least some
  data**, not (or not only) `CO_ID`/`AFD_COID` scoping within one shared table — a materially
  different fiscal-year architecture than §12.12 and the rest of `02-*` assumed. New open item, see
  `11-open-decisions.md`.
- **`POL_Namad`** — 39 columns, denormalised (`NK_Name`, `NM_Name`, `Ko_Name`… pre-joined display
  names alongside the codes), `DebitAmount`/`CreditAmount decimal(28,4)` (the only `decimal`
  columns in the whole schema — every other money column is `bigint`). Read only by the
  unreferenced-by-app procedure `Pol_Select` (§12.3), which has commented-out joins to a *separate*
  database (`Account.dbo.AcnAccounts`). Looks like an import/export staging table for integration
  with another system (`نماد` = stock-market ticker symbol) — plausibly related to the
  `Rppc_Solution`/`Saham` cross-database question in §12.12. Not part of the application's own
  transactional schema; flagged, not modelled in §11.

### 12.6 Constraints and indexes — absent, or merely invisible?

**RESOLVED — mostly absent, confirmed, with two real exceptions.** The dump's DDL is definitive
here (constraints and indexes are schema, not data — the dump does not need rows to answer this).
Findings across all 29 real tables:

- **Zero `FOREIGN KEY` declarations anywhere** (`grep FOREIGN KEY|REFERENCES` on the whole dump: no
  matches). §11's "no FKs exist today" premise is confirmed, not just inferred.
- **Zero named `DEFAULT` constraints, zero `CHECK` constraints.** `DEFAULT` values exist (e.g.
  `AC_Kharid int DEFAULT ((0))`) but are anonymous/inline, matching what T-SQL generates when a
  default is added without a name — not evidence of hand-designed data integrity.
- **Two real, enforced composite primary keys exist** — a genuine exception to "nothing is
  enforced": `Sarfasl` has `CONSTRAINT [PK_Sarfasl_1] PRIMARY KEY (S_Ko, S_Mo, S_Ta1, S_Ta2)`, and
  `SahamdarConfig` has an unnamed-by-hand (designer-generated name
  `PK__Sahamdar__1B0CAE6BDF059AFA`) `PRIMARY KEY (SC_K, SC_M, SC_T)`. **This confirms the chart of
  accounts' natural key really is DB-enforced** — duplicate `(Ko,Mo,Ta1,Ta2)` rows are impossible,
  which also explains why `Sarfasl_ADD`'s duplicate check (§12.3 item 5) never actually needs to
  race against a concurrent insert of the same key; the constraint is the backstop.
- Besides those two, single-column `PRIMARY KEY` exists on: `Anbar_Config.AC_ID`,
  `Anbar_FactorD.AFD_SSN`, `Anbar_Jens.AJ_Code` (not `AJ_SSN` — the surrogate identity is not the
  key), `Anbar_Vahed.AV_Code`, `Base.CO_ID`, `Moein.M_SSN`, `Sahamdar.S_Card` (**not `S_SSN`** — see
  §12.15 below), `Tah1403.M_SSN`, `Tanzim.T_ID`.
- **No primary key at all** — despite having an `IDENTITY` column that looks like one — on:
  `Anbar_Factor` (`AF_SSN`), `Base_Config`, `CheckDetail` (`CD_SSN`), `CheckMaster` (`CM_SSN`),
  `DCheck` (`S_SSN`), `DCheck2` (`S_SSN`), `DFish` (`S_SSN`), `DMoein` (`DM_SSN`), `Moadian`
  (`M_SSN`), `Pass_Config`, `PassWord`, `POL_Namad`, `Rooznameh` (`R_SSN`), `SahamdarInfo`
  (`SI_SSN`), `TankhahDetail` (`TD_SSN`), `TankhahMaster` (`TM_SSN`), `TCheck` (`S_SSN`). Notably
  **`PassWord` has no primary key and no unique constraint on `UserName`** — nothing at the DB level
  stops two rows with the same username, matching §12.11's duplicate-username concern but now
  confirmed as a schema fact rather than an inference from missing client-side error handling.
- **Three indexes exist, all on `Sarfasl`, all single-column, all nonclustered**:
  `Sarfasl_Adaryafti (S_IS_ADaryafti)`, `Sarfasl_Check (S_IS_Check)`, `Sarfasl_Fish (S_IS_Fish)`.
  This is a genuinely new and load-bearing finding: **these three columns are indexed, which is
  something a DBA does deliberately for a query that filters on them** — yet §12.3 item 7 confirms
  no procedure in this dump ever writes to any of the four `S_IS_*` columns, and the app-side
  assignments are commented out too. Indexed-but-unwritten is a stronger signal than
  unwritten-alone: either something outside this dump writes them (another tool, a manual process,
  a since-removed feature), or they are vestigial indexes left over from a feature that was fully
  removed, including its writer, but not its index. `S_IS_APArdakhti` has a `DEFAULT (0)` but **no
  index** — the odd one out of the four. New open item, see `11-open-decisions.md`.
- No index exists on any other table — no index backs the FK-shaped links either (e.g.
  `CheckDetail.CD_CMSSN` → `CheckMaster.CM_SSN` has neither a constraint nor an index).

Confirm before §11's constraints are treated as "new" — the original query block is retained below
for reference; it has been superseded by the findings above:

```sql
SELECT t.name AS tbl, kc.name, kc.type_desc FROM sys.key_constraints kc
  JOIN sys.tables t ON t.object_id = kc.parent_object_id;
SELECT * FROM sys.foreign_keys;
SELECT t.name, cc.name, cc.definition FROM sys.check_constraints cc
  JOIN sys.tables t ON t.object_id = cc.parent_object_id;
SELECT t.name AS tbl, i.name, i.type_desc, i.is_unique,
       STRING_AGG(c.name, ',') WITHIN GROUP (ORDER BY ic.key_ordinal) AS cols
FROM sys.indexes i
  JOIN sys.tables t        ON t.object_id  = i.object_id
  JOIN sys.index_columns ic ON ic.object_id = i.object_id AND ic.index_id = i.index_id
  JOIN sys.columns c        ON c.object_id  = i.object_id AND c.column_id = ic.column_id
WHERE t.is_ms_shipped = 0
GROUP BY t.name, i.name, i.type_desc, i.is_unique;
```

The index list is doubly valuable: it reveals the **real access paths** and therefore which of
§11's proposed indexes already exist.

### 12.7 Triggers — cannot be ruled out, and `@@IDENTITY` depends on it

**RESOLVED. Exactly one trigger exists in the entire database**, and — reassuringly — it does not
create the `@@IDENTITY` hazard the question worried about:

```sql
CREATE TRIGGER [dbo].[Jens_Update] ON  [dbo].[Anbar_Jens]
   AFTER DELETE,UPDATE
AS
BEGIN
	SET NOCOUNT ON;
    insert AnbarJens_B
       Select * From Deleted
END;
```

This is a **soft-delete/change-history trigger**, previously undocumented anywhere in `02-*` or
`05-inventory.md`: every `DELETE` or `UPDATE` against `Anbar_Jens` (the item master) copies the
**pre-change row** into `AnbarJens_B` (a structurally identical shadow table, itself confirmed to
exist in the dump, no PK, no `IDENTITY`). This means **item edits and deletes are already
audited server-side**, silently, and have been for as long as this trigger has existed — a fact
none of the inventory documentation currently states. Since `AnbarJens_B.AJ_SSN` is a plain `int`
column (not `IDENTITY`), the trigger inserts a copy of the identity value from `Deleted`, not a new
one — so it generates **no new identity value** and does not touch `@@IDENTITY` at all. The
original concern (an insert-time trigger silently consuming the session's next identity value) does
not materialize for any table in this dump: no trigger fires on `INSERT` anywhere. **§2's "the
application maintains this" claims stand for every table except `Anbar_Jens`'s change history**,
which is server-maintained. New open item (why was this never documented, does the app read
`AnbarJens_B` anywhere) — see `11-open-decisions.md`.

### 12.8 Server and column collation

**Partially resolved.** `sys.databases.collation_name` and `sys.servers` are server-level metadata
that a schema-only `CREATE TABLE`/`CREATE PROCEDURE` script export does not carry — genuinely still
open, needs a live connection. However, **column-level collation overrides are schema, and this
dump proves there are none**: zero `COLLATE` clauses anywhere in 2,955 lines. Every text column
uses the database's default collation — consistent with (but not proof of) case-insensitivity,
since a case-sensitive *default* would still show no per-column `COLLATE` clause. The
`Sarfasl_Add`/`Sarfasl_ADD` and `Taraz4Setooni`/`Taraz_6Sotooni` case-consistency question is
narrowed to "what is the server's default collation", not "did someone override collation
per-column" — the latter is now ruled out.

**Also resolved: `varchar` vs `nvarchar`.** Every text column in every one of the 29 real tables is
`varchar` — **zero `nvarchar` columns exist in this schema**, including `SahamdarConfig.SC_Name`
which is the sole exception at `nvarchar(100)` (worth double-checking, since it is the only
Unicode-capable text column in the whole database — everything else, including every Persian name,
address, and narration field, is `varchar`, i.e. stored in the database's single-byte code page,
not Unicode). This confirms the `varchar(1256)`/CP1256 transcoding concern is real and not
hypothetical: essentially the entire Persian text estate needs code-page-aware transcoding during
migration, with `SahamdarConfig.SC_Name` as the one column that was already Unicode-safe.


---

[← 02-11-h-ddl-compliance-and-deferred.md](02-11-h-ddl-compliance-and-deferred.md) | [02-12-b-open-questions-schema-and-volume.md →](02-12-b-open-questions-schema-and-volume.md)
