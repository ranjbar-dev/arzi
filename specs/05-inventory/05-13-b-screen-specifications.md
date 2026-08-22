_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 13.7 Invoice line editor — `AnbarFactorAddU`

Form caption `افزودن به فاکتور` ("add to invoice"). Pricing at §7; stock check at §5.2.1.

| Control | Persian label | Read-only | Source |
|---|---|---|---|
| `Code` | `کد کالا` | no | keyed; `OnExit = CodeExit` |
| `sSpeedButton1` | `?` | — | opens `AnbarCalaSelect` |
| `Name` | — | **yes** | `AJ_Name` |
| `prop` | `درصد`-adjacent, unlabelled | **yes** | `AJ_Prop` |
| `Vahed` | `واحد شمارش` | **yes** | `AJ_Vahed` |
| `Anbar` | — | **yes** | the item's home warehouse name |
| `Phi1` | `متوسط قیمت خرید` | **yes** | average purchase price; **click to copy into `Phi`** (§7.1) |
| `Rem1` | `موجودی انبار` | **yes** | stock on hand, integer-truncated (§5.2.2) |
| `Rem2` | `حداقل مجاز` | **yes** | `AJ_Alarm`, the minimum-stock level — displayed only (§1.2) |
| `Num` | `تعداد / مقدار` | no | `DecimalLength = 2`; `OnChange = PhiChange` |
| `Phi` | `قیمت کالا` | no | `OnChange = PhiChange` |
| `Kasr` | `تخفیف` | no | absolute discount; `OnChange = PhiChange` |
| `KasrD` | `درصد` | no | discount %, `TabOrder = 2`; `OnChange = KasrDChange` — §7.2 defect |
| `Kol` | `مبلغ` | **yes** | derived |
| `Maliat` | `مالیات` | **yes** | derived |
| `Total` | `جمع` | **yes** | derived |
| `B_OK` | `تایید` | — | §7.4 |
| `B_Exit` | `برگشت` | — | `Tag := 0; Close` |

**Tab order gotcha for the rebuild:** `Code`(0) → `Phi`(1) → `KasrD`(2) → … Quantity comes after
the discount percentage, which is what makes the §7.2 defect reachable.

---

### 13.8 Stock balance report — `Anbar_MandehU`

Fully specified at §11.2. Filters `از کد`/`تا کد`/`از تاریخ`/`تا تاریخ`; buttons
`محاسبه` / `چاپ لیست` / `مانده منفی` / `برگشت`; eighteen result columns.

### 13.9 Stock card — `AnbarCardJensiU`

Fully specified at §11.1. Note the four **silent** validation failures and the selectable fiscal
year.

---

### 13.10 Inbound/outbound report — `Anbar_Amalkard`

Form caption `گزارش ورود و خروج انبار` ("warehouse in/out report"). Subsystem A only.

| Control | Persian caption | Options |
|---|---|---|
| `R1: TRadioGroup` | `نوع فاکتورها` "invoice types" | `خرید` purchase / `فروش` sale / `برگشت از خرید` purchase return / `برگشت از فروش` sale return → `AFD_Type := ItemIndex + 1` |
| `L1: TRadioGroup` | `نوع گزارش` "report type" | `گزارش ریز فاکتورها` line detail / `گزارش سر جمع روزانه` daily subtotal / `گزارش سر جمع کل` grand total |
| `D1` / `D2` | `از تاریخ` / `تا تاریخ` | `TFullDate`; defaults to today with day forced to 1 and 31 (`:153-156`) — same hazard as §11.2.1 |
| `C1` / `C2` | `از کد` / `تا کد` | item-code range; **no defaults** (unlike `Anbar_MandehU`, which pre-fills min/max) |

