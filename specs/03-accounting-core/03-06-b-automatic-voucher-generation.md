_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 6.5 Exact posting rules

Each row below becomes one `Moein` line with `M_Kind = 1`, `M_Tx = 0`, `M_Ted = 0`,
`M_Id = <the type's id>`, `M_Link = FM_SSN`, `M_Code = <account S_SSN>`, and
`M_Ko/M_Mo/M_Ta1/M_Ta2` back-filled afterwards (§6.6 step 3).

**Missing-account guard, applied before every line:** if `Taraf.Get_SSn <= 0` the routine aborts with
a specific message and **leaves any lines already appended in the buffer**. The dialog stays open with
a partial voucher. (A defect — see §14.)

#### (a) `init11` — Opening stock (`FM_ID = 11`, `M_Id = 31`)

Caption `'   صدور سند موجودی اول دوره  '` ("issue opening-stock voucher") — `MakeSanadU.pas:190`.
Default narration: `' عملیات انبار مورخ '` + date ("inventory operation dated …") — `MakeSanadU.pas:203`.

| Line | Account | Debit | Credit | Narration | Cite |
|---|---|---|---|---|---|
| 1 | `Anbar.A_Aval` (opening stock) | `FM_Mab` | — | `'بابت رسید شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:219-238` |
| 2 | `FM_TSSN` (counterparty) | — | `FM_Total` | `'رسید شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:242-261` |
| 3 | `Anbar.A_Kasr` (discount) — **only if `FM_Kasr > 0`** | `FM_Kasr` | — | `'تخفیف رسید شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:264-286` |
| 4 | `Anbar.A_Maliat` (tax) | — | `FM_Maliat` | `'مالیات رسید شماره '` … | **DISABLED** — the guard is `if false then` (`MakeSanadU.pas:289`) |

Error messages:
- `A_Aval` missing → `'  کد اول دوره برای انبار تعریف نشده است  '`
  ("The opening-stock code is not defined for the warehouse") — `MakeSanadU.pas:222`
- `FM_TSSN` missing → `'  کد طرف حساب  برای انبار تعریف نشده است  '`
  ("The counterparty code is not defined for the warehouse") — `MakeSanadU.pas:246`
- `A_Kasr` missing → `'  کد تخفیفات  برای انبار تعریف نشده است  '`
  ("The discounts code is not defined for the warehouse") — `MakeSanadU.pas:269`
- `A_Maliat` missing → `'  کد مالیات  برای انبار تعریف نشده است  '` — `MakeSanadU.pas:294`

Implied balance identity: `FM_Mab + FM_Kasr = FM_Total`. The generator does **not** verify it.

#### (b) `init12` — Purchase / goods receipt (`FM_ID = 12`, `M_Id = 32`)

Caption `'   صدور سند خرید مواد و کالا  '` ("issue materials-and-goods purchase voucher") —
`MakeSanadU.pas:325`. **Structurally identical to `init11`**, substituting `A_Aval` → `A_Kharid`:

| Line | Account | Debit | Credit | Cite |
|---|---|---|---|---|
| 1 | `Anbar.A_Kharid` (purchases) | `FM_Mab` | — | `MakeSanadU.pas:354-373` |
| 2 | `FM_TSSN` (supplier) | — | `FM_Total` | `MakeSanadU.pas:377-396` |
| 3 | `Anbar.A_Kasr`, only if `FM_Kasr > 0` | `FM_Kasr` | — | `MakeSanadU.pas:399-421` |
| 4 | `Anbar.A_Maliat` | — | `FM_Maliat` | **DISABLED** (`if false then`, `MakeSanadU.pas:424`) |

`A_Kharid` missing → `'  کد خرید مواد و کالا برای انبار معرفی نشده است  '`
("The materials-and-goods purchase code has not been introduced for the warehouse") —
`MakeSanadU.pas:357`.

#### (c) `init22` — Sale (`FM_ID = 22`, `M_Id = 33`)

Caption `'   صدور سند فروش  '` ("issue sales voucher") — `MakeSanadU.pas:594`.

| Line | Account | Debit | Credit | Narration | Cite |
|---|---|---|---|---|---|
| 1 | `FM_TSSN` (customer) | `FM_Total` | — | `'فاکتور فروش شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:622-641` |
| 2 | `Anbar.A_Kasr`, only if `FM_Kasr > 0` | `FM_Kasr` | — | `'تخفیف فاکتور فروش شماره '` … | `MakeSanadU.pas:644-666` |
| 3 | `Anbar.A_Foroosh` (sales revenue) | — | `FM_Mab` | `'بابت فاکتور فروش شماره '` … | `MakeSanadU.pas:669-688` |
| 4 | `Anbar.A_Maliat` (tax), only if `FM_Maliat > 0` | — | `FM_Maliat` | `'مالیات فاکتور فروش شماره '` … | `MakeSanadU.pas:691-713` |

