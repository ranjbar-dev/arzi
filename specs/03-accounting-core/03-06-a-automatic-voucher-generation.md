_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 6. Automatic voucher generation

### 6.1 Overview

Two subsystems generate accounting vouchers automatically. Only the **inventory** generator is in
scope for this document.

| Generator | Source documents | `M_Id` values | Unit |
|---|---|---|---|
| Inventory (Anbar) | `Anbar.dbo.FactorMaster` | 31, 32, 33, 35 | `MakeSanadU.pas`, driven by `SodoorSanadU.pas` |
| Treasury (cheques, deposit slips, petty cash) | `DCheck`, `DCheck2`, `DFish`, `CheckMaster`, `TankhahMaster` | 21..29 | treasury module — out of scope |

`Dm.MakeSanad_FishVariz` (`Dmu.pas:341-360`) is a **stub**: it looks up the deposit slip, errors with
`'فیش واریزی پیدا نشد'` ("deposit slip not found") if absent, and then contains only three comments
where the deletion and creation logic should be. Not implemented.

### 6.2 The driver screen — `SodoorSanadU`

Opened from the main form's `B_Anbar` speed button (`Mainu.pas:972-975`), caption
`'عملیات انبار'` ("inventory operations").

**Precondition** (`SodoorSanadU.pas:325-329`):

```pascal
    if Length( DM.Anbar_DB)=0 then
      MessageDlg('  سیستم انبار نصب نشده است امکان اجرا بدون سیستم انبار نمی باشد', mtError, [mbok], 0);
```
("The inventory system is not installed; running without the inventory system is not possible.")

`Dm.Anbar_DB` is resolved at start-up by probing for the database (`Dmu.pas:763-780`):

```sql
Declare @Saham varchar(20) Set @Saham='Saham.Dbo'
Declare @Anbar varchar(20) Set @Anbar='Anbar.Dbo'
Declare @Basc  varchar(20) Set @Basc='RPPC_Solution.Dbo'
 if DB_ID('Saham') is null Set @Saham=''
 if DB_ID('Anbar') is null Set @Anbar=''
 if DB_ID('Rppc_Solution') is null Set @Basc=''
   Select @Saham As Saham, @Basc As Basc, @Anbar As Anbar
```

**The inventory data lives in a separate SQL Server database.** In the rebuild this becomes a
separate schema (or module boundary) inside one PostgreSQL database.

The list query (`SodoorSanadU.pas:335-358`):

```sql
Declare @Coid int Set @Coid=<CO_ID>
 Select isnull( (Select Sum(S_Mab) from dfish  where s_Coid=@Coid and S_linkSSN=FM.FM_factor ),0) as Mab1
  ,isnull( (Select Sum(S_Mab) from dcheck where s_Coid=@Coid and S_linkSSN=FM.FM_factor ),0) as Mab2
  , FM.* , FK.* From Anbar.DBO.FactorMaster as FM
  Left Join Anbar.DBO.FactorKind AS FK on( FK.FK_ID=FM.FM_ID)
  Where FM.FM_Lock=<0|1|2>        -- or "Where 1=1" for "all"
  and FM.FM_ID in (…)             -- optional document-kind filter
      and FM.FM_Coid=<CO_ID>
 Order By FM_Anbar | FM_Date | FM_ID
```

`Mab1` / `Mab2` are the amounts already settled by deposit slips and cheques against this invoice —
the `S_LinkSSN` → `FM_Factor` link.

Filters (radio-button groups, `SodoorSanadU.pas:345-356`):

| Group | Options | Meaning | Predicate |
|---|---|---|---|
| Approval state | `TaeidAll`, `Taeid0`, `Taeid1`, `Taeid2` | all / not approved / approved / posted | `FM_Lock = 0|1|2` |
| Document kind | `FKAll`, `FK261611`, `FK1214`, `FK1322`, `FK1525` | all / 11,16,26 / 12,14 / 13,22 / 15,25 | `FM_ID in (…)` |
| Sort | `Order1`, `Order2`, `Order3` | warehouse / date / kind | `Order By` |

**`FM_Lock` is the inventory document's own state machine:**

