_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 12.4 `SanadEditU` — Voucher editor (`TSanadEditF`)

**Caption:** `'نمایش سند معین'` ("display subsidiary voucher") — the caption does not change with mode.

**Four entry modes**, controlled by the private field `_NewEditView`:

| Mode | Method | `_NewEditView` | Behaviour |
|---|---|---|---|
| New | `New` (`SanadEditU.pas:476-493`) | 1 | Empty buffer; number, date and narration all editable |
| Edit | `Edit(sanad, coid)` (`:454-474`) | 2 | Loads the voucher; number becomes read-only |
| View | `View(sanad, coid)` (`:495-515`) | 3 | Read-only; navigation and print enabled |
| Copy | `Copy(sanad, coid)` (`:678-703`) | 2 → 1 | Loads a voucher, then blanks the number and switches to New |

**Control enablement** — `Set_Key` (`SanadEditU.pas:730-754`) is the single source of truth:

```pascal
    Coid.ReadOnly := True;                        // fiscal year is never editable here
    B_Print1.Enabled := _NewEditView = 3;
    B_Print2.Enabled := _NewEditView = 3;
    B_Add.Enabled    := _NewEditView<>3;
    B_Edit.Enabled   := _NewEditView<>3;
    B_Delete.Enabled := _NewEditView<>3;
    B_Save.Enabled   := _NewEditView<>3;
    S_Save.Enabled   := _NewEditView<>3;
    S_Open.Enabled   := _NewEditView=3;
    S_Sanad.ReadOnly := _NewEditView=2;           // number locked in Edit mode
    S_Date.ReadOnly  := _NewEditView>1;           // date locked in Edit and View
    S_Desc.ReadOnly  := _NewEditView>2;           // narration locked in View only
    S_Next.Enabled     := _NewEditView = 3;
    S_Preior.Enabled   := _NewEditView = 3;
    S_Edit.Enabled     := _NewEditView = 3;
    S_Print.Enabled    := _NewEditView = 3;
    S_FastPrint.Enabled:= _NewEditView = 3;
```

Note: **in Edit mode the date is read-only** — changing a date requires the dedicated action on
`SanadViewU` (§12.3), which is correct given how many tables it must touch.

**Header fields** (top panel `P1`):

| Control | Persian label | English | Notes |
|---|---|---|---|
| `S_Sanad` (`TEditInt`) | شماره سند | Voucher number | |
| `S_Date` (`TFullDate`) | تاریخ سند | Voucher date | Jalali |
| `S_TX` (read-only) | وضعیت سند | Voucher state | text from `DmoeinLoad` (§3.6) |
| `S_Desc` | شرح سند | Voucher narration | |
| `COID` (`TDBLookupComboBox`) | سال مالی | Fiscal year | always read-only |

Four further `TsComboEdit` controls labelled `'ایجاد کننده'` / `'تاریخ ایجاد'` /
`'آخرین تاریخ تغییر'` / `'آخرین تغییر دهنده'` ("creator" / "creation date" / "last change date" /
"last changed by") exist on the form but are **`Visible = False`** (`SanadEditU.dfm:76`, `:86`, `:96`,
`:106`) and never populated. The audit data is in `DMoein` — surface it in the rebuild.

**Toolbar** (`sPanel1`), all icon buttons with tooltips:

| Button | Hint | English | Handler |
|---|---|---|---|
| `S_Preior` | سند قبلی | Previous voucher | `S_Sanad := S_Sanad - 1` then open (`:892-897`) |
| `S_Next` | سند بعدی | Next voucher | `S_Sanad := S_Sanad + 1` then open (`:833-837`) |
| `S_Open` | جستجوی سند | Find voucher | `S_OpenClick` (`:839-890`) |
| `S_Edit` | اصلاح سند | Edit voucher | switch View → Edit (§3.6) |
| `S_Save` | ذخیره | Save | `B_SaveClick` |
| `S_Print` | چاپ | Print | `PrintN.Print` |
| `S_FastPrint` | چاپ سریع | Fast print | `PrintN.FastPrint` |
| `S_Replace` | تغییر شرح | Change description | find/replace (§5.8b) |
| `S_BedBes` | تغییر ستون | Change column | swap debit/credit (§5.8a) |
| `B_Import` | — | Import | `.GGS` import — **user 68 only** (§5.9) |
| `B_Close` | *(mislabelled `چاپ`)* | Close | `Close` |
| `GridFontSize` | — | Grid font size | persisted |

**Side buttons** (`P4`): `جدید` ("new") = `B_Add`, `اصلاح` ("edit") = `B_Edit`,
`حذف` ("delete") = `B_Delete`, `ذخیره` ("save") = `B_Save`, `چاپ سند` ("print voucher") = `B_Print1`,
`چاپ خطی` ("line print") = `B_Print2`, `خروج` ("exit") = `B_Quit`.

Grid columns, footer totals and keyboard shortcuts: §5.2, §5.3.

**Navigating away from a voucher.** `S_SanadChange` (`:950-960`) clears the buffer whenever the
number changes in View mode; `S_SanadExit` (`:962-972`) re-opens the new number on focus loss.

### 12.5 `EditArticleMoeinU` — Voucher line dialog (`TEditArticleMoein`)

**Caption:** `'طرف حساب'` ("counterparty" — a misnomer; it is the line editor).

