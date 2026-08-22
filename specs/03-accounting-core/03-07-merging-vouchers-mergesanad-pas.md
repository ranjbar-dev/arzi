_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 7. Merging vouchers — `MergeSanad.pas`

Merges voucher **S1 (source, مبدا)** into voucher **S2 (destination, مقصد)**, moving every line and
every subsystem back-reference. Invoked from `SanadViewU.B_MergeClick` (`SanadViewU.pas:699-716`),
permission `1143` (`'ادغام اسناد'` = "merge vouchers").

### 7.1 Preconditions

`SanadViewU.pas:706-708` first requires `DM.Is_New_Sanad_Valid(CO_ID)`. Then, inside the dialog
(`MergeSanad.pas:73-114`), in order:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `S1 = 0` | `'سند مبدا را وارد کنید'` | "Enter the source voucher" | `MergeSanad.pas:78` |
| 2 | `S2 = 0` | `'سند مقصد را وارد کنید'` | "Enter the destination voucher" | `MergeSanad.pas:84` |
| 3 | `S1 = S2` | `'  سند مبدا و مقصد یکسان هستند'` | "The source and destination vouchers are identical" | `MergeSanad.pas:90` |
| 4 | `Date1 <> Date2` | `'  تاریخ دو سند باید یکسان باشند  '` | "The dates of the two vouchers must be identical" | `MergeSanad.pas:96` |
| 5 | `Tx1.Tag <> Tx2.Tag` | `'  ثبت دو سند باید یکسان باشد  '` | "The posting state of the two vouchers must be the same" | `MergeSanad.pas:102` |
| 6 | `ID1.Tag <> ID2.Tag` | `'   نوع دو سند باید یکسان باشد  '` | "The type of the two vouchers must be the same" | `MergeSanad.pas:108` |
| 7 | confirmation | `'ادغام سند ' + S1 + ' در سند ' + S2 + 'انجام شود ؟'` | "Shall voucher S1 be merged into voucher S2?" | `MergeSanad.pas:113` |

Check 5 compares the two `DM_Tx` values. Check 6 compares the **source-system classification** derived
from the `M_Id` values on each voucher (§3.4) — a manual voucher cannot be merged into an inventory
voucher.

**Neither voucher is required to be in draft state.** Two permanently-posted vouchers of equal state
and date may be merged. Arguably wrong — see §15.

Both vouchers must exist in `DMoein`; otherwise the dialog fills the description with
`'   سند پیدا نشد   '` ("voucher not found") — `MergeSanad.pas:188`, `MergeSanad.pas:258` — and the
state/type tags remain 0, which then makes checks 5 and 6 pass vacuously. **A missing header will not
block a merge.** Defect, see §14.

### 7.2 Algorithm

```sql
-- MergeSanad.pas:118-136, verbatim
 Begin Transaction
 Declare @OLD  int Set @OLD=<S1>
 Declare @New  int Set @New= <S2>
 Declare @COID int Set @COID= <CO_ID>
 Update Moein        Set M_Sanad=@New   Where M_Sanad=@OLD   and M_CoID=@COID
 Update Anbar_Factor Set AF_Sanad=@New  Where AF_Sanad=@OLD  and AF_CoID=@COID
 Update DFish        Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update DCheck       Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update DCheck2      Set S_Sanad=@New   Where S_Sanad=@OLD   and S_CoID=@COID
 Update CheckMaster  Set CM_Sanad=@New  Where CM_Sanad=@OLD  and CM_CoID=@COID
 Delete DMoein Where DM_Sanad=@OLD and DM_Coid=@COID
 Declare @Tbed bigint, @Tbes bigint, @C int
 Select @Tbed=Sum(M_Bed), @TBes=Sum(M_Bes), @C=Count(*) From moein Where M_Sanad=@New and M_Coid=@Coid
 update DMoein  set DM_Count=@C, DM_TBes=@TBes, DM_TBed=@TBed, DM_DESC='<Desc2>'
    where DMoein.DM_Sanad=@new and DM_COid=@COID
Commit
```

Step by step:
1. Repoint all lines of S1 to S2.
2. Repoint the six subsystem back-reference tables.
3. Delete S1's header.
4. Recompute S2's cached totals and line count from the merged line set.
5. Set S2's description to whatever the dialog currently shows for S2.

The dialog sets `Tag := S2.IntValue` **before** `ExecSQL` (`MergeSanad.pas:137-138`) so the caller can
navigate to the surviving voucher. Success: `'   ادغام انجام شد   '` ("the merge was performed").

### 7.3 Constraints and defects

- **`TankhahMaster` is not updated.** The "change voucher number" action updates seven tables
  including `TankhahMaster` (`SanadViewU.pas:354-363`); merge updates only six. Petty-cash documents
  keep a dangling `TM_Sanad` after a merge. **Bug — fix in the rebuild.**
- **`Anbar.dbo.FactorMaster.FM_SanadNo` is not updated either.** "Change number" handles it
  (`SanadViewU.pas:362-363`); merge does not. Same class of bug.
- The confirmation compares against a literal: `if MessageDlg(…, mtWarning, [mbok,mbCancel], 0) <> 1`
  (`MergeSanad.pas:113-114`). `mrOk = 1`, so it works, but it is fragile.
- No check that the merged result stays balanced. Two individually-balanced vouchers sum to a balanced
  one; two drafts may not.

---

_Prev: [03-06-b-automatic-voucher-generation](03-06-b-automatic-voucher-generation.md) | Next: [03-08-journal-rooznameh-generation](03-08-journal-rooznameh-generation.md)_