| Button | Caption | English | Handler |
|---|---|---|---|
| **`BCancel`** | `چاپ لیست` | **Print list** — the name is misleading, this is the OK/run button | `BCancelClick` (`:59-147`) |
| `sBitBtn4` | `ایجاد فایل Execl` *(sic)* | Create Excel file | `sBitBtn4Click` (`:241+`) — OLE-automates a live Excel instance |
| `sBitBtn3` | `برگشت` | Back | `Close` |

**Validation** (`:62-99`), six checks, all with messages: `تاریخ را وارد کنید` ("enter the date"),
`رنج تاریخ را وارد کنید` ("enter the date range"), `کد کالا را وارد کنید` ("enter the item code"),
`رنج کد کالا را وارد کنید` ("enter the item code range"). Empty result →
`چیزی یافت نشد` ("nothing found").

**Queries** — `makequery1/2/3` (`:163-234`), three shapes over `Anbar_FactorD`, all filtered by
`AFD_Type`, `AFD_Coid`, `AFD_Date` range and `AFD_Code` range:

| Report type | Grain | Extra join |
|---|---|---|
| 0 line detail | one row per `Anbar_FactorD` row, ordered `AFD_Date, AFD_Factor` | `Left join sarfasl on s_ssn = AFD_Customer` for the counterparty name |
| 1 daily subtotal | `group by AFD_Date, AFD_Code` | none |
| 2 grand total | `group by AFD_Code` | none |

Aggregates in 1 and 2: `Sum(AFD_Num)`, `Sum(AFD_Kol)`, `Sum(AFD_Kasr)`, `Sum(AFD_Maliat)`,
`Sum(AFD_Total)`, with `min(AFD_Name)` / `Min(AFD_Vahed)` for the labels.

> **Critical defect — a report performs an unscoped table-wide UPDATE.**
> All three query builders begin with the **same unconditional statement**
> (`Anbar_Amalkard.pas:168-170`, `:189-191`, `:215-217`):
> ```sql
> Update Anbar_FactorD Set AFD_Customer = (Select AF_Customer From Anbar_Factor
>                                          Where AF_COID=AFD_COID And AF_Factor=AFD_Factor)
> ```
> **There is no `WHERE` clause.** Running this report rewrites `AFD_Customer` on **every row of
> `Anbar_FactorD`, in every fiscal year**, before selecting anything. On a large table this is a
> long blocking write executed by a read-only operator pressing "print". It also silently
> overwrites `AFD_Customer` with `NULL` for any orphaned line whose header no longer exists.
>
> What it tells us: `AFD_Customer` is a **denormalised cache that drifts**, and this report is the
> only thing that repairs it. `Anbar_AddToFactor` receives `@Customer` (§10.1.2) so it presumably
> sets it at insert time — but changing an invoice's counterparty via the header (`:611`) does
> **not** propagate to the lines, which is exactly the drift this statement fixes. A repair job
> disguised as a report. Do not port; make `party_account_id` a header-only column.

**Excel export** (`:241-…`): `CreateOleObject('Excel.Application')`, one worksheet, header row at
row 3, data from row 4, thirteen columns with hard-coded widths and Persian headers
(`ردبف` *(sic — typo for `ردیف`)*, `تاریخ`, `شماره فاکتور`, `طرف حساب`, `کد کالا`, `نام کالا`,
`واحد شمارش`, `تعداد`, `فی`, `مبلغ`, `تخفیف`, `مالیات`, `خالص`), then `=Sum(J4:Jn)` formulas for
the four money columns. Requires Excel installed on the client. **In the rebuild this becomes a
server-side XLSX/CSV download.**

---

### 13.11 External-warehouse activity report — `AnbarReportU`

Subsystem B (§5.0). Guarded on `Length(DM.Anbar_DB) = 0` with
`سیستم انبار نصب نشده است امکان اجرا بدون سیستم انبار نمی باشد`
("the warehouse system is not installed; running without it is not possible", `:139-143`).

