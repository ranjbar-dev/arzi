_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 12.5 The readable SQL — complete inventory

Every SQL statement in the inventory domain that is *not* behind a stored procedure. Quoted in
full in the sections listed; here as a cross-reference index with intent.

#### 12.5.1 Balance and costing

| SQL | Location | Intent | Quoted at |
|---|---|---|---|
| `Anbar_MandehU.Q1` — the `#R` temp-table report | `Anbar_MandehU.dfm:1613-1727` | opening + four movement buckets + closing, per item, per date window | §5.1.2 |
| `Anbar_Jens_Phi1` | `Dmu.dfm:635-720` | on-hand and average purchase price for one item, excluding the invoice being edited | §5.1.3 |
| `AnbarFactorU.Q1` | `AnbarFactorU.dfm:587-624` | **dead** third balance implementation, `AFD_Coid` hard-coded to `1400` | §5.1.4 |

#### 12.5.2 Invoice lifecycle

| SQL | Location | Intent |
|---|---|---|
| `Select * From Anbar_Factor Where AF_Factor=<n> And AF_COID=<y>` | `AnbarFactorU.pas:704` | load header |
| `Select * From Anbar_FactorD Where AFD_Factor=<n> And AFD_COID=<y>` | `AnbarFactorU.pas:719` | load lines |
| `Delete Anbar_FactorD …` + `Delete Moein … M_ID=1 and M_Link=<n>` | `AnbarFactorU.pas:620-621` | destroy the previous version before re-inserting (§4.2.2 step 7) |
| four correlated `Update Anbar_Factor Set AF_Total/AF_Kasr/AF_Maliat/AF_Mab = …` | `AnbarFactorU.pas:654-661` | recompute the header caches (§4.2.2 step 10) |
| `Select isnull(Max(AF_Factor),0)+1 as NewFactor From Anbar_factor Where AF_COid=<y>` | `Dmu.pas:1253-1262` | allocate an invoice number — unlocked (§5.4) |
| `Select isnull(Max(M_Sanad),0) as S From Moein Where M_Tx=0 and M_Coid=<y> and M_ID in(<list>) and M_Date=<jalali>` | `Dmu.pas:1461-1477` | **voucher-merging allocator** (§4.2.2 step 4) |
| `Select isnull(max(M_TX),0) As TX From Moein Where M_Sanad=<n> And M_COID=<y>` | `Dmu.pas:1166-1178` | the immutability gate (§4.0) |
| the delete-invoice transaction | `AnbarListU.pas:379-388` | §4.2.5 |
| the renumber transaction | `AnbarListU.pas:433-457` | §4.2.6 — five tables |
| the invoice-list query with `Send` / `payF` / `payC` | `AnbarListU.pas:535-547` | §13.5 |

#### 12.5.3 Item master

| SQL | Location | Intent |
|---|---|---|
| `Update Anbar_Jens Set … Where AJ_Code=@Code` / `insert Anbar_jens (…)` | `AnbarCalaAddU.pas:167-176` | §2.3 |
| `Select * From Anbar_FactorD Where AFD_Code=<n>` then `Delete Anbar_Jens Where AJ_Code=<n>` | `AnbarCalaU.pas:177-188` | §2.4 |
| `AnbarCala_SeekName` — `PATINDEX` search | `Dmu.dfm:604-628` | §2.6 |
| `select * from Anbar_Vahed Order By AV_Name` | `AnbarCalaAddU.dfm:267-268` | unit-of-measure lookup |
| `Select Max(AJ_Code) As MaxC, Min(AJ_Code) as MinC From Anbar_Jens` | `Anbar_MandehU.pas:125` | default the code range |

#### 12.5.4 Warehouse settings

| SQL | Location | Intent |
|---|---|---|
| `Set @C = (Select max(AC_ID) From Anbar_Config) + 1` … `insert Anbar_Config (AC_ID, AC_Name)` | `AnbarTanzimU.pas:216-219` | add a warehouse |

#### 12.5.5 Reports

