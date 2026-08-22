_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 11. Index of all SQL in the accounting core

Every SQL statement in scope is quoted verbatim in the section shown. This is the index; use it as
the checklist when porting.

| # | Purpose | `file:line` | Quoted in |
|---|---|---|---|
| 1 | Recompute `S_Child` for all four levels | `Dmu.pas:303-314` | §1.6 |
| 2 | Add a column if missing (`ALTER TABLE` guard) | `Dmu.pas:326-327` | below |
| 3 | Rebuild `FullName` from the name path (disabled) | `Dmu.pas:284-295` | §1.4 |
| 4 | Rebuild `M_R` / `M_L` via `dbo.Make_R` / `dbo.Make_L` (disabled) | `Dmu.pas:274-279` | §1.4 |
| 5 | Upsert a voucher header from its lines | `Dmu.pas:820-836` | §3.5 |
| 6 | Recompute header totals; delete empty header | `Dmu.pas:846-855` | §3.5 |
| 7 | Next voucher number | `Dmu.pas:1247` | §3.7 |
| 8 | Next inventory invoice number | `Dmu.pas:1258` | below |
| 9 | Reuse a same-day draft voucher for generated documents | `Dmu.pas:1467-1471` | §3.7 |
| 10 | Voucher date lookup | `Dmu.pas:1486-1488` | below |
| 11 | Voucher exists? | `Dmu.pas:1529-1531` | below |
| 12 | Max `M_Tx` on a voucher | `Dmu.pas:1544-1546` | below |
| 13 | Validate a generated voucher's date + id list | `Dmu.pas:1515-1517` | below |
| 14 | Max `M_Tx` on a voucher (alt.) | `Dmu.pas:1171-1173` | below |
| 15 | Permission check | `Dmu.pas:1557-1558` | §13 |
| 16 | Account-lock ancestor walk (4 queries) | `Dmu.pas:929-960` | §2.5 |
| 17 | Voucher-lock check | `Dmu.pas:990` | §3.6 |
| 18 | Current-account (Jari) party resolution | `Dmu.pas:1393`, `:1412-1413`, `:1423` | §1.11 |
| 19 | Leaf test by tuple | `Dmu.pas:1026-1029` | §1.6 |
| 20 | Leaf test by id | `Dmu.pas:1044`, `:1056-1059` | §1.6 |
| 21 | Delete a voucher (probe + delete) | `Dmu.pas:1289`, `:1318`, `:1320` | §4.5 |
| 22 | Delete a voucher line (probe + delete) | `Dmu.pas:1335`, `:1363` | §4.6 |
| 23 | Detect the presence of the auxiliary databases | `Dmu.pas:765-773` | §6.2 |
| 24 | Find the neighbouring voucher of the same class | `Dmu.pas:872-877` | below |
| 25 | Chart of accounts, Kol level | `SNewu.pas:485-489` | §12.2 |
| 26 | Chart of accounts, Moein level (with zero-padding) | `SNewu.pas:499-506` | §12.2 |
| 27 | Chart of accounts, Tafsil-1 level | `SNewu.pas:517-525` | §12.2 |
| 28 | Chart of accounts, Tafsil-2 level | `SNewu.pas:536-540` | §12.2 |
| 29 | Next account code, four variants | `SNewu.pas:553-591` | §2.1 |
| 30 | "Has postings?" test for an account | `SNewu.pas:134-143` | §2.3 |
| 31 | Delete an account | `SNewu.pas:182-188` | §2.4 |
| 32 | Renumber an account, four variants | `SNewu.pas:274-304` | §2.3 |
| 33 | Rename an account + dirty-mark + `Active_Set` | `SNewu.pas:332-348` | §2.2 |
| 34 | Toggle the account lock | `SNewu.pas:365` | §2.5 |
| 35 | Register a special-role account in `base_config` | `SNewu.pas:721-732` | §1.9 |
| 36 | Resolve a Kol code to an account | `EditArticleMoeinU.pas:178-179` | §5.6 |
| 37 | Resolve a Moein code | `EditArticleMoeinU.pas:306-308` | §5.6 |
| 38 | Resolve a Tafsil-1 code | `EditArticleMoeinU.pas:214-216` | §5.6 |
| 39 | Resolve a Tafsil-2 code | `EditArticleMoeinU.pas:263-265` | §5.6 |
| 40 | Leaf-check + id lookup for a line | `EditArticleMoeinU.pas:134-140` | §1.6 |
| 41 | Account browse list, four levels | `SelectSarfasl.pas:93-95`, `:110-112`, `:127-129`, `:144-146` | §5.6 |
| 42 | Postable-account picker (all) | `Sarfasl_SelectU.pas:206-208` | §12.6 |
| 43 | Person current-account picker | `Sarfasl_SelectU.pas:107-111` | §12.6 |
| 44 | `103-1` picker | `Sarfasl_SelectU.pas:85-87` | §12.6 |
| 45 | `109-1/2` picker | `Sarfasl_SelectU.pas:96-98` | §12.6 |
| 46 | Role-account pickers via `base_config` (4 variants) | `Sarfasl_SelectU.pas:225-228`, `:238-241`, `:251-254`, `:269-272` | §1.9 |
| 47 | Account lookup by id | `Sarfasl_SelectU.pas:323-324` | below |
| 48 | Voucher header lookup | `SanadEditU.pas:430-431` | §4.1 |
| 49 | Voucher lines + account display name | `SanadEditU.pas:373-376` | §4.1 |
| 50 | Delete manual lines before re-save | `SanadEditU.pas:618-620` | §3.4 |
| 51 | Insert a manual voucher line | `SanadEditU.pas:626-630` | below |
| 52 | Header state probe before edit | `SanadEditU.pas:797`, `:807` | §3.6 |
| 53 | Voucher header list by state | `SanadViewU.pas:723-725` | §12.3 |
| 54 | Approve a voucher range (0→1) | `SanadViewU.pas:292-304` | §3.6 |
| 55 | Return a voucher range to draft (1→0) | `SanadViewU.pas:447-458` | §3.6 |
| 56 | Post a voucher range permanently (1→2) | `SanadViewU.pas:483-494` | §3.6 |
| 57 | Return a voucher range to approved (2→1) | `SanadViewU.pas:522-533` | §3.6 |
| 58 | Change a voucher number across 8 tables | `SanadViewU.pas:338`, `:348-366` | §12.3 |
| 59 | Change a voucher date across 9 tables | `SanadViewU.pas:397-420` | §12.3 |
| 60 | Voucher delete probe + delete | `SanadViewU.pas:224-225`, `:247-248` | §4.5 |
| 61 | Copy-eligibility probe | `SanadViewU.pas:667-668` | §12.3 |
| 62 | Merge two vouchers | `MergeSanad.pas:118-136` | §7.2 |
| 63 | Merge dialog header lookup | `MergeSanad.pas:184`, `:254` | §7.1 |
| 64 | Merge dialog source-class probe | `MergeSanad.pas:203-204`, `:273-274` | §3.4 |
| 65 | Delete previously generated inventory lines | `MakeSanadU.pas:84-85` | §6.6 |
| 66 | Insert a generated voucher line | `MakeSanadU.pas:92-93` | §6.6 |
| 67 | Back-fill the account tuple from `M_Code` | `MakeSanadU.pas:112-114` | §6.6 |
| 68 | Mark the inventory document as posted | `MakeSanadU.pas:121-122` | §6.6 |
| 69 | Inventory document header lookup | `MakeSanadU.pas:197-198` (and `:332`, `:467`, `:601`) | below |
| 70 | Warehouse account configuration lookup | `MakeSanadU.pas:213` (and `:348`, `:483`, `:617`) | §6.4 |
| 71 | Inventory document list with settlement totals | `SodoorSanadU.pas:340-356` | §6.2 |
| 72 | Un-post an inventory document | `SodoorSanadU.pas:250-262` | §6.7 |
| 73 | Journal source range probe | `MoeinToRU.pas:170-171` | §8.1 |
| 74 | Journal duplicate-number check | `MoeinToRU.pas:146` | §8.1 |
| 75 | Generate a journal voucher | `MoeinToRU.pas:192-220` | §8.1 |
| 76 | Next journal voucher number | `MoeinToRU.pas:264` | §8.1 |
| 77 | Legacy journal aggregation | `MakeRooznamehU.pas:95-99` | §8.2 |
| 78 | Legacy journal completeness probe | `MakeRooznamehU.pas:67-70` | §8.2 |
| 79 | Legacy journal duplicate check | `MakeRooznamehU.pas:82-83` | §8.2 |
| 80 | Journal voucher list | `RooznamehViewU.pas:138` | §8.3 |
| 81 | Journal voucher print dataset | `RooznamehViewU.B_PrintClick` | §8.4 |
| 82 | Candidate Kol accounts for closing | `NewFinalu.dfm:358-376` | §9.2 |
| 83 | Closing aggregation to leaf level | `NewFinalu.pas:162-179` | §9.2 |
| 84 | Closing-entry insert (parameterised) | `NewFinalu.dfm:323-343` | §9.2 |
| 85 | Closing back-fill of the account tuple | `NewFinalu.pas:218-223` | §9.2 |
| 86 | Closing voucher existence probe | `NewFinalu.pas:95-96` | §9.2 |
| 87 | Carry-forward: next-year existence | `EnteghalU.pas:94` | §9.3 |
| 88 | Carry-forward: all-vouchers-posted check | `EnteghalU.pas:104` | §9.3 |
| 89 | Carry-forward: closing/opening voucher duplicate checks | `EnteghalU.pas:120`, `:137` | §9.3 |
| 90 | Carry-forward: fiscal-year date range lookups | `EnteghalU.pas:154`, `:173` | §9.3 |
| 91 | Carry-forward driving query | `EnteghalU.dfm:659-695` | §9.3 |
| 92 | Carry-forward: four inserts per account | `EnteghalU.pas:250-276` | §9.3 |
| 93 | Consolidated ledger: parent list | `TajmiU.pas:89-92` | §10 |
| 94 | Consolidated ledger: child aggregation | `TajmiU.pas:102-119` | §10 |
| 95 | Manual balance-reversal fill | `SanadMoeinu.pas:446-450` | §9.7 |
| 96 | Journal-voucher import into a voucher | `SanadMoeinu.pas:299-332` (dataset writes) | §9.6 |
| 97 | Voucher-line search | `MoeinSearchU.pas:90-141` | §12.7 |
| 98 | Chart-of-accounts drill-down (dead unit) | `Sarfasl_Kolu.pas:105-120` | below |
| 99 | Debtors/creditors report parameters | `BedBes.pas:100-107` | §12.8 |
| 100 | Legacy account list by company | `Sarfasl_ListU.pas:44` | §1.7 |

