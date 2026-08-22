_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 3.2 Server-side user-defined functions

Two scalar UDFs appear, both in **commented-out** maintenance code at `Dmu.pas:274` and `:278`:

```pascal
//  UpdateQ.SQL.Add('Update sarfasl set M_L = Dbo.Make_L( '+inttostr(CO_ID)+', S_ko, S_Mo, S_ta1, S_ta2 )   ');
//  UpdateQ.SQL.Add('Update sarfasl set M_R = Dbo.Make_R( '+inttostr(CO_ID)+', S_ko, S_Mo, S_ta1, S_ta2 )   ');
```

`dbo.Make_L(@coid, @ko, @mo, @ta1, @ta2)` and `dbo.Make_R(...)` produce the `M_L` / `M_R` columns on
`Sarfasl` (`varchar(25)`/`varchar(50)`, `SanadMoeinu.dfm`, `TajmiU.dfm`, `RoyatJU.dfm`). Given the
names and that they are used for `ORDER BY` and display (`Sarfasl_Select` persists
`MRL=M_R` as a UI preference, §8.1.2), they are almost certainly **left-padded / right-padded
sort keys** for the 4-segment account code, built from the per-year widths in `Base.NO_Ko…NO_Ta2`.
They are still referenced by live queries even though the maintenance update is disabled.

A third UDF is used in **live** code: `dbo.Noto3(<bigint>)` (`CheckDaryaftU.pas:324`), inside the
narration builder for a received cheque:

```sql
Set @SBed = ( Select 'بابت دریافت  چک شماره ' + S_CheckNo + ' مبلغ ' + dbo.Noto3(S_Mab)
              + ' سررسید ' + S_DateS + ' توسط ' + S_BesName  from Dcheck Where S_SSn=@SSN)
```

*("for receipt of cheque number &lt;n&gt;, amount &lt;m&gt;, due &lt;d&gt;, from &lt;party&gt;")*.
`Noto3` is the **server-side twin of `TDM.inttoStr3`** (`Dmu.pas:859-867`, §7.5) — a `bigint` →
comma-grouped string formatter. Its exact grouping and negative-number behaviour must be compared
with the Delphi version; if they differ, printed narrations and screen totals disagree.

`master.dbo.xp_fileexist` (`Mainu.pas:409`) is a built-in undocumented extended procedure, not
application code (§10.1).

**Add `dbo.Noto3` to the extraction list in §3.5 item 2.**

### 3.3 Classification summary

| Procedure | Kind | Business rules? | Priority to extract |
|---|---|---|---|
| `MoeinAdd` | write | **yes** — parses string amounts, resolves `M_Code`, maintains `DMoein` | **1** |
| `MakeSanad_CheckDaryafti` | write | **yes** — cheque posting logic | **1** |
| `XNew` | read | **yes** — the live Jalali conversion | **1** |
| `Anbar_AddToFactor` | write | **yes** — invoice header roll-up | **1** |
| `Sahamdar_Edit` | write | **yes** — upsert semantics | **2** |
| `Sarfasl_ADD` | write | **yes** — `S_Child`, `FullName`, `M_L`/`M_R` | **2** |
| `Sarfasl_Deep` | write? | **yes** — delete-safety checks | **2** |
| `Active_Set` | write | **yes** — derived-flag recomputation | **2** |
| `Moein_All` | read | **yes** — year-end balance definition | **2** |
| `Anbar_Mandeh` | read | **yes** — inventory valuation | **2** |
| `Anbar_CardJensi` | read | **yes** — stock running balance | **2** |
| `Taraz4Setooni` | read | partly — filter semantics | **3** |
| `Taraz_6Sotooni` | read | partly — opening-balance rule | **3** |
| `Moein_View_Daftar` | read | partly — running balance | **3** |
| `Anbar_ReportKharidForoosh` | read | no | 4 |
| `Anbar_PrintFactor` | read | no | 4 |
| `Anbar_AjnasView` | read | no | 4 |
| `Moein_ChapSanad` | read | no | 4 |
| `MoeinViewSanad`, `MoeinTotalSanad` | read | no | 4 |
| `Asnad_View` | read | no | 4 |
| `KolState` | read | no | 4 |
| `Sarfasl_view` | read | no | 4 |
| `Sarfasl_Seek_SSN`, `Sarfasl_Seek_Name` | read | no | 4 |
| `Select_Kol/moein/Taf1/Taf2` | read | no | 4 |
| `Sahamdar_Seek`, `Sahamdar_Show` | read | no | 4 |
| `B_SelectSerial` | read | external | n/a |

