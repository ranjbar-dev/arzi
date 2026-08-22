_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 12.8 `BedBes` — Debtors and creditors report (`TBedBesF`)

Parameterised query (`BedBes.pas:98-107`) with seven parameters: `COID`, `D1`, `D2` (date range),
`M1`, `M2` (amount range), `BedBes` (`ItemIndex + 1` — 1 = debtors, 2 = creditors), `GType`
(grouping). Result columns: `Jari` (party card), `S_Name`, `Rem1` (opening balance), `GBed`, `GBes`
(turnover), `Rem2` (closing balance).

Validations (`BedBes.pas:76-96`):

| Condition | Persian | English |
|---|---|---|
| `D1` invalid | `'تاریخ را وارد کنید'` | "Enter the date" |
| `D2` invalid | `'تاریخ را وارد کنید'` | "Enter the date" |
| `D1 > D2` | `'رنج تاریخ را وارد کنید'` | "Enter the date range" |
| no rows | `'موردی یافت نشد'` | "No item found" |

Defaults: `M1 = 1,000,000`, `M2 = 100,000,000`, `D1` = first of the current month, `D2` = today,
`BedBes.ItemIndex = 1` (`BedBes.pas:154-160`). Double-clicking a row opens that party's ledger card
(`CardJarif.init`, `:130-136`).

### 12.9 `TajmiU` — Consolidated ledger (`TTajmiF`)

**Caption:** `'     دفتر تجمیعی'` ("consolidated ledger").
Instruction panel: `'برای مشاهده دفتر معین تجمیعی کد مورد نظر را انتخاب کنید'`
("To view the consolidated subsidiary ledger, select the desired code").

**Left grid `G1`** (parent selector):

| Field | Persian title | English |
|---|---|---|
| `M_R` | کد حساب | Account code |
| `FullName` | نام حساب | Account name |
| `S_Child` | تعداد | Count (of children) |

**Right grid `sDBGrid1`** (consolidated result):

| Field | Persian title | English |
|---|---|---|
| `M_R` | کد حساب | Account code |
| `FullName` | نام حساب | Account name |
| `BedS` | گردش بدهکار | Debit turnover |
| `BesS` | گردش بستانکار | Credit turnover |
| `BedRS` | مانده بدهکار | Debit balance |
| `BesRS` | مانده بستانکار | Credit balance |

Buttons: `B_Exit` = `'برگشت'` ("back"). `sBitBtn2`, `sBitBtn3` and `B_Calc` all still carry the design
caption `'sBitBtn1'` and have **no handlers** — dead controls. Queries: §10.

### 12.10 `NewFinalu` — Close the books (`TNewFinalF`)

**Caption:** `'بستن حسابها'` ("closing the accounts").

| Control | Persian label | English |
|---|---|---|
| `_Sanad` | شماره سند: | Voucher number: |
| `_Date` | تاریخ سند: | Voucher date: |
| `_Bed` + `B_Bed` (`...`) + `S_BesName` | حساب مقصد: | Destination account: (code / browse / name) |
| `_Desc` | شرح سند: | Voucher narration: |
| `B_Save` | صدور سند | Issue voucher |
| `B_Exit` | برگشت | Back |

**Grid `G1`** (`NewFinalu.dfm:191-221`):

| Field | Persian title | English |
|---|---|---|
| `Tik` | `#` | Selection tick (toggle by double-click) |
| `M_Ko` | کل | General-ledger code |
| `K_Name` | نام کل | General-ledger name |
| `BedR` | مانده بدهکار | Debit balance |
| `BesR` | مانده بستانکار | Credit balance |

Behaviour: §9.2.

### 12.11 `EnteghalU` — Year-end carry-forward (`TEnteghalF`)

**Caption:** `'بستن حسابها'`; header panel `'بستن حسابها و انتقال مانده به دوره بعد '`
("closing the accounts and carrying the balance forward to the next period").

**Two symmetric column groups**, current year on the right (RTL-leading), next year on the left:

| Right column (year N) | Left column (year N+1) | Persian label | English |
|---|---|---|---|
| `sal1` (read-only) | `Sal2` (read-only) | دوره مالی جاری / دوره مالی آینده | Current / next fiscal period |
| `A_Code1` + `B_Code1` + `A_Code1N` + `A_Code1Name` | `A_Code2` + `B_Code2` + `A_Code2N` + `A_Code2Name` | کد اختتامیه / کد افتتاحیه | Closing code / Opening code |
| `Desc1` | `Desc2` | شرح سند | Voucher narration |
| `Date1` | `Date2` | تاریخ سند | Voucher date |
| `Sanad1` | `Sanad2` | شماره سند | Voucher number |

`A_Code*N` shows the last level's name (`Taraf.Get_LastName`); `A_Code*Name` shows the full `/`-joined
path (`Taraf.Get_FullName`) — `EnteghalU.pas:338-348`.

Buttons: `B_Save`, `B_Cancel`. Behaviour: §9.3.

### 12.12 `MergeSanad` — Merge vouchers (`TMergeSanadF`)

**Caption:** `'ادغام اسناد'` ("merge vouchers"). Two symmetric read-only summary panels:

| Field pair | Persian label | English |
|---|---|---|
| `S1` / `S2` (editable) | سند مبدا : / سند مقصد : | Source voucher: / Destination voucher: |
| `Date1` / `Date2` | تاریخ : | Date: |
| `TX1` / `TX2` | وضعیت : | State: |
| `ID1` / `ID2` | نوع سند : | Voucher type: |
| `Bed1` / `Bed2` | بدهکار : | Debit: |
| `Bes1` / `Bes2` | بستانکار : | Credit: |
| `Desc1` / `Desc2` | شرح سند : | Voucher narration: |

Every field except `S1`, `S2` and `Desc2` is populated on `S1Change` / `S2Change` and is display-only.
Buttons: `B_Ok` = `'تایید'` ("confirm"), `B_Exit` = `'خروج'` ("exit"). Behaviour: §7.

### 12.13 `SodoorSanadU` — Inventory posting list (`TSodoorSanad`)

Three radio-button filter groups (§6.2) plus buttons:

| Button | Purpose | Behaviour |
|---|---|---|
| `B_Sodoor` | Issue voucher | §6.3 |
| `B_Delete` | Delete voucher | §6.7 |
| `B_Taeid` | Approve invoice | Always refuses: `'  از برنامه انبار جهت تایید فاکتور استفاده کنید  '` ("Use the inventory program to approve the invoice") |
| `B_ViewSanad` | View voucher | Requires `FM_Lock = 2`, then `'     Not implemented yet.      '` |
| `B_ViewFactor` | Print invoice | Only for `FM_ID ∈ {15,25}` (production) and `{16,26}` (transfer) |
| `B_Pay` | Settle | Only for `FM_ID = 22`; else `'   فقط برای فاکتورهای فروش فعال است   '` ("Enabled only for sales invoices") |
| `B_PrintList` | Print the list | FastReport, title `'لیست فاکتورها'` ("invoice list") |

### 12.14 `MakeSanadU` — Generated-voucher preview (`TMakeSanadF`)

Caption is set per document type (§6.5). Header fields: `DM_Coid` (fiscal year), `DM_Date` (date),
`_Factor` (invoice number), `DM_Sanad` (voucher number), `DM_Desc` (narration, the only editable
field, focused on open). Grid `G1` shows the buffered lines: `M_CR` (code), `M_Name`, `M_Bed`,
`M_Bes`, `M_Ted`, `Article`. Buttons `B_Ok` and `B_Exit`. Behaviour: §6.5–6.6.

### 12.15 `MoeinToRU` — Journal generation (`TMoeinToR`)