### 11.1 Statements not quoted elsewhere

**#2 — add a column if missing** (`Dmu.pas:326-327`). Runs at start-up through
`Dm.CreateFieldInTable`; every call site is currently commented out (`Dmu.pas:249-270`).

```sql
IF COL_LENGTH('<table>', '<column>') IS NULL
ALTER TABLE <table> ADD [<column>] <type>
```

This is a **schema-migration-on-launch** pattern. Replace it with proper migrations.

**#8 — next inventory invoice number** (`Dmu.pas:1258`):

```sql
  Select isnull(Max( AF_Factor ),0 )+1 as NewFactor From Anbar_factor Where AF_COid=<CO_ID>
```

**#10 — voucher date** (`Dmu.pas:1486-1488`):

```sql
 Select isnull( Max(M_Date),'' ) as F_Date From Moein
   Where M_Coid=<CO_ID>
   and M_sanad=<n>
```

**#11 — voucher exists** (`Dmu.pas:1529-1531`):

```sql
 Select * From Moein
   Where M_Coid=<CO_ID>
   and M_sanad=<n>
```

**#12 / #14 — max state on a voucher** (`Dmu.pas:1544-1546` and `Dmu.pas:1171-1173`):

```sql
 Select isnull( Max(M_Tx),0) as TX From Moein
   Where M_Coid=<CO_ID>
   and M_sanad=<n>
--
  Select isnull( max(M_TX) , 0)  As TX  From Moein
   Where M_Sanad =<n>
   And M_COID=<CO_ID>
```

