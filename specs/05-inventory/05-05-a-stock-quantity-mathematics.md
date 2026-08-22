_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 5. Stock quantity mathematics

### 5.0 There are two separate inventory subsystems

This is the single most important structural fact about the inventory domain, and nothing else in
this document makes sense without it.

| # | Subsystem | Physical location | Tables | Owned by |
|---|---|---|---|---|
| **A** | **Local warehouse invoices** | The main `arzi` database (same catalog as `Moein`, `Sarfasl`) | `Anbar_Jens`, `Anbar_Config`, `Anbar_Factor`, `Anbar_FactorD` | This application. Full CRUD. |
| **B** | **External warehouse system** | A *separate SQL Server database* `Anbar`, referenced as `Anbar.Dbo.<table>` | `Anbar`, `Cala`, `FactorMaster` (`FM_*`), `FactorDetail` (`FD_*`), `FactorKind` (`FK_*`) | A **different application** not present in this repo. `arzi` reads it, posts vouchers from it, and inserts into it in exactly one place. |

Subsystem B is **feature-detected at runtime**. `Dmu.pas:763-778` runs:

```sql
Declare @Anbar varchar(20) Set @Anbar='Anbar.Dbo'
if DB_ID('Anbar') is null Set @Anbar=''
```

and stores the result in `DM.Anbar_DB` (`Dmu.pas:777`). Every subsystem-B screen guards on it:

> `Anbar_MandehU`-style guard, e.g. `SodoorSanadU.pas:325-329`, `FactorPesteh_U.pas:276-280`
> — `'  سیستم انبار نصب نشده است امکان اجرا بدون سیستم انبار نمی باشد'`
> ("the warehouse system is not installed; running without the warehouse system is not possible")

A third external database `Rppc_Solution` (`DM.Basc_DB`, `Dmu.pas:761,778`) holds the
**weighbridge** table `NewRamz` — see §8.

**Rebuild consequence:** in PostgreSQL these become three schemas (`inventory`, `warehouse_ext`,
`weighbridge`) of one database, or — preferably — subsystem A and B are **merged**, because they
are two implementations of the same idea that disagree with each other (see §15).

---

### 5.1 On-hand quantity is never stored. It is always a full re-scan of movement lines.

There is **no balance column** anywhere. Not on `Anbar_Jens`, not on `Cala`, not on any
period-snapshot table. Every quantity displayed anywhere in the system is computed on demand by
summing `Anbar_FactorD.AFD_Num` over the *entire* fiscal year, filtered by movement direction.

Three independent implementations of the same sum exist, and they do not agree with each other on
which lines to include. All three are quoted verbatim below.

#### 5.1.1 Direction is derived from the document type, not from a sign

`Anbar_FactorD.AFD_Num` is **always positive**. Direction comes from `AFD_Type`, which is copied
onto every detail line from the header's `AF_Type` when the line is written
(`AnbarFactorU.pas:629` passes `@Type := AF_Type.Tag` into `SP_AnbarAddToFactor`).

| `AF_Type` / `AFD_Type` | Persian label | English | Stock effect |
|---|---|---|---|
| `1` | `رسید انبار` | Goods receipt / purchase | **+ inbound** |
| `2` | `حواله انبار` (also labelled `فاکتور فروش کالاو خدمات`) | Goods issue / sales invoice | **− outbound** |
| `3` | `برگشت از خرید` | Return to supplier (purchase return) | **− outbound** |
| `4` | `برگشت از فروش` | Return from customer (sales return) | **+ inbound** |

Label sources: `AnbarListU.pas:181-184` (the grid display function) and `AnbarListU.pas:540-541`
(the same mapping duplicated as a SQL `CASE`). A **third**, divergent copy exists at
`AnbarFactorU.pas:116-120`:

```pascal
TypeC[1] := 'رسيد انبار' ;
TypeC[2] := 'حواله انبار' ;
TypeC[2] := 'فاکتور فروش کالاو خدمات' ;   // ← line 118 overwrites line 117
TypeC[3] := 'برگشت رسيد انبار' ;
TypeC[4] := 'برگشت حواله انبار' ;
```

Line 118 silently overwrites line 117, so the *invoice screen* titles type 2 "Goods and services
sales invoice" while the *list screen* calls the same type "Warehouse issue note". Types 3 and 4
are named "Reversal of receipt note" / "Reversal of issue note" on the invoice screen but
"Return from purchase" / "Return from sale" on the list. Same numbers, three vocabularies.
**Pick one in the rebuild** (§16 proposes `receipt` / `issue` / `purchase_return` / `sales_return`).

#### 5.1.2 Implementation 1 — the stock-balance report (`Anbar_MandehU`)

`Anbar_MandehU.dfm:1613-1727`, the `Q1` query, executed by `B_Enteghal1Click`
(`Anbar_MandehU.pas:195-201`). It builds a `#R` temp table over `Anbar_Jens` and fills it with
five `Update` passes. Verbatim (line breaks normalised out of the `.dfm` string concatenation):