| `FM_Lock` | Meaning | Persian |
|---|---|---|
| 0 | Not approved | فاکتور تایید نشده |
| 1 | Approved, not yet posted | تایید شده |
| 2 | Posted (a voucher exists) | سند صادر شده |

Approval itself is **not** done here. `B_TaeidClick` (`SodoorSanadU.pas:108-139`) shows
`'  از برنامه انبار جهت تایید فاکتور استفاده کنید  '` ("Use the inventory program to approve the
invoice") and returns — the rest of the routine is unreachable. If already approved it first shows
`'  قبلا تایید شده است  '` ("it has already been approved").

### 6.3 Posting an inventory document — `B_SodoorClick`

(`SodoorSanadU.pas:167-208`) Guards, in order:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `DM.Is_New_Sanad_Valid(CO_ID)` | (see §3.8) | fiscal year must be open | `SodoorSanadU.pas:172` |
| 2 | `FM_Lock = 0` | `' فاکتور تایید نشده است'` | "The invoice has not been approved" | `SodoorSanadU.pas:178` |
| 3 | `FM_Lock > 1` | `' سند قبلا صادر شده است'` | "The voucher has already been issued" | `SodoorSanadU.pas:183` |

Then dispatch on `FM_ID` (`SodoorSanadU.pas:190-204`):

| `FM_ID` | Persian | English | Handler | Assigned `M_Id` |
|---|---|---|---|---|
| 11 | موجودی اول دوره | Opening stock | `MakeSanadF.init11` | 31 |
| 12 | خرید / ورود به انبار | Purchase / goods receipt | `MakeSanadF.init12` | 32 |
| 22 | فروش | Sale | `MakeSanadF.init22` | 33 |
| 13 | برگشت از فروش | Sales return | `MakeSanadF.init13` | 35 |
| anything else | — | — | `MessageDlg(' Not implemented yet. ')` | — |

**`FM_ID` values 14, 15, 16, 25, 26 appear in the list filters but have no generator.** They are
production receipts/issues (15, 25) and inter-warehouse transfers (16, 26), which can be viewed and
printed (`SodoorSanadU.pas:272-285`) but never post to the ledger. Record as a functional gap (§14).

### 6.4 The account configuration source: `Anbar.dbo.Anbar`

Every posting rule reads its accounts from the **warehouse master record**:

```sql
-- MakeSanadU.pas:213 (also :348, :483, :617)
 Select * From Anbar.DBo.Anbar Where A_Code=<FM_Anbar>
```

Columns used as account references (all hold `Sarfasl.S_SSN`):

| Column | Persian role | English | Used by |
|---|---|---|---|
| `A_Aval` | کد اول دوره | Opening-stock account | `init11` |
| `A_Kharid` | کد خرید مواد و کالا | Purchases account | `init12` |
| `A_Foroosh` | کد فروش | Sales revenue account | `init22` |
| `A_BForoosh` | کد برگشت از فروش | Sales-returns account | `init13` |
| `A_Kasr` | کد تخفیفات | Discounts account | all four |
| `A_Maliat` | کد مالیات | Tax account | all four |

And from the document header `FactorMaster`:

| Column | Meaning |
|---|---|
| `FM_SSN` | Document primary key — becomes `Moein.M_Link` |
| `FM_COID` | Fiscal year |
| `FM_Anbar` | Warehouse code |
| `FM_Factor` | Human-readable invoice number |
| `FM_Date` | Jalali document date — becomes the voucher date |
| `FM_ID` | Document kind |
| `FM_TSSN` | **Counterparty account** (`Sarfasl.S_SSN`) — the customer or supplier |
| `FM_Desc` | Document narration |
| `FM_Mab` | Net goods amount (مبلغ) |
| `FM_Kasr` | Discount amount (کسر / تخفیف) |
| `FM_Maliat` | Tax amount (مالیات) |
| `FM_Total` | Gross total charged to the counterparty |
| `FM_Lock` | State (see above) |
| `FM_SanadNo`, `FM_SanadDate` | Back-link to the generated voucher |

---

_Prev: [03-05-b-voucher-line-editing-behaviour](03-05-b-voucher-line-editing-behaviour.md) | Next: [03-06-b-automatic-voucher-generation](03-06-b-automatic-voucher-generation.md)_
