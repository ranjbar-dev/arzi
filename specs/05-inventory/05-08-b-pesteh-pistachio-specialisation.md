_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 8.3 The live pipeline (implementation P2) — weighbridge → warehouse receipt → voucher

#### 8.3.1 The source table `NewRamz`

`FactorPesteh_U.dfm:353-373`. The screen's `Q1` is a plain `TADOQuery`:

```sql
Select *
    , STN = Case NR_State
       When 1 then 'رمز'
       When 2 then 'کشف رمز'
       When 3 then 'فاکتور'
       When 4 then 'سند '
       When 5 then 'باطل '
       else ''
       end
From   Rppc_Solution.DBO.NewRamz
Order By NR_Ghabz
```

Two portability defects visible in that one query:

- The database name `Rppc_Solution.DBO` is **hard-coded in the `.dfm`**, while every other
  reference goes through `DM.Basc_DB` (`Dmu.pas:761,778`; `FactorPesteh_U.pas:202,211,218`).
  If the deployment renames the weighbridge database, the list breaks even though the write path
  still works.
- The design-time `ConnectionString` (`FactorPesteh_U.dfm:354-359`) embeds a developer machine and
  `User ID=sa`: `Data Source=MOHSEN-RANJBAR\SQLEXPRESS; Initial Catalog=Anbar; User ID=sa`.
  It is overwritten at runtime by `Q1.ConnectionString := Dm.Ado.ConnectionString`
  (`FactorPesteh_U.pas:282`), so it is inert — but it ships in the binary. Same pattern in
  `Dmu.dfm:1125-1136` (`ADO_RPPCSOLUTION`, `Data Source=Pesteh`, `User ID=sa`, no password).
  Cross-reference `docs/08-platform-and-security.md`.

**`NewRamz` column inventory** (from the persistent field definitions,
`FactorPesteh_U.dfm:376-537`, and the grid titles at `:191-352`):

| Column | Type | Grid title | English | Notes |
|---|---|---|---|---|
| `NR_Serial` | identity | — | id | Primary key; the value passed as `@Serial` |
| `NR_Ramz` | varchar(5) | `رمز` | Blind code | The lab "code" that anonymises the lot |
| `NR_State` | smallint | `وضعیت` (via `STN`) | Status | 1…5, see §8.3.2 |
| `NR_Ghabz` | int | `قبض` | Weighbridge ticket no. | The list is ordered by this |
| `NR_GhabzDate` | varchar(10) | `تاریخ` | Ticket date (Jalali string) | |
| `NR_Date` | varchar(10) | `تاریخ` | Code/lab date (Jalali string) | Written into `FM_SanadDate` (§8.3.4 — a defect) |
| `NR_Jari` | int | `جاری` | Supplier current-account sub-code | Third segment of `301-1-<NR_Jari>` |
| `NR_Name` | varchar(50) | `جاری` (duplicate title) | Supplier name | Grid column title is wrong — two columns titled `جاری` (`:235`, `:242`) |
| `NR_Kind` | int | — | Grade id | See §8.1.1 |
| `NR_KindName` | varchar(30) | `نوع` | Grade name (denormalised) | |
| `NR_Adl` | int | `تعداد` | Bale count | Carried for display only; **not** used in any arithmetic on this path |
| `NR_P1` | varchar(10) | `انس` | Ounce | Descriptive |
| `NR_P1G` | varchar(5) | — | (unused) | Never read in code |
| `NR_P2` | varchar(10) | `دهن بست` | Closed-shell | Descriptive |
| `NR_P2V`, `NR_P2VV` | numeric(18,2) | — | (unused) | Never read in code |
| `NR_P3` … `NR_P12` | varchar(10) | — | Further lab attributes | **Never read anywhere in this repository.** Their meaning is owned by the weighbridge/lab application and is not recoverable from source. |
| `NR_Vazn1` | int | `وزن باسکول` | Gross weighbridge weight | = `BascV` of §8.2 |
| `NR_Vazn2` | numeric(18,3) | `وزن ظرف` | Tare (container) weight | = `Adlk` of §8.2 |
| `NR_Vazn3`, `NR_Vazn4`, `NR_Vazn5` | numeric(18,3) | — | Further deduction buckets | Not displayed, never read. By position they correspond to moisture / blanks / other, but **this is inference, not evidence** — the weighbridge source is unavailable. |
| `NR_Vazn` | numeric(18,3) | `خالص` | **Net weight** | The only weight the receipt uses |
| `NR_Phi` | int | `فی` | Unit price rial/kg | `DisplayFormat = '###,###'` |
| `NR_Kol` | bigint | `مبلغ` | Line/document total | The only amount the voucher uses |
| `NR_Factor` | int | `شماره فاکتور` | Purchase invoice number (weighbridge side) | |
| `NR_FDate` | varchar(10) | `تاریخ فاکتور` | Purchase invoice date (Jalali) | **This** is the voucher date |
| `NR_Resid` | int | `شماره رسید` | Warehouse receipt number | Written back by this screen |
| `NR_Sanad` | int | `شماره سند` | Voucher number | Written back by this screen |
| `User1`, `User2`, `User3` | int | — | Operator ids per stage | Never read here |