```sql
Declare @Coid int Set @Coid =:COID
Declare @FCode int Set @FCode=:FCode
Declare @TCode int Set @TCode = :TCode
Declare @FDate varchar(10) Set @FDate=:FDate
Declare @TDate varchar(10) Set @TDate =:TDate

IF OBJECT_ID('tempdb..#R') IS NOT NULL DROP TABLE #R

Select AJ_Code, AJ_Name, AJ_Prop, AJ_Vahed
     , cast(0 as Numeric(14,3)) As R1
     , cast(0 as Numeric(14,3)) As Tedin1 , cast(0 as bigint) as Mabin1 , cast(0 as bigint) as Phiin1
     , cast(0 as Numeric(14,3)) As TedIn2 , cast(0 as bigint) as Mabin2 , cast(0 as bigint) as Phiin2
     , cast(0 as Numeric(14,3)) As TedOut1, cast(0 as bigint) as MabOut1, cast(0 as bigint) as PhiOut1
     , cast(0 as Numeric(14,3)) As TedOut2, cast(0 as bigint) as MabOut2, cast(0 as bigint) as PhiOut2
     , cast(0 as Numeric(14,3)) As R2
into #R
From Anbar_Jens
Where AJ_Code>=@FCode and AJ_Code<=@TCode

--Aval Doreh                                              -- opening balance
Update #R Set R1 = isnull( (Select Sum(AFD_Num) From Anbar_FactorD
                            Where Anbar_FactorD.AFD_Code=#R.AJ_Code
                              and Anbar_FactorD.AFD_Coid = @Coid
                              and AFD_Date<@FDate and Anbar_FactorD.Afd_Type in(1,4) ) , 0)
                   - isnull( (Select Sum(AFD_Num) From Anbar_FactorD
                            Where Anbar_FactorD.AFD_Code=#R.AJ_Code
                              and Anbar_FactorD.AFD_Coid = @Coid
                              and AFD_Date<@FDate and Anbar_FactorD.Afd_Type in(2,3) ) , 0)

-- kharid                                                 -- purchases in period
Update #R Set Tedin1 = isnull((Select Sum(AFD_Num) From Anbar_FactorD
        Where AFD_Code=#R.AJ_Code and AFD_Coid=@Coid
          and AFD_Date>=@FDate and AFD_Date<=@TDate and Afd_Type=1),0)
Update #R Set Mabin1 = isnull((Select Sum(AFD_Kol) From Anbar_FactorD
        Where AFD_Code=#R.AJ_Code and AFD_Coid=@Coid
          and AFD_Date>=@FDate and AFD_Date<=@TDate and Afd_Type=1),0)
Update #R Set Phiin1 = Cast( Mabin1 / Tedin1 as int ) Where Tedin1 > 0

-- B Foroosh (Afd_Type=4) → Tedin2 / Mabin2 / Phiin2      -- identical shape
-- Foroosh   (Afd_Type=2) → TedOut1 / MabOut1 / PhiOut1   -- identical shape
-- B kharid  (Afd_Type=3) → TedOut2 / MabOut2 / PhiOut2   -- identical shape

Update #R Set R2 = R1 + Tedin1 + Tedin2 - TedOut1 - TedOut2

Delete #R Where R1=0 and R2=0 And Tedin1=0 and Tedin2=0 and Tedout1=0 And Tedout2=0

Select * from #R
```

**The canonical formula, therefore:**

```
opening(item, d)        = Σ AFD_Num[type ∈ {1,4}, AFD_Date <  d]
                        − Σ AFD_Num[type ∈ {2,3}, AFD_Date <  d]

closing(item, d1..d2)   = opening(item, d1)
                        + Σ AFD_Num[type = 1, d1 ≤ AFD_Date ≤ d2]   -- purchases
                        + Σ AFD_Num[type = 4, d1 ≤ AFD_Date ≤ d2]   -- sales returns
                        − Σ AFD_Num[type = 2, d1 ≤ AFD_Date ≤ d2]   -- sales
                        − Σ AFD_Num[type = 3, d1 ≤ AFD_Date ≤ d2]   -- purchase returns
```

Notes that matter for the rebuild:

- **Date comparison is a string comparison.** `AFD_Date` is `varchar(10)` holding a Jalali date
  `'1403/02/01'`; the parameters `@FDate`/`@TDate` are declared `varchar(10)`
  (`Anbar_MandehU.dfm:1602-1611`). It works only because the zero-padded `YYYY/MM/DD` Jalali
  format happens to sort lexicographically. Any unpadded date (`1403/2/1`) silently falls in the
  wrong bucket. In PostgreSQL store a real `date` and derive the Jalali string for display.
- **`AFD_Date` is denormalised onto every line** from the header (`AnbarFactorU.pas:631`,
  `@Date := AF_Date.Text`). If a header date is ever changed without rewriting the lines, the
  stock report and the invoice disagree. In practice the save path deletes and re-inserts all
  lines (`AnbarFactorU.pas:620`), so they stay in step — but nothing enforces it.
- **The `Delete #R` line hides items whose net movement is zero**, including items that had
  activity that netted to nil. That is a display filter, not a data rule.
