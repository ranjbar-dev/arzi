_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 4. Voucher validation rules

### 4.1 Header-level, on save — `SanadEditU.B_SaveClick`

Evaluated **in this exact order** (`SanadEditU.pas:542-610`). Each failure shows the message, focuses
the named control, re-enables the Save button, and aborts.

| # | Condition | Persian message | English translation | Focus | Cite |
|---|---|---|---|---|---|
| 1 | `S_Sanad.IntValue = 0` | `'  شماره سند را وارد کنید  '` | "Enter the voucher number" | `S_Sanad` | `SanadEditU.pas:552` |
| 2 | mode = New **and** a `DMoein` row already has this number | `'شماره سند تکراری است'` | "The voucher number is duplicate" | `S_Sanad` | `SanadEditU.pas:560` |
| 3 | mode = New **and** a `Moein` row already has this number | `'شماره سند تکراری است'` | "The voucher number is duplicate" | `S_Sanad` | `SanadEditU.pas:568` |
| 4 | `not S_Date.Farsi_Valid` | `'  تاریخ را وارد کنید  '` | "Enter the date" | `S_Date` | `SanadEditU.pas:577` |
| 5 | `Trim(S_Desc.Text)` is empty | `'  شرح سند خالی است  '` | "The voucher narration is empty" | `S_Desc` | `SanadEditU.pas:585` |
| 6 | `VSanad.RecordCount = 0` | `'  سند خالی است  '` | "The voucher is empty" | `B_Add` | `SanadEditU.pas:594` |
| 7 | grid footer total debit ≠ total credit | `'  سند متوازن نیست  '` | "The voucher is not balanced" | `G1` | `SanadEditU.pas:606` |

Duplicate checks #2 and #3 use:

```sql
-- SanadEditU.pas:430-431 (DmoeinIsFound)
Select * From Dmoein Where DM_Coid=<coid> And DM_Sanad =<n>
-- SanadEditU.pas:373-376 (moeinIsFound)
Select * , M_Name=( Select LineName From Sarfasl
                    Where M_Ko=S_Ko and M_Mo=S_Mo and M_Ta1=S_Ta1 and M_Ta2=S_Ta2)
    From moein Where M_Coid=<coid> And M_Sanad =<n> Order By M_SSN
```