| Control | Persian label | English |
|---|---|---|
| `EKo` + `SKo` + `BKo` | کل: | General ledger: (code / name / browse) |
| `EMo` + `SMO` + `BMO` | معین: | Subsidiary: |
| `ETa1` + `STa1` + `BTa1` | تفضیل 1: | Analytic 1: |
| `ETa2` + `STa2` + `BTa2` | تفضیل 2: | Analytic 2: |
| `Bed` (`TEditInt`) | بدهکار: | Debit: |
| `Bes` (`TEditInt`) | بستانکار: | Credit: |
| `Ted` (`TEditDecimal`) | تعداد/مقدار: | Count/quantity: |
| `Des` (`TMyEdit`) | شرح: | Description: |
| `B_OK` | تایید | Confirm |
| `B_Exit` | برگشت | Back |

Browse buttons show `...` and are visible only while their level has focus (§5.6).

**Keyboard:** `Enter` and `↓` move to the next control, `↑` to the previous
(`EditArticleMoeinU.pas:105-119`, `WM_NEXTDLGCTL`). `Enter` is swallowed so it never triggers the
default button (`:121-127`). The code fields accept digits and backspace only (`:163-167`).

Validation: §4.2. Cascading resolution and focus guards: §5.6.

### 12.6 `Sarfasl_SelectU` — Postable-account picker (`TSarfasl_Select`)

**Caption:** `'          انتخاب سرفصل'` ("select account"). Distinct from `SelectSarfasl` (§5.6),
which browses **one level**; this one lists **leaf accounts across the whole tree**.

**Grid columns** (`Sarfasl_SelectU.dfm:99-114`):

| Field | Persian title | English |
|---|---|---|
| `M_R` | کد حساب | Account code (RTL form) |
| `S_Name` | نام کد | Code name |
| `FullName` | نام کامل | Full name path |

**Buttons:** `B_Ok` = `'تاييد و انتخاب'` ("confirm and select"), `B_Exit` = `'برگشت'` ("back").
Filter speed buttons on `Panel2`:

| Button | Caption | English | Query |
|---|---|---|---|
| `SP_All` | همه حسابها | All accounts | `Where S_Mo> 0 and S_Child=0 Order by S_Ko, S_Mo, S_Ta1` (`:340-342`) |
| `SP_Jari` | اشخاص | Persons | `Filter_Jari` (`:102-113`) |
| `SP_103` | پرداختنی اشخاص | Persons payable | `Filter_103` (`:80-89`) |

```sql
-- Filter_Jari, Sarfasl_SelectU.pas:107-111
 Select Sarfasl.* From Sarfasl
    Where ( (S_Ko=103 and S_mo=1) or (S_ko=104 and S_Mo in(1,2)) or (S_ko=303 and S_Mo in(1,3) ) )
     and S_Child=0 and S_Ta1>0
    Order by S_Ta1, S_Ko, S_MO

-- Filter_103, Sarfasl_SelectU.pas:85-87
 Select Sarfasl.* From Sarfasl
    Where S_Ko=103 and S_Mo=1 and S_Child=0 and S_Ta1>0
    Order by S_Ta1

-- Filter_109, Sarfasl_SelectU.pas:96-98  (reachable only via init_109)
 Select Sarfasl.* From Sarfasl
    Where S_Ko=109 and S_Mo in (1,2) and S_Child=0 and S_Ta1>0
    Order by S_Ta1
```

**Hard-coded account numbers `103`, `104`, `109`, `303` are business configuration embedded in code.**
Move them to configuration in the rebuild (§15).

`Tag` is the result flag: `1` = a selection was made, `0` = cancelled (`:76`, `:185`). The accessors
`Get_SSN`, `Get_Code`, `Get_CodeName`, `Get_Name`, `Get_FullName` all return empty/zero unless
`Tag = 1` (`:136-180`). `init_Filter` (`:282-293`) is **unimplemented** — it builds a query, never
opens it, and has a `// Parse filter;` comment where the filter parsing should be.

### 12.7 `MoeinSearchU` — Voucher-line search (`TMoeinSearch`)

Five optional criteria, each gated by its own checkbox (`MoeinSearchU.pas:86-142`):

| Checkbox | Field | Predicate | Empty-value error |
|---|---|---|---|
| `CBed1` | `Bed1` | `and m_bed>=<v>` | `'مبلغ را وارد کنید'` ("Enter the amount") |
| `CBed2` | `Bed2` | `and m_bed<=<v>` | same |
| `CBes1` | `Bes1` | `and m_bes>=<v>` | same |
| `CBes2` | `Bes2` | `and m_bes<=<v>` | same |
| `CDesc1` | `Desc1` | `and len(lTrim(Article))>0 And ( Article Like '%<v>%' )` | `'متن را وارد کنید'` ("Enter the text") |

Base query (`MoeinSearchU.pas:90`):

```sql
 Select * from moein where M_Kind=1 and M_coid=<CO_ID>
```

Defaults: only `CDesc1` is ticked (`MoeinSearchU.pas:235`). Deletion from the result grid is blocked
(`:243-246`). Note the SQL-injection hazard flagged in §11.2.

---

_Prev: [03-12-a-screen-by-screen-ui-specification](03-12-a-screen-by-screen-ui-specification.md) | Next: [03-12-c-screen-by-screen-ui-specification](03-12-c-screen-by-screen-ui-specification.md)_