Radio pair `_R2` / `_R3` toggles which range control pair is visible, at the same coordinates
(`MoeinToRU.pas:271-286`). Fields: `_N1`/`_N2` or `_D1`/`_D2`, then `_Sanad`, `_Date`, `_Desc`, and a
speed button that fetches the next free voucher number. Buttons `B_Save`, `B_Exit`. Behaviour: §8.1.

### 12.16 `SanadMoeinu` — Legacy voucher screen (`TSanadMoein`)

**Caption:** `'صدور ستد حسابداری'` ("issue accounting voucher" — `ستد` is a typo for `سند`).
Now used only for **journal vouchers** (`Kind = 2`) via `Mainu.SRooz0Click`.

Header: `Edit1` = `'شماره سند'` ("voucher number"), `Edit2` = `'تاريخ سند'` ("voucher date"),
`Edit3` = `'وضعيت سند'` ("voucher state"), `Edit6` = `'توازن'` ("balance"),
`MD_Desc` = `'شرح سند'` ("voucher narration"). Two static labels carry the **hard-coded company name**
`'شرکت تعاوني توليد کنندگان پسته شهرستان رفسنجان'` ("Rafsanjan Pistachio Producers' Cooperative
Company") and `'سيستم حسابداري    -   سند حسابداري'` ("Accounting system — accounting voucher") in the
`.dfm` (`SanadMoeinu.dfm:79`, `:94`); `Label1` is overwritten at runtime from `Base.Co_Name`
(`SanadMoeinu.pas:375`).

Buttons: `San8` = `'آرتيکل جديد '` ("new article"), `San9` = `'اصلاح آرتيکل'` ("edit article"),
`San10` = `'حذف آرتيکل'` ("delete article"), `InFile` = `'ورود از فایل'` ("import from file"),
`Button1` = `'برگشت'` ("back").

**Keyboard** (`SanadMoeinu.pas:271-279`): `Insert` (VK 45) = new article, `Enter` (13) = edit article.
Note `caption := inttostr(key)` at `:278` — leftover debug code that writes the key code into the
window title on every keystroke.

Grid `G1` columns:

| Field | Persian title | English |
|---|---|---|
| `M_L` (or `M_R`) | کد حساب | Account code — toggled by the `R1`/`L1` popup items (`:417-427`) |
| `M_Ta1` | تفضيل | Analytic |
| `M_Mo` | معين | Subsidiary |
| `M_Ko` | کل | General ledger |
| `CodeName` | نام حساب | Account name |
| `M_Bed` | بدهکار | Debit |
| `M_Bes` | بستانکار | Credit |
| `M_Ted` | تعداد / مقدار | Count / quantity |

Popup menu `POP1`: `R1`/`L1` switch the code column between `M_R` and `M_L`; `N3`/`N5` are the
balance-fill helpers (§9.7).

### 12.17 Shared dialogs

| Unit | Function | Prompt shape |
|---|---|---|
| `GetS.pas` | `GetString(title, label, maxLen, var S)` | single text field |
| `GetN.pas` | `GetNo(title, label, default) : integer` | single number field |
| `GetN2N.pas` | `Get2No(title, subtitle, label1, label2, var N1, var N2)` | two number fields (voucher ranges) |
| `GetD.pas` | `GetDate(title, label, default) : string` + `GetD_Ok` | Jalali date field |
| `YesOrNo.pas` | `GetYes(title, question [, q2, q3])` | confirmation |
| `SayMessage.pas` | `saymsg(title, text)` | information |
| `CodeNameU.pas` | `GetCodeName(title, lbl1, lbl2, len1, len2, var Code, var Name, align)` | code + name, OK disabled until both valid |
| `WaitU.pas` | `WaitF.initForm(caption, min, max)` / `Gotonextposition` / `Hide` | progress bar |
| `InFile.pas` | `InFileF.init` | file picker + debit/credit radio + description, for `.GGS` import |

---

_Prev: [03-12-b-screen-by-screen-ui-specification](03-12-b-screen-by-screen-ui-specification.md) | Next: [03-13-permissions](03-13-permissions.md)_