The balancing check (#7) is **string comparison of the rendered grid footer totals**:

```pascal
// SanadEditU.pas:600-610
   G1.ResetFilter;
   G1.RecalculateSummaryResults(True);
   S1 :=  G1.FooterRow.GetFooterText( VSanad.FieldByName('M_Bed') ) ;
   S2 :=  G1.FooterRow.GetFooterText( VSanad.FieldByName('M_Bes') ) ;
   if S1<>S2 then
      MessageDlg('  سند متوازن نیست  ', mterror, [mbok] , 0 );
```

Note `G1.ResetFilter` first — the grid supports column filters and the totals must cover all rows,
not just the visible ones. **In the rebuild, compare `SUM(debit)` and `SUM(credit)` as integers
server-side.** Do not reproduce the formatted-string comparison.

**Missing validations (record as gaps, decide in §15):**
- **No check that the voucher date lies inside the fiscal year.** `S_Date.Farsi_Valid` is a
  syntactic check on the `TFullDate` control only. The range check `Dm.isValidDate` exists
  (`Dmu.pas:911`) but `SanadEditU` never calls it. The legacy `Sanad_NDU.pas:60-70` did:
  ```pascal
   if Not Dm.isValidDate( D1.Text ) then
      MessageDlg('تاريخ بايد در رنج سال مالي باشد' +#13#10 +
          Dm.from_date +'  <=>  '+ Dm.to_date , mterror, [mbok] , 0 );
  ```
  ("The date must be within the fiscal-year range").
- **No check that the account is not locked** (`Is_Admin_Or_Valid_Daftar` is not consulted).
- **No check that lines with zero amounts are excluded** at header level (it is enforced per-line).

### 4.2 Line-level — `EditArticleMoeinU.B_OKClick`

Evaluated **in this order** (`EditArticleMoeinU.pas:363-388`):

| # | Condition | Persian message | English translation | Focus | Cite |
|---|---|---|---|---|---|
| 1 | `Get_SSn = 0` — account not found **or not a leaf** | `'  کد حساب را به درستی وارد کنید'` | "Enter the account code correctly" | `EKo` | `EditArticleMoeinU.pas:368` |
| 2 | `Bed = 0` **and** `Bes = 0` | `'  مبلغ را وارد کنید  '` | "Enter the amount" | `Bed` | `EditArticleMoeinU.pas:375` |
| 3 | `Trim(Des.Text)` is empty | `'  شرح را وارد کنید  '` | "Enter the description" | `Des` | `EditArticleMoeinU.pas:382` |

On success: `_Ok := 1` and the form closes (`EditArticleMoeinU.pas:386-387`).
On failures 2 and 3 the routine also resets `SSN := 0` so the caller discards the line.

**Both-sides-filled is prevented structurally**, not by a check:

```pascal
// EditArticleMoeinU.pas:317-321
procedure TEditArticleMoein.BedChange(Sender: TObject);
begin
    Bes.ReadOnly := Bed.IntValue >0 ;
    Bed.ReadOnly := Bes.IntValue >0;
end;
```

Entering a debit makes the credit field read-only and vice versa. The legacy screen used
`Enabled` instead of `ReadOnly` (`ArticleMoeinu.pas:361-369`).

**Negative amounts are impossible** — `Bed` and `Bes` are `TEditInt` controls whose key filter
accepts only `'0'..'9'` and backspace (`EditArticleMoeinU.pas:163-167` shows the same filter applied
to the code fields; the amount fields use the component's built-in integer mask).

### 4.3 Legacy line validations — `ArticleMoeinu.SabtClick`

Reachable only through `SanadMoeinu` with `Kind=1`. Order (`ArticleMoeinu.pas:121-141`):

| # | Condition | Persian message | English |
|---|---|---|---|
| 1 | `kol.IntValue = 0` | `'سرفصل کل را وارد کنيد'` | "Enter the general-ledger account" |
| 2 | `moein.IntValue = 0` | `'سرفصل معين را وارد کنيد'` | "Enter the subsidiary account" |
| 3 | `Bed = 0` and `Bes = 0` | `'مبلغ را وارد کنيد'` | "Enter the amount" |

Note: **no description check** and **no leaf check** on this path — a weaker validator than
`EditArticleMoeinU`. Another reason to treat `EditArticleMoeinU` as canonical.

### 4.4 Journal-line validations — `ArticleRooznamehU.Button1Click`

(`ArticleRooznamehU.pas:97-146`)

| # | Condition | Persian message | English |
|---|---|---|---|
| 1 | `kol.IntValue = 0` | `'سرفصل کل را وارد کنيد'` | "Enter the general-ledger account" |
| 2 | `Trim(TKol.Text)` empty (the Kol code did not resolve to a name) | `'سرفصل کل را وارد کنيد'` | "Enter the general-ledger account" |
| 3 | `Bed = 0` and `Bes = 0` | `'مبلغ را وارد کنيد'` | "Enter the amount" |
| 4 | `Trim(Des.Text)` empty | `'شرح را وارد کنيد'` | "Enter the description" |

Journal lines post **at Kol level only**: `M_Mo := 0; M_Ta1 := 0; M_Ta2 := 0` and `M_Kind := 2`
(`ArticleRooznamehU.pas:128-139`). There is a comment `// control tarikh`
(`ArticleRooznamehU.pas:121`) marking a date check that was never written.

### 4.5 Voucher deletion validations

`SanadViewU.B_DeleteClick` (`SanadViewU.pas:193-257`), in order:

1. `Q1.Active = False` or `RecordCount = 0` → silent exit.
2. `DM.Is_New_Sanad_Valid(CO_ID)` → fiscal year must be open (§3.8).
3. `Dm.Is_Admin_Or_Valid_Sanad(S1, CO_ID)` → `'   اجازه دسترسی فقط برای مدیر فعال است  '`
   ("access is enabled for the administrator only") — `SanadViewU.pas:215`.
4. Confirmation `GetYes('حذف سند', ' سند حذف شود ؟ ')` — "delete voucher" / "delete the voucher?"
   (`SanadViewU.pas:219`).
5. Probe query:
   ```sql
   -- SanadViewU.pas:224-225
   Select Count(*) As C, Max(M_Tx) As Tx , Max(M_Id) As ID From Moein
      Where M_Sanad=<n> and M_Coid=<coid>
   ```
   - `C = 0` → `'  سند پیدا نشد  '` ("voucher not found") — `SanadViewU.pas:229`
   - `TX > 0` → `'  سند در حالت تحریر نیست  '` ("the voucher is not in draft state") — `SanadViewU.pas:235`
   - `ID > 0` → `'  سند از اینجا قابل حذف نیست از برنامه های جانبی استفاده کنید '`
     ("the voucher cannot be deleted from here, use the auxiliary programs") — `SanadViewU.pas:241`
6. Delete:
   ```sql
   -- SanadViewU.pas:247-248
   Delete moein  Where M_Sanad=<n> and M_Coid=<coid>
   Delete Dmoein Where DM_Sanad=<n> and DM_Coid=<coid>
   ```
7. `'  سند حذف شد  '` ("the voucher was deleted") — `SanadViewU.pas:251`.

The data-module equivalent `Dm.Delete_Sanad_moein` (`Dmu.pas:1279-1326`) is identical in structure
with different message wording (`'سند پيدا نشد'`, `'سند در حالت تحرير نيست'`,
`'سند غير قابل حذف است '`, success `'سند حذف شد '`).

**Neither path checks `DM_Lock`.** Journal-voucher deletion does (`RooznamehViewU`), Moein-voucher
deletion does not. Inconsistency — see §14.

### 4.6 Single-line deletion — `Dm.Delete_Moein_ssn`

(`Dmu.pas:1328-1369`) Probe:

```sql
Select  Max(M_ID) As ID, Count(*) As N, Max(M_Tx) As TX From Moein Where M_SSN=<lineId>
```

| Condition | Persian | English |
|---|---|---|
| `N = 0` | `'ارتيکل پيدا نشد'` | "Article not found" |
| `TX > 0` | `'سند در حالت تحرير نيست'` | "The voucher is not in draft state" |
| `ID > 0` | `'اجازه حذف آرتيکل از اينجا وجود ندارد'` + newline + `'از برنامه هاي جانبي استفاده شود'` | "You are not permitted to delete the article from here" / "Use the auxiliary programs" |
| success | `'ارتيکل حذف شد '` | "The article was deleted" |

Then `Delete Moein Where M_SSN=<lineId>`. **It does not call `Dmoein_UpdateMab` afterwards**, so the
header totals go stale. Bug — see §14.

---

_Prev: [03-03-c-voucher-sanad-model](03-03-c-voucher-sanad-model.md) | Next: [03-05-a-voucher-line-editing-behaviour](03-05-a-voucher-line-editing-behaviour.md)_