**The deduction arithmetic of §8.2 is not re-performed on this path.** `arzi` takes `NR_Vazn`,
`NR_Phi` and `NR_Kol` as given and never checks that `NR_Kol = NR_Vazn × NR_Phi`. If the
weighbridge application writes an inconsistent triple, the receipt and the voucher inherit it.

#### 8.3.2 `NR_State` — the pistachio lot state machine

Two different status vocabularies exist for the same physical lot.

**`NR_State`** (`FactorPesteh_U.dfm:364-371`), the lot's progress through the purchase pipeline:

| Value | Persian | English | Meaning |
|---|---|---|---|
| 1 | `رمز` | Coded | Sample sealed under a blind code (`NR_Ramz`) |
| 2 | `کشف رمز` | Decoded | Blind code broken, lab result attached to the supplier |
| 3 | `فاکتور` | Invoiced | Price agreed; `NR_Phi`, `NR_Kol`, `NR_Factor`, `NR_FDate` set |
| 4 | `سند ` | Vouchered | Warehouse receipt + accounting voucher issued (**set by this screen**) |
| 5 | `باطل ` | Void | Cancelled |

**`StatusBts`** (`Ghabz.pas:70-86`), the weighbridge ticket's own status — a different, longer
scale on the same lot, read via the stored procedure `B_SelectSerial`:

| Value | Persian | English |
|---|---|---|
| 1 | `توزين اول` | First weighing (gross, loaded) |
| 2 | `تشخيص` | Assessment / inspection |
| 3 | `توزين دوم` | Second weighing (tare, empty) |
| 4 | `انبار` | Warehoused |
| 5 | `باطل` | Void |
| 6 | `رمز شده` | Coded |
| 7 | `کشف رمز شده` | Decoded |
| 8 | `قيمت شده` | Priced |

The two scales overlap (`رمز`/`کشف رمز` appear in both, at different numbers) and are maintained
by different applications. **Nothing reconciles them.** In the rebuild there must be exactly one
lot lifecycle. `NR_State` is authoritative for anything `arzi` does.

#### 8.3.3 Preconditions for issuing the receipt

`FactorPesteh_U.pas:108-168`, `B_NewResidClick` — button `صدور سند` ("issue voucher",
`FactorPesteh_U.dfm:152-166`). Guards in order:

