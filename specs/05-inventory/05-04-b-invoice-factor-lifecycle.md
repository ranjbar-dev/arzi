_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

#### 4.2.2 Save — the only write path

`B_SaveClick` (`AnbarFactorU.pas:567-667`). Ten steps, quoted and annotated:

**Step 1 — editability guard** (`:575-578`):
```pascal
if _NewEditView > 2  Then Begin
    Application.MessageBox('اجازه تغييرات در فاکتور را نداريد', 'Error');
    Exit;
End;
```
"You are not permitted to change the invoice."

**Step 2 — counterparty guard, which is broken** (`:579-583`):
```pascal
if not S_Bed.tag=0 then Begin
   Application.MessageBox('کد حساب را وارد کنيد','InFormation');
   ActiveControl := S_Bed;
   Exit;
End;
```
> **Defect — this validation is unreachable.** In Object Pascal `not` binds tighter than `=`, and
> `S_Bed.Tag` is an `Integer`, so `not` is the *bitwise* complement. The expression parses as
> `(not S_Bed.Tag) = 0`, which is true only when `S_Bed.Tag = -1`. `S_Bed.Tag` is only ever
> assigned `0` or a valid `Sarfasl.S_SSN` (`S_BedChange`, `:356-362`; `B_BedClick`, `:548`), never
> `-1`. **The message can never fire and an invoice can be saved with no counterparty at all**,
> writing `AF_Customer = 0`. The author's intent was plainly `if S_Bed.Tag = 0 then`.
> Consequences: the voucher line produced by `Anbar_AddToFactor` gets account id `0` (§10), and
> `LoadFactor` later resolves it through `Taraf.Set_SSN(0)` to an empty code (`:707-708`).
> **This is a high-severity data-integrity hole and the first thing a rebuild must close.**

**Step 3 — non-empty guard** (`:584-588`): `فاکتور خالي است` — "the invoice is empty".

**Step 4 — allocate the voucher number** (`:591-593`):
```pascal
G1.resetFilter;
_OLD := AF_Sanad.intValue;
AF_Sanad.intValue := DM.Get_NewSanad_DateID(AF_Date.Farsi_Date, '1,2,3,4,5,6,7,8,9' ) ;
```
`Get_NewSanad_DateID` (`Dmu.pas:1461-1477`) is a **voucher-merging allocator**, not a sequence:

```sql
Select isnull( Max(M_Sanad) , 0 ) as S
  From Moein
  Where M_Tx=0 and M_Coid=<year> and M_ID in( 1,2,3,4,5,6,7,8,9 ) and M_Date='<jalali>'
```
If a *draft* (`M_Tx = 0`) inventory voucher already exists on that Jalali date, the invoice joins
it; otherwise `New_Sanad` issues a fresh number. So **all inventory invoices dated the same day
share one accounting voucher** — until someone finalises it, after which the next invoice on that
date starts a new one. The `M_ID` range `1..9` is subsystem A's, disjoint from subsystem B's
`31..39` (§5.3.1) and from pistachio's `34`.

Note `_OLD` is captured *before* reallocation: re-saving an invoice can move it to a **different**
voucher, and the old voucher's total is recomputed at `:649`.

**Step 5 — allocate or locate the invoice number** (`:595-603`):
```pascal
if _NewEditView = 1 then Begin
   AF_Factor.intValue :=  Dm.New_AnbarFactor ;
   Dm.AnbarFactor.Append;
End Else Begin
   Dm.AnbarFactor.Locate('AF_Coid;AF_Factor', vararrayof([Dm.Co_id, AF_Factor.Text]), …);
   dm.AnbarFactor.Edit;
End;
```
`New_AnbarFactor` (`Dmu.pas:1253-1262`) is `Select isnull(Max(AF_Factor),0)+1 … Where AF_COid=<year>`
— unlocked, unconstrained, per §5.4 a concurrency hazard. Numbering is **per fiscal year**, not
per type: receipts, issues and both returns draw from one sequence.

**Step 6 — write the header** (`:606-614`). Eight columns:
`AF_COID`, `AF_Type`, `AF_Factor`, `AF_Sanad`, `AF_Date`, `AF_Customer` (= `S_Bed.Tag`),
`AF_CustomerN` (= `Taraf.Get_LastName`, denormalised), `AF_Desc`.
The money totals are **not** written here — see step 10.

> **`AF_CustomerN` is denormalised from a global singleton.** `Taraf` is the shared account-picker
> object (`docs/01-glossary.md` §6b). `Taraf.Get_LastName` returns whatever was last resolved into
> it. Because `S_BedChange` calls `Taraf.Set_FullCode` on every keystroke this is *usually* right,
> but if any other screen touched `Taraf` since, the name saved does not belong to the account id
> saved. There is no re-resolution at save time.

**Step 7 — destroy the previous version** (`:617-622`):
```sql
Delete Anbar_FactorD where AFD_Coid=<year> and AFD_Factor=<n>
Delete Moein          where M_Coid=<year> and M_ID=1 and M_Link=<n>
```
executed as one `ExecSQL` **with no transaction**. Note the `Moein` delete is scoped by
`M_ID = 1` and `M_Link` but **not by `M_Sanad`** — which is what rescues the re-save case
described in §5.4, because the lines under the *old* voucher are removed too.

But note also: it deletes only `M_ID = 1`. `Get_NewSanad_DateID` was asked for the range `1..9`,
and `Anbar_AddToFactor`'s parameter list carries `@Type` — so if the stored procedure ever writes
`M_ID = 2` for a sales invoice (which the `1..9` allocation range strongly implies was the
design), **those lines would never be deleted on re-save and would double.** The SP body is not in
the repository. This is the highest-priority unknown in the module — **open question §14**.

**Step 8 — re-insert every line** (`:626-645`): a client-side loop calling `SP_AnbarAddToFactor`
once per row, `ExecProc`, result ignored. Per §5.4 this is not transactional and line identity
(`AFD_SSN`) is not stable across edits.

**Step 9 — voucher header** (`:647-649`):
```pascal
DM.DMoein_Make( AF_Sanad.intvalue, AF_Date.farsi_Date , 'فروش کالا ' );
DM.Dmoein_UpdateMab(AF_Sanad.intvalue );
if _OLD>0 then DM.Dmoein_UpdateMab(_OLD );
```
The narration is the literal `'فروش کالا '` — "goods sale" — **for all four document types**. A
goods receipt posts a voucher headed "goods sale". Cosmetic, but it is what appears in the journal.

**Step 10 — recompute the four header caches** (`:654-661`): four separate correlated `UPDATE`s
over `Anbar_FactorD`, one per column:

| Header column | Source |
|---|---|
| `AF_Total` | `Sum(AFD_Total)` |
| `AF_Kasr` | `Sum(AFD_Kasr)` |
| `AF_Maliat` | `Sum(AFD_Maliat)` |
| `AF_Mab` | `Sum(AFD_Kol)` |

Note the naming trap: `AF_Mab` ("amount") is the **gross** total and `AF_Total` is the **net**
total — the opposite of what the names suggest, and the opposite of the line-level pairing where
`AFD_Kol` is gross and `AFD_Total` is net. See §16.

Then `ذخيره انجام شد` ("save completed") and the form closes.


---

[← 4. The invoice (Factor) lifecycle (part a)](05-04-a-invoice-factor-lifecycle.md) | [index](00-index.md) | [4. The invoice (Factor) lifecycle (part c) →](05-04-c-invoice-factor-lifecycle.md)
