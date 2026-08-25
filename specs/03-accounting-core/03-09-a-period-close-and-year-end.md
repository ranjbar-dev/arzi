_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 9. Period close and year-end

There are **four distinct operations** and they are easy to confuse. In execution order for a normal
year-end:

| Order | Operation | Unit | Menu | Purpose |
|---|---|---|---|---|
| 1 | **Close the books** (بستن حسابها) | `NewFinalu.pas` | `B_Enteghal1` | Zero out income-statement accounts into a summary account |
| 2 | **Carry balances forward** (انتقال مانده) | `EnteghalU.pas` | `B_Enteghal3` | Create the closing and opening vouchers for the year boundary |
| — | *(dead)* single-Kol close | `FinalU.pas` | — | Superseded by 1 |
| — | Export balances to file | `BastanHesab.pas` | `B_CloseMoein` | Not a close; a `.GGS` export |
| — | Fill a voucher with reversed balances | `SanadMoeinu.fillsanad` | popup on the legacy voucher screen | Manual helper |

Both 1 and 2 require **supervisor** rights (`Mainu.pas:958-960`):

```pascal
    B_Enteghal1.Enabled := Dm.Password.FieldByName('Supervisor').AsInteger = 1;
    B_Enteghal2.Enabled := Dm.Password.FieldByName('Supervisor').AsInteger = 1;
    B_Enteghal3.Enabled := Dm.Password.FieldByName('Supervisor').AsInteger = 1;
```

### 9.1 The balance convention used everywhere

Every closing routine computes an account's balance the same way — as **two non-negative numbers**,
at most one of which is non-zero:

```sql
  Sum(M_Bed-M_Bes) As BedR    -- net debit
, Sum(M_Bes-M_Bed) As BesR    -- net credit
...
Update #R Set BedR=0 Where BedR<0
Update #R Set BesR=0 Where BesR<0
```

If the net is a debit, `BedR = |net|` and `BesR = 0`; if a credit, the reverse; if zero, both are 0
and the row is usually deleted. This avoids signed arithmetic entirely.

**Preserve this convention** — it is what makes the closing entries symmetric without any notion of
account nature.

### 9.2 Closing the books — `NewFinalu.pas`

Menu caption `'بستن حسابها'` ("closing the accounts"). Purpose: **transfer the balances of a chosen
set of Kol accounts into one destination account** — i.e. close the profit-and-loss accounts to a
summary account.

**Step 1 — list candidate Kol accounts.** `Q1` (`NewFinalu.dfm:358-376`):

```sql
Declare @Co int Set @Co=:Co

Select  Tik=Cast( 0 as tinyint), M_Ko, K_Name = ( Select S_Name From sarfasl Where S_Ko=M_Ko and S_Mo=0)
       , Sum(M_Bed) As Bed , Sum(M_Bes) As Bes
       , Sum(M_Bed-M_Bes) As BedR
       , Sum(M_Bes-M_Bed) As BesR
into #R
From Moein
Where M_Kind=1 And M_COID=@Co
Group By M_Ko

Delete #R Where Bes - Bed = 0
Update #R Set BedR=0 Where BedR<0
Update #R Set BesR=0 Where BesR<0

Select *  From #R Order By M_Ko
```

**`Tik`** is a client-side tick box: `0` = not selected, `1` = selected. Toggled by double-clicking the
grid row (`NewFinalu.pas:254-264`):

```pascal
    I := Q1.FieldByName('Tik').Value;
    if I=1 then I:=0 Else I:=1;
    Q1.Edit; Q1.FieldByName('Tik').Value := I; Q1.Post;
```

**Note the filter `Delete #R Where Bes - Bed = 0`** — Kol accounts with zero net balance are hidden.

**Step 2 — pick the destination account.** `_Bed` edit + `B_Bed` browse button, backed by the `TarafU`
account-code control (`NewFinalu.pas:62-73`, `:284-290`). `_Bed.Tag` holds the resolved `S_SSN`.