**The tax line is live here** (unlike `init11`/`init12`). Implied identity:
`FM_Total + FM_Kasr = FM_Mab + FM_Maliat` — the discount is debited (expense), revenue is recognised
gross, and tax is a separate credit.

`A_Foroosh` missing → `'  کد فروش  برای انبار تعریف نشده است  '` ("The sales code is not defined for
the warehouse") — `MakeSanadU.pas:673`. `FM_TSSN` missing here uses different wording:
`'  کد طرف حساب  برای فاکتور تعریف نشده است  '` ("… not defined for the invoice") —
`MakeSanadU.pas:626`.

**Ordering bug (do not port):** at `MakeSanadU.pas:670` and `:694` the `VSanad.Append` executes
*before* the `Get_SSn <= 0` guard, so a missing account leaves an empty row in the buffer. In
`init11`/`init12` the order is correct.

#### (d) `init13` — Sales return (`FM_ID = 13`, `M_Id = 35`)

Caption `'   صدور سند برگشت از فروش  '` ("issue sales-return voucher") — `MakeSanadU.pas:460`.
The **mirror image of `init22`**:

| Line | Account | Debit | Credit | Narration | Cite |
|---|---|---|---|---|---|
| 1 | `Anbar.A_BForoosh` (sales returns) | `FM_Mab` | — | `'فاکتور شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:488-507` |
| 2 | `Anbar.A_Maliat` (tax), only if `FM_Maliat > 0` | `FM_Maliat` | — | `'مالیات فاکتور شماره '` … | `MakeSanadU.pas:510-532` |
| 3 | `FM_TSSN` (customer) | — | `FM_Total` | `' فاکتور شماره '` + factor + `FM_Desc` | `MakeSanadU.pas:535-554` |
| 4 | `Anbar.A_Kasr` (discount), only if `FM_Kasr > 0` | — | `FM_Kasr` | `'تخفیف فاکتور شماره '` … | `MakeSanadU.pas:557-579` |

`A_BForoosh` missing → `'  کد برگشت از فروش  برای انبار تعریف نشده است  '`
("The sales-return code is not defined for the warehouse") — `MakeSanadU.pas:491`.

### 6.6 Committing a generated voucher — `MakeSanadU.B_OkClick`

(`MakeSanadU.pas:75-132`) Five steps, **not wrapped in a transaction**:

**Step 1 — remove any previously generated lines for this source document:**

```sql
-- MakeSanadU.pas:84-85
 Delete moein
  Where M_Coid=<CO_ID> and  M_Id in(31,32,33,34,35,36,37,38,39) and M_Link=<FM_SSN>
```

Idempotency comes from `(M_Id ∈ inventory range) AND (M_Link = source id)`. It does **not** constrain
`M_Sanad`, so a re-post landing on a different voucher number still cleans up correctly.

**Step 2 — insert the buffered lines:**

```sql
-- MakeSanadU.pas:92-93, verbatim
 Insert Moein (M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, Article, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2,
               M_Id, M_Link, M_User, M_Kind, M_Code, M_Time )
 Values ( <CO_ID>, :Sanad, :Date, :Bed, :Bes, 0, :Article, 0, 0, 0, 0, 0,
          <_ID>, <_SSN>, <userId>, 1, :Code, GetDate() )
```

Executed once per buffer row. `M_Ko/M_Mo/M_Ta1/M_Ta2` are inserted as **zeros**.

**Step 3 — back-fill the account tuple from `M_Code`:**

```sql
-- MakeSanadU.pas:112-114, verbatim
  Update Moein Set Moein.M_Ko=sarfasl.S_Ko, Moein.M_Mo=sarfasl.S_Mo,
                   Moein.M_Ta1=sarfasl.S_Ta1, Moein.M_Ta2=sarfasl.S_Ta2
     from sarfasl Where S_SSN=Moein.M_Code
       and Moein.M_Coid=<CO_ID> and Moein.M_Sanad=<voucherNo>
```

A T-SQL `UPDATE … FROM` correlated join. It rewrites **every line of the voucher**, including manual
lines already present — harmless when their tuple already matches `M_Code`, but a manual line with
`M_Code = 0` (as written by `ArticleMoeinu.pas:160` and `SanadMoeinu.pas:327`) matches no `Sarfasl`
row and is left untouched by the join. **However** if a `Sarfasl` row with `S_SSN = 0` ever existed
the tuple would be zeroed. See §14.

**Step 4 — mark the source document as posted:**

```sql
-- MakeSanadU.pas:121-122
  Update Anbar.Dbo.FactorMaster Set FM_Lock=2, FM_SanadNo=<voucherNo>, FM_SanadDate='<jalali>'
     Where FM_SSN= <FM_SSN>
```

**Step 5 —** `Dm.DMoein_Make(voucherNo, date, desc)` (`MakeSanadU.pas:126`), then
`'  سند ذخیره شد   '` ("the voucher was saved") and `Tag := 1` to tell the caller it succeeded.

### 6.7 Un-posting — `SodoorSanadU.B_DeleteClick`

(`SodoorSanadU.pas:211-270`) Guards, in order:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `Is_New_Sanad_Valid` | (see §3.8) | fiscal year open | `SodoorSanadU.pas:216` |
| 2 | `FM_Lock <> 2` | `'   برروی این فاکتور هنوز سند صادر نشده است   '` | "No voucher has been issued against this invoice yet" | `SodoorSanadU.pas:222` |
| 3 | `FM_COID <> Dm.CO_ID` | `'   سال صدور سند با سال مالی جاری یکسان نیست   '` | "The voucher issue year is not the same as the current fiscal year" | `SodoorSanadU.pas:231` |
| 4 | `Dm.Get_SanadMaxTX(sanad) <> 0` | `Format('   سند %d را در حالت تحریر قرار دهید' , [_Sanad])` | "Put voucher %d into draft state" | `SodoorSanadU.pas:236` |
| 5 | confirmation | `Format('   برای حذف سند %d مطمئن هستید ؟ ', [_sanad])` | "Are you sure about deleting voucher %d?" | `SodoorSanadU.pas:240` |

```sql
-- SodoorSanadU.pas:250-262, verbatim
 Begin Transaction
Declare @Coid int Set @Coid=<FM_COID>
Declare @Sanad int Set @Sanad=<FM_SanadNo>
Declare @link int Set @Link=<FM_SSN>
Declare @Factor int Set @Factor=<FM_Factor>
Delete moein Where M_Coid=@Coid and M_Sanad=@Sanad and M_Link=@Link and M_id in (32,33,35)
Update Anbar.DBO.FactorMaster  Set FM_SanadNo=0 , FM_SanadDate='' , FM_Lock=1
    Where FM_Coid=@Coid and FM_Factor=@Factor and FM_SSN=@Link
 Commit
```

Then `DM.Dmoein_UpdateMab(_Sanad)` (`SodoorSanadU.pas:265`) — which also deletes the header if the
voucher is now empty. Success: `'     انجام شد      '` ("done"), shown with `mterror` styling
(cosmetic bug, `SodoorSanadU.pas:268`).

**Asymmetry bug — must be fixed, not ported:** creation deletes `M_id in (31..39)` but un-posting
deletes only `M_id in (32,33,35)`. **Opening-stock lines (`M_Id = 31`) are never removed.** Un-posting
an opening-stock document resets `FM_Lock` to 1 while leaving its ledger lines in place, allowing a
duplicate posting. See §14/§15.

### 6.8 Source-document linkage summary

| Legacy column | On table | Points to | Proposed name |
|---|---|---|---|
| `Moein.M_Id` | voucher line | *class* of the source subsystem/document | `source_kind` |
| `Moein.M_Link` | voucher line | `Anbar.FactorMaster.FM_SSN` (or a treasury doc PK) | `source_id` |
| `FactorMaster.FM_SanadNo` | source document | `DMoein.DM_Sanad` | `voucher_number` |
| `FactorMaster.FM_SanadDate` | source document | `DMoein.DM_Date` | `voucher_date` |
| `FactorMaster.FM_Lock` | source document | posting state 0/1/2 | `posting_status` |
| `DCheck.S_linkPrg` | cheque | source module id | `source_module` |
| `DCheck.S_LinkSSN` | cheque | source document id | `source_id` |
| `DCheck.S_Sanad` | cheque | `DMoein.DM_Sanad` | `voucher_number` |

`S_linkPrg` / `S_LinkSSN` (declared at `Dmu.pas:68-69`) are the treasury equivalents of
`M_Id` / `M_Link`. `SodoorSanadU.pas:341-342` shows them reconciling settlements against an invoice
number.

**Rebuild recommendation:** replace the `(source_kind, source_id)` pair with a proper link table plus
a real foreign key per source type, or a single `source_document_id` with a discriminator enum.
Preserve the idempotent-regeneration semantics.

---

_Prev: [03-06-a-automatic-voucher-generation](03-06-a-automatic-voucher-generation.md) | Next: [03-07-merging-vouchers-mergesanad-pas](03-07-merging-vouchers-mergesanad-pas.md)_