| Control | Persian label | Bound to |
|---|---|---|
| `L_Anbar` | `نام انبار` | `Q_Anbar` over `Anbar.Dbo.Anbar`, key `A_Code` |
| `L_Action` | `نوع عملیات` | `Q_Action` = `Select * From Anbar.Dbo.FactorKind where FK_ID in(11,12,13,14,16,22,26)` — **production (15, 25) and type 21 are excluded** |
| `L_COID` | `سال مالی` | `Q_COID` over `Base` — the fiscal year **is** selectable here |
| `D1` / `D2` | from / to date | `TFullDate` |

All five selections are persisted to the INI file on close and restored on open
(`:129-133, 158-177`) — the only screen in the module that remembers its filters.

| Button | Report shape (`_ST`) | `Group By` | Print title |
|---|---|---|---|
| `B_Rep1` | 1 | `FM_Date, FD_Code` | `خلاصه عملیات انبار به تفکیک روز و نوع` "warehouse activity summary by day and type" |
| `B_Rep3` | 2 | `FD_Code` | `خلاصه عملیات انبار به تفکیک نوع` "…by type" |
| `B_Rep2` | 3 | `FM_Date` | `خلاصه عملیات انبار به تفکیک روزانه` "…daily" |
| `S_Print` | — | prints the current `_ST` via `RP1`/`RP2`/`RP3` | |
| `B_Close` / `B_Exit` | — | close | |
| **`B_Lock`** | — | **DEAD — `AnbarReportU.dfm:27`, no `OnClick`, no handler** | |

**Query** (`Open_Q1`, `:183-249`) over `FactorMaster LEFT JOIN FactorDetail on FD_FMSSN = FM_SSN`,
filtered `FM_id = @FMID and FM_Date between @D1 and @D2 and FM_Anbar = @Anbar and FM_COID = @COID`,
selecting `Count(*) As FD_Count, Sum(FD_Num), Sum(FD_Mab), Sum(FD_Kasr), Sum(FD_Maliat),
Sum(FD_Total), Sum(FD_Vaznp)` into `#R`, then
`Delete #R Where FD_Num=0 and FD_Total=0` and `Delete #R Where FD_Total is null`.

**One document type at a time.** `@FMID` is a scalar, so the report cannot show receipts and
issues together. Empty result →
`در بازه زمانی مشخص شده در انبار عملیاتی جهت محاسبه یافت نشد`
("no activity found in the specified period in the warehouse to calculate").

`D1Change` and `DS_AnbarDataChange` call `CloseData` (`:103-117`), hiding the grid whenever a
filter changes — a good pattern the rebuild should keep (stale results are never shown).

---

### 13.12 **Unreachable** — purchase/sale summary, `AnbarReportKharidU`

`Mainu.pas:559-564`:

```pascal
procedure TMain.Anbar_ReportClick(Sender: TObject);
begin
     Anbar_AmalkardF.init;
//      AnbarReportKharid.init;
end;
```

The unit is in `Mainu.pas:281`'s `uses` clause and `AnbarReportKharid.init` is **commented out**.
Nothing else references it. Specified here only so the rebuild team knows what was lost:

| Control | Persian label | Notes |
|---|---|---|
| `K1: TsComboBox` | *(operation type)* | four entries → `@Type := ItemIndex + 1`: `خلاصه عمليات خريد` purchase / `خلاصه عمليات فروش` sale / `خلاصه عمليات برگشت از خريد` purchase return / `خلاصه عمليات برگشت از فروش` sale return |
| `D1` / `D2` | from / to date | defaulted in `init` (`:91-101`) to the first and 31st of the current Jalali month, **with the century hard-coded**: `D1.Text := '13' + _D1.Farsi_Date` — this breaks in Jalali year 1400+ if `Farsi_Date` returns four digits, and it is why the field is `TMyEdit` rather than `TFullDate` |
| `B_OK` / `B_Exit` | | three **silent** validations (`IsDate(D1)`, `IsDate(D2)`, `D1 <= D2`), then `SP1` = `Anbar_ReportKharidForoosh` with `@D1`, `@D2`, `@Type`; empty result → `پيدا نشد` ("not found") |