**Step 3 — validations**, in order (`NewFinalu.pas:80-157`):

| # | Condition | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `_Sanad = 0` | `'  شماره سند را وارد کنید  '` | "Enter the voucher number" | `NewFinalu.pas:89` |
| 2 | voucher already exists → **confirmation, not an error** | `'  سند از قبل وجود دارد   ' + newline + ' ایا به این سند اضافه شود ؟ '` | "The voucher already exists / Shall it be appended to this voucher?" | `NewFinalu.pas:99` |
| 3 | existing voucher has `M_TX > 0` | `'  سند را در حالت تحریر قرار دهید  '` | "Put the voucher into draft state" | `NewFinalu.pas:105` |
| 4 | existing voucher has `M_ID > 0` | `'  امکان افزودن به این سند را ندارید  '` | "You cannot add to this voucher" | `NewFinalu.pas:112` |
| 5 | no rows ticked | `'  هیچ انتخابی انجام نشده است  '` | "No selection has been made" | `NewFinalu.pas:127` |
| 6 | `_Bed.Tag = 0` (no destination) | `'  مقصد انتخاب نشده است  '` | "The destination has not been selected" | `NewFinalu.pas:135` |
| 7 | destination Kol appears in the ticked list | `'  مقصد انتخاب شده نباید در لیست کل باشد    '` | "The selected destination must not be in the general-ledger list" | `NewFinalu.pas:146` |
| 8 | `Trim(_Desc)` empty | `'  شرح سند را وارد کنید  '` | "Enter the voucher narration" | `NewFinalu.pas:154` |

Validation 7 is a **string containment test** on the comma-joined Kol list, guarded by sentinels so
that `,11,` does not match inside `,111,`:

```pascal
// NewFinalu.pas:139-150
   S2:= _Bed.Text;                       // e.g. "705-1-3"
   S1 := '99999'+S1+',99999';            // S1 is ",705,801,802"
   I := Pos('-',S2);
   S2:=','+Copy(S2,1,I-1)+',';           // ",705,"
   I := Pos( S2, S1 );
   if I>0 then  <reject>
   S1 := Copy(S1,7,Length(S1)-12);       // strip the sentinels back off
```

**This works only when the destination code contains a `-`.** If the user types a bare Kol code,
`Pos('-',S2)` returns 0, `Copy(S2,1,-1)` yields an empty string, `S2` becomes `',,'` which never
matches — the guard **silently fails open**. Fix in the rebuild by comparing integers.

**Step 4 — aggregate the ticked accounts down to leaf level.** `Q2` (`NewFinalu.pas:160-181`):

```sql
 Declare @Desc varchar(200) Set @Desc='<narration>'
 Declare @Coid int Set @Coid=<CO_ID>
 Declare @Sanad int set @Sanad=<voucherNo>
 Declare @Date varchar(10) Set @Date='<jalali>'
 Declare @User int Set @User=<userId>
 IF OBJECT_ID('tempdb..#R') IS NOT NULL    DROP TABLE #R

 Select M_Code=( Select S_SSN from Sarfasl Where S_Ko=M_Ko and S_Mo=M_Mo and S_Ta1=M_Ta1 and S_Ta2=M_Ta2)
   ,M_Ko, M_Mo, M_Ta1, M_Ta2
   ,Sum(M_Bed-M_Bes) As M_Bed , Sum(M_Bes-M_Bed) As M_Bes
 into #R From moein Where M_Ko in (<tickedKolList>) and M_Kind=1 And M_Coid= @Coid
 Group By M_Ko, M_Mo, M_Ta1, M_Ta2
 Update #R Set M_Bes=0 where M_Bes<0
 Update #R Set M_Bed=0 where M_Bed<0
 Select * from #R order by M_Ko, M_Mo, M_Ta1, M_Ta2
```

The roll-up is at the **full leaf tuple**, not at Kol level: each posting account under a ticked Kol
gets its own closing pair.

