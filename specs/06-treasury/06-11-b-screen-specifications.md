_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

### 11.3 `CheckDaryaft2U` — Hand a cheque to the bank

**Title**: `واگذاری چک به بانک` "handing the cheque over to the bank".
**Entry**: `New(_SSN)` only.

**Top panel — read-only view of the cheque** (loaded by `Select * From DCheck Where S_SSN=…`):
`شماره سند` voucher no, `تاریخ سند` voucher date, `تاریخ سررسید` due date, `شماره چک` cheque no,
`مبلغ چک` amount, `توضیحات` description (a `TsMemo`), `: دهنده چک` cheque giver (`S_P`/`S_PN`),
`: وضعیت فعلی` "current status" (`S_Bes`/`S_BesN`, showing the cheque's current notes-receivable
account). The two `...` buttons on this panel are decorative.

**Bottom panel — the action**:

| Field | Label | Control | Behaviour |
|---|---|---|---|
| `S_Date2` | `تاریخ واگذاری` "hand-over date" | `TFullDate` | required, in fiscal range. Defaults to today |
| `S_Sanad2` | `شماره سند` "voucher number" | `TEditInt` | auto-allocated when 0 |
| `S_Bed` | `چکهای درجریان وصول` "cheques in course of collection" | code + `...` `B_Bed` + names | pre-filled `Jaryan_K-Jaryan_M-<Tafsil-1>`; required, must be a leaf |
| `S_Desc2` | `شرح واگذاری` "hand-over description" | `TsMemo` | optional |

**Buttons**: `واگذار به بانک` "Hand over to the bank" (`sBitBtn5`) → the transaction of §10.3, then
`سند انتقال ثبت شد` "the transfer voucher was recorded" and close; `برگشت` "Back".

**Rebuild note**: the target bank is *not* captured — only the internal collection account. If the
business needs "which bank is holding this cheque", it must be added.

---

### 11.4 `CheckBargashtu` — Bounce a cheque back from the bank

**Title**: `برگشت چک از بانک` "return of the cheque from the bank".
Same two-panel shape as 11.3. The read-only top panel adds nothing new.

**Bottom panel**: `تاریخ برگشت` "bounce date" (`S_Date2`, required, in range), `شماره سند`
(`S_Sanad2`, auto), `چکهای موجود در صندوق` "cheques held in the cash box" (`S_Bed`, pre-resolved
`Sandoogh_K-Sandoogh_M-<Tafsil-1>`, required leaf), `توضیحات` (`S_Desc2`).
**Buttons**: `برگشت چک به صندوق` "Return the cheque to the cash box" (`S_Save`), `برگشت` "Back".

**Rebuild notes**: no field captures the bank's reason code, the returned-cheque certificate number
(گواهی عدم پرداخت) or any bank charge. Also, the credit account (`S_Bes`, the collection account) is
resolved silently and never shown to the user.

---

### 11.5 `CheckVosoolU` — Declare a cheque collected

**Title**: `اعلام وصول چک` "declaration of cheque collection".
Read-only top panel as above.

**Bottom panel**:

| Field | Label | Control | Behaviour |
|---|---|---|---|
| `S_Date2` | `تاریخ وصول` "collection date" | `TFullDate` | required, in range |
| `S_Sanad2` | `شماره سند` | `TEditInt` | auto; additionally checked by `Get_SanadDateID_Valid` |
| `S_Bank` | `: کد حساب بانک` "bank account code" | code + `...` `B_Bank` + names | **the account the money lands in**. Remembered in the INI via the right-click `ذخیره به عنوان پیش فرض`. **Never validated for `> 0`** |
| `S_Desc2` | `توضیحات` | `TsMemo` | optional |

**Buttons**: `ذخیره` "Save", `برگشت` "Back". On success: `سند وصول ثبت شد` "the collection voucher
was recorded".

**Rebuild notes**: add the missing `S_Bank` validation and the missing voucher-header build
(§8.5 defect 1).

---

### 11.6 `CheckEsterdadU` — Return a cheque to its issuer