- The report is **not** filtered by warehouse. `Anbar_Jens.AJ_ID` (the warehouse/group id) is
  never used here. Subsystem A has, in effect, one global stock pool per item code.

#### 5.1.3 Implementation 2 — the line-entry lookup (`Anbar_Jens_Phi1`)

`Dmu.dfm:635-720`. This is the query the invoice line editor runs to show current stock and the
average cost. Verbatim:

```sql
Declare @C int    Set @C   = :Code
Declare @CID int  Set @CID = :CoID
Declare @F int    Set @F   = :Factor

Declare @Noin   Real Set @Noin   = (Select sum(AFD_num) from Anbar_FactorD
                                    where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F )
Declare @NoOut  Real Set @NoOut  = (... AFD_Type=2 ...)
Declare @NoBin  Real Set @NoBin  = (... AFD_Type=3 ...)
Declare @NoBOut Real Set @NoBOut = (... AFD_Type=4 ...)

Declare @Mabin   bigint Set @Mabin   = (Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD
                                        Where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @MabOut  bigint Set @MabOut  = (... AFD_Type=2 ...)
Declare @MabBin  bigint Set @MabBin  = (... AFD_Type=3 ...)
Declare @MabBOut bigint Set @MabBOut = (... AFD_Type=4 ...)

Declare @Phiin int
Set @Phiin = 0
if @Noin >0  Set @phiin = Cast( @Mabin / @Noin as int )

Select  Anbar_Jens.*
       ,@Noin as Noin , @Mabin as Mabin , @phiin as phiin , @NOOut As NoOut, @MabOut As MabOut
       ,@NoBIn As NoBin ,@MabbIn As MabBIn , @NoBOut As NoBOut ,@MabBOut as MabBOut
       , (@Noin - @NoOut - @NoBin + @NoBOut) As Remi
From Anbar_Jens
where AJ_code=@C
```

So here:

```
Remi = Σ[type1] − Σ[type2] − Σ[type3] + Σ[type4]      -- all dates, whole fiscal year
```

**Differences from Implementation 1 that are real behavioural divergences:**

| | `Anbar_Mandeh` report | `Anbar_Jens_Phi1` lookup |
|---|---|---|
| Date window | `@FDate .. @TDate` | none — whole `COID` |
| Current invoice | included | **excluded** (`AFD_Factor <> @F`) |
| Amount column | `AFD_Kol` (a stored column) | recomputed `AFD_Num * AFD_Phi` |
| Warehouse filter | none | none |

Excluding the current invoice is *deliberate and correct* for editing: while you re-key invoice
#412, its own lines must not count against available stock. But it is done by comparing
`AFD_Factor` to `AF_Factor.Text`, which is **empty on a brand-new invoice**
(`AnbarFactorAddU.pas:117`, `StrToIntDef(AnbarFactor.AF_Factor.Text,0)` → `0`), and `0` matches
no invoice, so on a new invoice nothing is excluded — correct by accident.

#### 5.1.4 Implementation 3 — a dead third copy

`AnbarFactorU.dfm:587-624` defines a `Q1: TADOQuery` with yet another version:

```sql
Update #R Set Vorood  = ( Select Sum(T1.AFD_Num) From Anbar_factorD T1
                          Where T1.AFD_Code = #R.AFD_Code And T1.AFD_Coid=1400 and T1.AFD_Type in (1,4) )
Update #R Set Khorooj = ( Select Sum(T1.AFD_Num) From Anbar_FactorD T1
                          Where T1.AFD_Code = #R.AFD_Code And T1.AFD_Coid=1400 and T1.AFD_Type in (2,3) )
Update #R Set mandeh  = vorood - khorooj
Update #R Set Kol     = Cast( mandeh * AFD_Phi As bigint )
Delete #R where mandeh <= 0
```

**This is dead code.** `AnbarFactorU.pas` never references `Q1` — it uses `QS` for every ad-hoc
query (`AnbarFactorU.pas:617,651,701,717`). The fiscal year is hard-coded to `1400`, and the unit
price used for valuation is `AFD_Phi` of the item's *most recent* movement line
(`T1.AFD_SSn = (Select Max(T2.AFD_SSN) ... )`) — i.e. latest-transaction price, which is not the
method used anywhere live. Do not port it. It is documented here only so a future reader does not
mistake it for a spec.

#### 5.1.5 Subsystem B has no stock derivation in this repo at all

Nothing in this codebase computes on-hand from `Anbar.Dbo.FactorDetail`. `AnbarReportU.pas:203-222`
aggregates `Sum(FD_Num)`, `Sum(FD_Total)`, `Sum(FD_VaznP)` **grouped by document**, for a
purchase/sale activity report — not a balance. The external warehouse application owns subsystem-B
stock, and its logic is not available. **Flag for the rebuild:** if subsystem B is absorbed, its
stock rules have to be recovered from the other application or re-specified from scratch.

---


---

[← 4. The invoice (Factor) lifecycle (part c)](05-04-c-invoice-factor-lifecycle.md) | [index](00-index.md) | [5. Stock quantity mathematics (part b) →](05-05-b-stock-quantity-mathematics.md)