**Step 5 — emit two lines per source account** (`NewFinalu.pas:191-214`), via the parameterised
insert `Q3` (`NewFinalu.dfm:323-343`):

```sql
Declare @Coid int Set @Coid=:Coid
Declare @Sanad int Set @Sanad=:Sanad

insert Moein (M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, Article, M_Tx,
              M_Ko, M_Mo, M_Ta1, M_Ta2, M_Id, M_Link, M_User, M_Kind, M_Code)
Values (@Coid, @Sanad, :Date, :Bed, :Bes, '0', :Article, 0,
              0, 0, 0, 0, 0, 0, :UserID, 1, :Code)
```

The pairing logic:

```pascal
// NewFinalu.pas:191-214
   for I := 1 to RC do
   Begin
      if Q2.FieldValues['M_Bed']>0 then          // source has a NET DEBIT balance
      Begin
         // debit the destination
         Q3.Parameters['Bed'] := Q2['M_Bed'];  Q3.Parameters['Bes'] := 0;
         Q3.Parameters['Code'] := _Bed.Tag;     Q3.ExecSQL;
         // credit the source (zeroing it)
         Q3.Parameters['Bed'] := 0;             Q3.Parameters['Bes'] := Q2['M_Bed'];
         Q3.Parameters['Code'] := Q2['M_Code']; Q3.ExecSQL;
      End Else begin                             // source has a NET CREDIT balance
         // debit the source (zeroing it)
         Q3.Parameters['Bed'] := Q2['M_Bes'];   Q3.Parameters['Bes'] := 0;
         Q3.Parameters['Code'] := Q2['M_Code']; Q3.ExecSQL;
         // credit the destination
         Q3.Parameters['Bed'] := 0;             Q3.Parameters['Bes'] := Q2['M_Bes'];
         Q3.Parameters['Code'] := _Bed.Tag;     Q3.ExecSQL;
      end;
      Q2.Next;
   End;
```

Pseudocode:

```
FOR each leaf account A under the ticked Kol accounts:
    (net_debit, net_credit) = clamped_balance(A)
    IF net_debit > 0:
        POST  DEBIT  destination  net_debit
        POST  CREDIT A            net_debit
    ELSE:
        POST  DEBIT  A            net_credit
        POST  CREDIT destination  net_credit
```

Each source account is zeroed and the net lands on the destination. The voucher is balanced by
construction.

**Step 6 — back-fill the account tuple** (`NewFinalu.pas:216-224`):

```sql
 Declare @Coid int Set @Coid = <CO_ID>
 Declare @sanad int set @Sanad = <voucherNo>
 Update moein Set Moein.M_Ko=Sarfasl.S_Ko, Moein.M_Mo=Sarfasl.S_Mo,
                  Moein.M_Ta1=Sarfasl.S_Ta1, Moein.M_Ta2=Sarfasl.S_Ta2
 From Moein Join Sarfasl on Moein.M_Code=Sarfasl.S_SSN
 Where Moein.M_Coid=@Coid and Moein.M_Sanad=@Sanad
```

Same pattern (and same hazard) as `MakeSanadU` step 3 — see §6.6.

**Step 7 —** `DM.DMoein_Make(...)` then `DM.Dmoein_UpdateMab(...)` (`NewFinalu.pas:227-228`), then
`MessageDlg(inttostr(2*RC)+ ' خط سند ثبت شد ')` ("%d voucher lines were recorded").

**Nothing is locked afterwards.** The closing voucher is an ordinary draft; the user must approve and
post it through the normal state machine (§3.6). The screen can be re-run, but after the first run the
source accounts net to zero, so `Delete #R Where Bes - Bed = 0` empties the candidate list on reload
(`NewFinalu.pas:232-233`) — self-limiting.

---

_Prev: [03-07-merging-vouchers-mergesanad-pas](03-07-merging-vouchers-mergesanad-pas.md) | Next: [03-09-b-period-close-and-year-end](03-09-b-period-close-and-year-end.md)_