| SQL | Location | Intent |
|---|---|---|
| **`Update Anbar_FactorD Set AFD_Customer = (…)` with no `WHERE`** | `Anbar_Amalkard.pas:168-170, 189-191, 215-217` | **table-wide repair disguised as a report — §13.10** |
| line-detail / daily-subtotal / grand-total selects | `Anbar_Amalkard.pas:174-180, 195-205, 221-231` | §13.10 |
| `#R` aggregate over `FactorMaster ⋈ FactorDetail` | `AnbarReportU.pas:189-234` | §13.11 |
| header + line + settlement for the invoice print | `FactorPrint3U.dfm:3238-3258, 3358-3360, 3388-3413` | §13.14 |
| `#R` join for the official invoice | `Factorprint2U.dfm:1101-1123` | §13.13 |
| `Select * From Sahamdar Where S_card=<jari>` | `Factorprint2U.pas:88` | buyer legal identity for the official invoice |

#### 12.5.6 Accounting integration

| SQL | Location | Intent |
|---|---|---|
| `Delete moein … M_Id in(31..39) and M_Link=<FM_SSN>` | `MakeSanadU.pas:84-86` | idempotency before posting |
| parameterised `Insert Moein (…)` loop | `MakeSanadU.pas:92-107` | write the voucher lines |
| `Update Moein Set M_Ko=…, M_Mo=…, M_Ta1=…, M_Ta2=… from sarfasl` | `MakeSanadU.pas:112-114` | back-fill account levels |
| `Update Anbar.Dbo.FactorMaster Set FM_Lock=2, FM_SanadNo=…, FM_SanadDate=…` | `MakeSanadU.pas:121-122` | stamp the source document |
| `Select * From Anbar.DBo.Anbar Where A_Code=<n>` | `MakeSanadU.pas:213, 348, 483, 617` | read the warehouse's posting accounts |
| `Delete moein … M_id in (32,33,35)` + `Update FactorMaster … FM_Lock=1` | `SodoorSanadU.pas:257-260` | un-post — **asymmetric, §5.3.2** |
| the `FactorMaster ⋈ FactorKind` list with `Mab1`/`Mab2` | `SodoorSanadU.dfm:521-533` | §13.16; **missing `S_LinkPRG` filter, §10.4** |

#### 12.5.7 Pistachio and weighbridge

| SQL | Location | Intent |
|---|---|---|
| `Select *, STN = Case NR_State … From Rppc_Solution.DBO.NewRamz Order By NR_Ghabz` | `FactorPesteh_U.dfm:362-373` | the delivery list — **database name hard-coded** (§8.3.1) |
| the whole receipt transaction | `FactorPesteh_U.pas:181-233` | §8.3.4 |
| `Select * From <Anbar>.cala where C_code=<n> and ( C_Anbar like '%,17,%')` | `FactorPesteh_U.pas:137` | item-in-warehouse check; **`_cala` interpolated without quoting** |
| `Select * From NewRamz` on the **main** connection | `Dmu.dfm:1114-1124` (`KharidPeste_List`) | **dead and would fail — §8.6** |
| the settlement union | `TasfiehFactor.dfm:427-446` | §9.2 |
| `Anbar_Tasfieh` | `Dmu.dfm:1017-1072` | **dead** second settlement implementation — §9.6 |

#### 12.5.8 Feature detection

```sql
Declare @Anbar varchar(20) Set @Anbar='Anbar.Dbo'
if DB_ID('Anbar') is null Set @Anbar=''
```
`Dmu.pas:763-778` — §5.0. Also sets `Basc_DB := 'Rppc_Solution.Dbo'` (`:761,778`).

---

### 12.6 Cross-cutting properties of the SQL