Two identical helpers (`Get_SanadMaxTX` and `Moein_Tx`). Collapse to one in the rebuild.

**#13 — validate a generated voucher** (`Dmu.pas:1515-1517`). Used by `Get_SanadDateID_Valid`, which
has **no callers**:

```sql
 Select * From Moein
   Where M_Coid=<CO_ID> and M_Sanad=<n>
   and M_ID in (<idList>)
```

**#24 — find the neighbouring voucher of the same source class** (`Dmu.pas:872-877`). Used by
`GetRefreshSanad`, which also has no callers. Contains a bug — the third statement hard-codes
`M_tx=1` instead of using the `tx` parameter:

```sql
Declare @N int  Set @N = 0
if Exists( Select * From moein Where M_tx=<tx> and M_sanad<<n>)
Set @N = (Select min(M_sanad) from moein where m_tx = <tx> and M_sanad <<n>)
if Exists( Select * From moein Where M_tx=1 and M_sanad><n>)      -- <<< should be @tx
Set @N = (Select min(M_sanad) from moein where m_tx = <tx> and M_sanad ><n>)
Select @N As N
```

**#47 — account lookup by id** (`Sarfasl_SelectU.pas:323-324`):

```sql
 Select Sarfasl.* From Sarfasl
    Where S_SSN=<id>
```