The stored procedure `Anbar_ReportKharidForoosh` (`Dmu.dfm:497-531`) is therefore **called from
nowhere reachable**. See §12.

---

### 13.13 Official invoice print — `Factorprint2U`

**Purpose:** the legally formatted `صورتحساب فروش کالا و خدمات` ("goods and services sales
invoice") or `پیش فاکتور فروش کالا و خدمات` ("proforma"). Which one is chosen by
`(Sender as TBitBtn).Tag = 1` (`Factorprint2U.pas:98-100`) — **two buttons share one handler**.

**Preconditions** (`B_PrintClick`, `:51-104`):

| # | Check | Persian message | English |
|---|---|---|---|
| 1 | invoice exists | `شماره فاکتور صحیح را برای چاپ وارد کنید` | "Enter a valid invoice number to print" |
| 2 | `AFD_Type = 2` | `فقط بر روی فاکتور فروش فاکتور رسمی صادر میگردد` | "An official invoice is issued only for a sales invoice" |
| 3 | a `Jari` (current-account) number resolves | `jari not found .` — **in English, untranslated** | |
| 4 | a `Sahamdar` row exists for that `Jari` | `برای صدور فاکتور رسمی اطلاعات جاری <n> را در قسمت اشخاص یا شرکتها وارد نمایید` | "To issue an official invoice, enter the details of current account \<n\> under Persons or Companies" |

Check 3 is `Jari := S_ta2; if Jari = 0 then Jari := S_ta1` (`:77-79`) — the counterparty's analytic
account segment, taken from `Sarfasl`. Check 4 then reads
`Select * From Sahamdar Where S_card = <Jari>` (`:88`), i.e. **the legal identity comes from the
person/company register** (`docs/07-parties-and-shareholders.md`), not from the account.

**Data the print needs** (`Q4`, `Factorprint2U.dfm:1101-1123`):

```sql
Select Anbar_Factor.*, Anbar_FactorD.*
     , Sarfasl.S_Ko, Sarfasl.S_mo, Sarfasl.S_ta1, Sarfasl.S_ta2, Sarfasl.S_name
     , Base.*
into #R
 From Anbar_Factor
Left Join Sarfasl on Anbar_Factor.AF_Customer=Sarfasl.S_SSN
Left Join Base on Base.CO_ID=Anbar_Factor.AF_COID
Left Join Anbar_FactorD on AFD_CoID=AF_CoID and AFD_Factor=AF_Factor
Where AF_CoID=:CoiD And AF_Factor=:Factor
```

plus the `Sahamdar` row (buyer legal identity: national id, tax id, registration number, address,
telephone, postcode — the `S_Melli`/`S_Egh`/`S_Sabt`/`S_Post` family, `docs/01-glossary.md` §6b)
and the seller's identity from `Base`.

Runtime-injected text: `SumS` = `' جمع کل قابل پرداخت : ' + Util.No2String(AF_Total) + ' ریال '`
("total payable … rial", **amount in words**), `Title`, `p1` (an image from `DBImage1` — the
company stamp/logo), `Panevis` (footnote) = `AF_Desc` + `Dm.Get_paramstr(1015)`.

> `Util.No2String` renders the amount in Persian words. That utility is required by the rebuild
> and is a non-trivial piece of localisation.
>
> **The `AFD_Type = 2` check reads a *line* column to decide a *header* question** (`:70`). It
> works because `AFD_Type` is copied from `AF_Type` (§3.1.2), and because `Q4` returns one row per
> line so `FieldByName` reads the first. If an invoice ever had no lines the check would read
> `NULL` and reject with the wrong message.

---


---

[← 13. Screen specifications (part a)](05-13-a-screen-specifications.md) | [index](00-index.md) | [13. Screen specifications (part c) →](05-13-c-screen-specifications.md)