**Title**: `استرداد چک` "restitution of the cheque".
Read-only top panel as above; **both** accounts are taken from the cheque, so there is no picker.

**Bottom panel**: `تاریخ استرداد` "restitution date" (`S_Date2`), `شماره سند` (`S_Sanad2`, auto),
`توضیحات` (`S_Desc2`).
**Buttons**: `استرداد چک` "Return the cheque" (`sBitBtn5`), `برگشت` "Back". On success:
`  سند استرداد ثبت شد   `.

---

### 11.7 `FishListD` — Deposit-slip list

**Title**: `لیست واریزیها` "list of deposits". **Entry**: `Mainu.pas:621` → `FishListDF.init`.

**Grid `G1`** — `Select * From DFish Where 1=1 Order By S_Dates`:

| # | Field | Persian header | English header |
|---|---|---|---|
| 1 | `S_StateName` | `نوع واریز` | Deposit channel |
| 2 | `S_FishNo` | `شماره فیش` | Slip number |
| 3 | `S_Sanad` | `سند` | Voucher |
| 4 | `S_Date` | `تاریخ سند` | Voucher date |
| 5 | `S_Mab` | `مبلغ` | Amount (formatted by `Q1S_MabGetText`) |
| 6 | `S_BesCR` | `واریز کننده` | Depositor (account code) |
| 7 | `S_BesName` | `نام صاحب حساب` | Account holder name |
| 8 | `S_Desc` | `شرح` | Description |
| 9 | `S_LinkPrg` | `اطلاعات اضافی` | Extra information — `' فاکتور کالا '`/`' فاکتور پسته '` + id (`FishListD.pas:188-192`) |

**Buttons**:

| Caption | English | Permission | Guard | Action |
|---|---|---|---|---|
| `ثبت واریزی فیش` | Record a deposit slip | 2102 | year open | `FishDaryaftF.new` |
| `اصلاح واریزی` | Amend deposit | 2103 | same year, voucher draft | `FishDaryaftF.Edit(S_SSN)` |
| `حذف واریزی` | Delete deposit | 2104 | same year, voucher draft, `S_LinkPrg = 0`, confirmation | raw delete (§10.10) |
| `برگشت` | Back | — | — | `Close` |

Same dead state-filter toolbar as the cheque list (`State1Click` short-circuits at
`FishListD.pas:242`).

---

### 11.8 `FISHDaryaftU` — Deposit-slip editor

**Title**: `دریافت وجه نقد - کارتخوان - واریز فیش بانکی` "cash receipt — card reader — bank deposit
slip".

| Field | Label | Control | Behaviour |
|---|---|---|---|
| `S_FishNo` | `شماره فیش /پیگیری  : ` "slip / tracking number" | `TsEdit` | **no validation** |
| `S_Date` | `تاریخ سند/واریز  : ` "voucher / deposit date" | `TFullDate` | required, in range |
| `S_Mab` | `مبلغ واریزی  : ` "deposit amount" | `TEditInt` | required `> 0` |
| `S_Sanad` | `شماره سند  : ` "voucher number" | `TEditInt` | read-only, auto |
| `S_Desc` | `شرح واریزی  : ` "deposit description" | `TsEdit` | **optional** |
| `S_State` | `نوع واریزی  : ` "deposit type" | `TsComboBox`, 4 items (§6.2) | defaults to item 0 |
| `S_Bed` | `کد  بانک  : ` "bank code" | code + `...` `B_Bed` + names | required `Tag > 0`; maps to `S_BankSSN`. Right-click → save as default |
| `S_Bes` | `واریز کننده  :` "depositor" | code + `...` `B_Bes` + names | required `Tag > 0`; read-only when `S_LinkPrg > 0`. Right-click → save as default |

**Buttons**: `ذخیره` "Save" (`sBitBtn5`), `برگشت` "Back". The bottom `sPanel2` holds three `TDBEdit`
controls with no visible `DataSource` binding — residue.

---


---

[← 11. Screen specifications (part a)](06-11-a-screen-specifications.md) | [index](00-index.md) | [11. Screen specifications (part c) →](06-11-c-screen-specifications.md)