| Property | Detail |
|---|---|
| **Dates are `varchar(10)` Jalali strings compared lexicographically** | Works only because the format is zero-padded `YYYY/MM/DD`. Every date filter in this document relies on it. §5.1.2. |
| **String concatenation is the norm; parameters are the exception** | Numeric values go through `inttostr`, strings through `QuotedStr` — so the code is *mostly* injection-safe by accident rather than by design. The one unquoted interpolation is `_cala` (`FactorPesteh_U.pas:137`), protected only by `NR_Kind` being an integer column. |
| **`#R` temp tables are the standard idiom for multi-step aggregation** | `Anbar_MandehU`, `AnbarReportU`, `Factorprint2U`, `AnbarFactorU.Q1`. Each begins `if Object_ID('tempdb..#R') is not null Drop Table #R`. In PostgreSQL these become CTEs. |
| **Design-time `ConnectionString` literals with `User ID=sa`** | `FactorPesteh_U.dfm:354-359` (`MOHSEN-RANJBAR\SQLEXPRESS`), `AnbarCalaAddU.dfm:260-264` (`Arzi89`), `AnbarCardJensiU.dfm:872-877` (`RPPC` on `PESTEH`), `TasfiehFactor.dfm:395-398`, `Dmu.dfm:1125-1136`. All overwritten at runtime, all shipped in the binary. See `docs/08-platform-and-security.md`. |
| **Almost nothing is transactional** | Exceptions: `AnbarListU.pas:379-388` (delete), `:433-457` (renumber), `FactorPesteh_U.pas:181-231` (pistachio receipt, no rollback), `SodoorSanadU.pas:250-262` (un-post). The invoice save, the voucher post and every report are not. |
| **The shared `Dm.Q1` / `Dm.QS` datasets are reused everywhere** | `Dmu.pas:1022, 1258, 1464`; `AnbarCalaU.pas:175-188`; `AnbarTanzimU.pas:214-220`. Any handler that leaves `Dm.Q1` open or repositioned affects unrelated code. |
| **`Order By` is frequently absent** | `AnbarCala_SeekName` orders; the settlement union does not (§9.2); the Card Jensi ordering is inside the SP (§11.1.3). |

---

### 12.7 Recovery checklist for the migration team

Run against the production database, in this order of importance:

```sql
-- 1. THE critical unknown (§12.1)
SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_AddToFactor'));

-- 2. Stock-card ordering and opening balance (§12.2, §11.1.3)
SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_CardJensi'));

-- 3. Year-opening quantities and average cost (§12.3, §6.4)
SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_Mandeh'));

-- 4. Item-list view (§12.4)
SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_AjnasView'));

-- 5. Everything else, for completeness
SELECT o.name, OBJECT_DEFINITION(o.object_id)
  FROM sys.objects o
 WHERE o.type IN ('P','FN','IF','TF','V')
   AND o.name LIKE 'Anbar%';

-- 6. The DDL this document had to infer (§1, §3)
SELECT t.name AS table_name, c.name AS column_name, ty.name AS type_name,
       c.max_length, c.precision, c.scale, c.is_nullable,
       OBJECT_DEFINITION(c.default_object_id) AS column_default
  FROM sys.columns c
  JOIN sys.tables  t ON t.object_id = c.object_id
  JOIN sys.types  ty ON ty.user_type_id = c.user_type_id
 WHERE t.name IN ('Anbar_Jens','Anbar_Config','Anbar_Vahed','Anbar_Factor','Anbar_FactorD','Kinds')
 ORDER BY t.name, c.column_id;

-- 7. Constraints and indexes — this document assumes there are none
SELECT * FROM sys.key_constraints    WHERE OBJECT_NAME(parent_object_id) LIKE 'Anbar%';
SELECT * FROM sys.foreign_keys       WHERE OBJECT_NAME(parent_object_id) LIKE 'Anbar%';
SELECT * FROM sys.indexes            WHERE OBJECT_NAME(object_id)        LIKE 'Anbar%';
SELECT * FROM sys.triggers           WHERE OBJECT_NAME(parent_id)        LIKE 'Anbar%';
```

Item 7 matters more than it looks: the `@@Identity` read at `FactorPesteh_U.pas:203` is only
correct if `FactorMaster` has **no** insert trigger (§8.3.4), and the whole of §5.4's concurrency
analysis assumes there is no unique constraint on `(AF_COID, AF_Factor)`.

Then, for the external databases:

```sql
-- subsystem B (§3.2, §5.1.5) — owned by another application
USE Anbar;
SELECT * FROM FactorKind ORDER BY FK_ID;        -- the authoritative document-type list
SELECT name FROM sys.columns WHERE object_id = OBJECT_ID('Anbar');   -- A_Aval, A_Kharid, …

-- the weighbridge (§8.3.1) — owned by a third application
USE Rppc_Solution;
SELECT OBJECT_DEFINITION(OBJECT_ID('B_SelectSerial'));
SELECT OBJECT_DEFINITION(OBJECT_ID('SP_SetRamz'));
SELECT OBJECT_DEFINITION(OBJECT_ID('Sp_NRSelectGhabz'));
-- and the meaning of NR_P3..NR_P12, NR_Vazn3..NR_Vazn5
```


---

[← 12. SQL and stored procedures (part a)](05-12-a-sql-and-stored-procedures.md) | [index](00-index.md) | [13. Screen specifications (part a) →](05-13-a-screen-specifications.md)