| # | Check | `.pas:line` | Persian message | English |
|---|---|---|---|---|
| 1 | Fiscal year open for new vouchers | `:114` | (message inside `DM.Is_New_Sanad_Valid`) | see `docs/03-accounting-core.md` |
| 2 | `Q1` active and non-empty | `:116-117` | — (silent `exit`) | |
| 3 | `NR_State >= 3` | `:120-125` | `ابتدا قبض باسکول رمز و کشف رمز و فاکتور ثبت شود` | "First the weighbridge ticket must be coded, decoded and invoiced" |
| 4 | `NR_State < 4` | `:126-131` | `سند  صادر شده است` | "The voucher has already been issued" |
| 5 | The grade exists as an item in external warehouse 17 | `:134-144` | `کد کالا <n> در سیستم انبار 17 تعریف نشده است` | "Item code \<n\> is not defined in warehouse system 17" |
| 6 | Purchase account `700-3-<NR_Kind>` exists | `:146-154` | `کد خرید <code> در سیستم حسابداری تعریف نشده است` | "Purchase code \<code\> is not defined in the accounting system" |
| 7 | Supplier current account `301-1-<NR_Jari>` exists | `:156-164` | `کد <code> در سیستم حسابداری تعریف نشده است` | "Code \<code\> is not defined in the accounting system" |
| 8 | Operator confirms | `:166-168` | `رسید انبار ثبت شود ؟` | "Register the warehouse receipt?" (`mtWarning`, Yes/No; proceeds only on `mrYes` = 6) |

Check 5 is `Select * From <Anbar>.cala where C_code=<kind> and ( C_Anbar like '%,17,%')`
(`:137`). `Cala.C_Anbar` is a **comma-delimited list of warehouse ids inside a varchar column** —
an item is "in" warehouse 17 when the string `,17,` occurs. This is subsystem B's schema, not
ours, but note the consequence: warehouse `1` and warehouse `17` are distinguishable only because
the delimiters are included, and an item list stored without leading/trailing commas would not
match at all.

Warehouse **17** is hard-coded in five places (`:137, 195, 200, 209`) with no constant and no
setting.

#### 8.3.4 What the receipt writes

`FactorPesteh_U.pas:178-234` builds one batch and executes it with `QS.Open` (it ends in a
`Select`, so `Open` not `ExecSQL`). Wrapped in `Begin Transaction` (`:181`) … `Commit` (`:231`),
with **no `Rollback` and no `Try…Except`** — a failure in the middle leaves the transaction open
on that connection until it is reused or dropped.

**Document number allocation** (`:194-195`):

```sql
Declare @FmFactor int
Set @FMFactor = isnull( ( Select Max(FM_Factor)+1 From <Anbar>.FactorMaster
                          Where FM_Coid=@Coid And FM_Anbar=17) , 1700001)
```

A read-then-write `Max()+1` with no lock and no unique constraint — the same concurrency defect as
§5.4's `New_AnbarFactor`. The seed `1700001` encodes warehouse 17 in the number itself.

**Voucher number allocation** (`:176`):

```pascal
Sanad := Dm.Get_NewSanad_DateID( Q1.FieldByName('NR_FDate').AsString , '31,32,33,34,35,36,37,38,39' );
```

— the same nine-value `M_Id` range as `MakeSanadU` (§5.3.1), allocated **before** the transaction
opens. If the transaction then fails, the number is burned.

**Header** — `Anbar.Dbo.FactorMaster` (`:197-203`), inserted by `Select … From <Basc>.NewRamz
Where NR_Serial=@Serial`:

| Column | Value | Note |
|---|---|---|
| `FM_Coid` | `@Coid` | fiscal year |
| `FM_Anbar` | `17` | literal |
| `FM_Factor` | `@FmFactor` | allocated above |
| `FM_Date` | `NR_FDate` | invoice date |
| `FM_InOut` | `1` | inbound |
| `FM_ID` | `14` | **the document kind** — pistachio purchase receipt |
| `FM_TCode` | `@BesCode` (`'301-1-<NR_Jari>'`) | counterparty account code |
| `FM_UserID` | `@User` | |
| `FM_Count` | `1` | one line always |
| `FM_Tssn` | `@BesSSN` | counterparty `Sarfasl.S_SSN` |
| `FM_TName` | `NR_Name` | denormalised supplier name |
| `FM_Desc` | `@Des` | the narration built at `:171-174` |
| `FM_SanadNo` | `@Sanad` | |
| `FM_SanadDate` | **`NR_Date`** | ⚠ **defect — see below** |
| `FM_Lock` | `2` | "voucher issued", set directly at insert |
| `FM_Mab` | `NR_Kol` | |
| `FM_Kasr` | `0` | no discount is ever possible on this path |
| `FM_Maliat` | `0` | **no VAT is ever applied to a pistachio purchase** |
| `FM_Total` | `NR_Kol` | |