Also at `TarafU.pas:241` and `Dmu.pas:1044`, `Dmu.pas:1393`, `Dmu.pas:1450`.

**#51 — insert a manual voucher line** (`SanadEditU.pas:626-630`):

```sql
 Insert Moein (M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, Article, M_Tx, M_Ko, M_Mo,
     M_Ta1,  M_Ta2, M_Id, M_Link, M_User, M_Kind, M_Code, M_Time)
  Values( <coid>, <sanad>, '<jalali>'
     , :Bed, :Bes, :Ted, :Article, 0, :Ko, :Mo, :Ta1, :Ta2, 0, 0, <userId>
     , 1, :Code, GetDate() )
```

Note the hard-coded `M_Tx = 0`, `M_Id = 0`, `M_Link = 0`, `M_Kind = 1`.

**#69 — inventory document header** (`MakeSanadU.pas:197-198`):

```sql
Select * From Anbar.DBO.FactorMaster
  Where FM_SSN=<id>
```

**#98 — chart-of-accounts drill-down** (`Sarfasl_Kolu.pas:105-120`, **dead unit** — its form is never
shown). Recorded because it demonstrates a three-level drill-down variant where level 3 shows the Kol
header alongside the Moein's Tafsil children:

```sql
 Select * from Sarfasl
 -- level 1:
 Where S_mo=0
 -- level 2:
 Where S_ta1=0 and S_ko=<kol>
 -- level 3:
 where (S_Mo=0 and S_ko=<kol>)
 or ( S_ko=<kol> and s_mo=<moein> and S_ta2=0)
 Order by S_ko, S_mo, S_ta1, S_Ta2
```

### 11.2 SQL construction hazards to eliminate in the rebuild

Every query above is built by **string concatenation**. Specific hazards:

1. **SQL injection.** Free-text goes into SQL unescaped in several places — most clearly
   `MoeinSearchU.pas:135`:
   ```pascal
   S1 := S1+ ' and len(lTrim(Article))>0 And (  Article Like ''%'+Trim(Desc1.Text)+'%'' ) ';
   ```
   A description containing `'` breaks or subverts the query. `QuotedStr` is used in some places
   (`EnteghalU.pas:253`, `MoeinToRU.pas:196`) but not consistently.
2. **The `'0' + text` idiom** (`EditArticleMoeinU.pas:179`, `:216`, `:265`, `:308`) — an empty edit box
   must not produce `S_Ko=`.
3. **Multi-statement batches with `Begin Transaction` / `Commit` embedded in the string.** If any
   statement fails, the client sees an exception but the transaction state is undefined. Several
   routines (§6.6) have no transaction at all.
4. **`AsInteger` on `bigint` sums** (`MakeRooznamehU.pas:108`) — 32-bit overflow.
5. **String comparison of numeric strings** (`BastanHesab.pas:57`) and of formatted totals
   (`SanadEditU.pas:604`).

---

_Prev: [03-10-aggregation-consolidation-tajmiu-pas](03-10-aggregation-consolidation-tajmiu-pas.md) | Next: [03-12-a-screen-by-screen-ui-specification](03-12-a-screen-by-screen-ui-specification.md)_