### 3.4 Cross-cutting observations

1. **Date parameters are inconsistently sized.** `@ToDate`/`@D1`/`@D2` are `varchar(8)` on
   `Taraz4Setooni` and `Moein_View_Daftar`, `varchar(10)` on `Anbar_CardJensi` and
   `Taraz_6Sotooni`, `varchar(12)` on one declaration of `Anbar_ReportKharidForoosh`. Callers pass
   the 10-character form. **Silent truncation to 8 characters turns `'1403/05/27'` into
   `'1403/05/'`** — which then compares wrong lexicographically (§6.5). Any report using those two
   procedures must be re-tested against live data before it is trusted as a porting reference.
2. **Money crosses the boundary as text** (`MoeinAdd.@bed/@bes varchar(20)`) and quantity as
   **float** (`Anbar_AddToFactor.@Num ftFloat`) — the two worst possible choices, and both must be
   fixed in the rebuild (typed `i64` / `Decimal`).
3. **Fiscal-year scoping is not uniform.** `Asnad_View`, `Moein_ChapSanad` (in `RoozViewU.dfm`) and
   `Anbar_ReportKharidForoosh` take **no** `@Coid`. Either they scope internally off a default, or
   they return cross-year data. This is a correctness question, not a style question.
4. **Design-time defaults are stale production values** (`1396`, `1397`, `1398`, `1403`), which is
   how the fiscal-year interpretation of `CO_ID` was confirmed (§1.4).
5. Procedure **naming is inconsistent** — `Taraz4Setooni` vs `Taraz_6Sotooni`, `Sarfasl_Add` vs
   `Sarfasl_ADD`, `Moein_View_Daftar` vs `MoeinViewSanad`. Under a case-sensitive or accent-
   sensitive collation some of these calls would fail. Confirm the server collation (§12).

### 3.5 What must be dumped from a live database

None of the following exists in this repository. Before implementation starts, obtain from a
production (or recent restored) database:

1. **`CREATE PROCEDURE` text for all 27 procedures** listed in §3.1–§3.2:
   ```sql
   SELECT OBJECT_SCHEMA_NAME(object_id) AS s, name, OBJECT_DEFINITION(object_id) AS body
   FROM sys.procedures ORDER BY name;
   ```
   plus any procedure **not** referenced by this codebase (jobs, ad-hoc admin scripts).
2. **`CREATE FUNCTION` text** for `dbo.Make_L` and `dbo.Make_R`, and any other UDF:
   ```sql
   SELECT name, OBJECT_DEFINITION(object_id) FROM sys.objects WHERE type IN ('FN','IF','TF');
   ```
3. **Full DDL for every table** — `sys.columns` with types, lengths, precision/scale, nullability,
   `is_identity`, identity seed/increment, and **`sys.default_constraints`**. §2 infers these from
   Delphi field metadata; the authoritative types must come from the server.
4. **All constraints**: primary keys, unique indexes, foreign keys, check constraints
   (`sys.key_constraints`, `sys.foreign_keys`, `sys.check_constraints`). §2 and §5 assert that
   several appear to be **absent**; that must be confirmed, not assumed.
5. **All indexes** (`sys.indexes` + `sys.index_columns`) — they reveal the real access paths and
   therefore the intended query patterns.
6. **All triggers** (`sys.triggers` + `OBJECT_DEFINITION`). §5.2 flags `@@IDENTITY` as unsafe *if*
   triggers exist; nothing in the source proves they do not.
7. **All views** (`sys.views`).
8. **Database and column collations** (`sys.databases.collation_name`,
   `sys.columns.collation_name`) — §6.5 and §3.4 both depend on collation behaviour.
9. **The catalogs `Anbar`, `Saham` and `Rppc_Solution`** (§1.5), at least for the tables this
   application touches: `Cala`, `Anbar`, `FactorMaster`, `FactorDetail`, `NSaham`, `NewRamz`, and
   the procedure `B_SelectSerial`.
10. **Row counts and value-domain profiling** for every enumerated column
    (`M_Kind`, `M_Tx`, `M_ID`, `S_State`, `AF_Type`, `S_Kind`, `FM_InOut`, …) — the source shows
    which values are *written*, never the full set that *exists*.
11. **A representative data extract** for the migration checks named in §6.8, §7.7 and §10.8.


---

[← 02-03-b-stored-procedures-continued.md](02-03-b-stored-procedures-continued.md) | [02-04-a-adhoc-sql-schema-and-startup.md →](02-04-a-adhoc-sql-schema-and-startup.md)