> **Defect — `FM_SanadDate` is set to the wrong date.** `FactorPesteh_U.pas:201` writes `NR_Date`
> (the lab/coding date) into `FM_SanadDate`, while the voucher itself is dated `@Date := NR_FDate`
> (the invoice date, `:189`) and every `Moein` line is written with `@date` (`:224,226`). So the
> document's own record of its voucher date disagrees with the voucher. Nothing reads
> `FM_SanadDate` in this repository, so it is latent — but `SodoorSanadU`'s un-post writes
> `FM_SanadDate=''` (`SodoorSanadU.pas:406`), which means the field is intended as the
> authoritative link timestamp. Contrast `MakeSanadU.pas:121-123`, which correctly writes
> `FM_SanadDate = DM_Date.Text`.

**Line** — `Anbar.Dbo.FactorDetail` (`:206-215`), again `Select … From <Basc>.NewRamz`:

| Column | Value |
|---|---|
| `FD_InOut` | `1` |
| `FD_Anbar` | `17` |
| `FD_FMSSN` | `@FMSSN` (`@@Identity` of the header, `:203`) |
| `FD_Code` | `NR_Kind` |
| `FD_CodeN`, `FD_CodeP`, `FD_CodeV` | `''` then back-filled (`:213-215`) from `Cala.C_Name` / `C_Prop` / `C_Vahed` |
| `FD_Num` | `NR_Vazn` — **net weight in kg is the quantity** |
| `FD_Phi` | `NR_Phi` |
| `FD_Mab` | `NR_Kol` |
| `FD_Kasr`, `FD_Maliat` | `0`, `0` |
| `FD_Total` | `NR_Kol` |
| `FD_VaznP` | `NR_Vazn` — **the same net weight again** |

So on this path quantity and weight are the same number in the same unit; the item's unit of
measure is kilograms by construction. The `@@Identity` read at `:203` is the standard SQL Server
trap: `@@IDENTITY` is scope-agnostic and returns the last identity generated on the connection,
including one generated by a trigger. There is no trigger visible here (no DDL in the repo), but
`SCOPE_IDENTITY()` is the correct call. **Open question §14.**

**Source-lot state update** (`:218-220`):

```sql
Update <Basc>.NewRamz Set NR_State=4, NR_Resid=@FmFactor, NR_Sanad=@Sanad Where NR_Serial=@Serial
```

This is the idempotency guard: `NR_State=4` makes precondition 4 (§8.3.3) reject a second
attempt. It is *state-based*, not a uniqueness constraint — see §8.3.6.

**Post-transaction** (`:237-238`):

```pascal
Dm.DMoein_Make(Sanad, NR_FDate, ' عملیات انبار و خرید پسته مورخ ' + NR_FDate);
Dm.Dmoein_UpdateMab(Sanad);
```

— "warehouse and pistachio purchase operations dated \<date\>". The `DMoein` header is created
**outside** the transaction (`Commit` is at `:231`, these calls at `:237-238`). If the process
dies in between, the voucher lines exist with no header. Cross-reference
`docs/03-accounting-core.md` on `DMoein` drift.

Then the confirmation `سند <n> مورخ <date> ثبت شد` ("voucher \<n\> dated \<date\> registered",
`:240`) and a full requery + `Locate` back to the same row (`:243-245`).

---


---

[← 8. The Pesteh (pistachio) specialisation (part a)](05-08-a-pesteh-pistachio-specialisation.md) | [index](00-index.md) | [8. The Pesteh (pistachio) specialisation (part c) →](05-08-c-pesteh-pistachio-specialisation.md)
