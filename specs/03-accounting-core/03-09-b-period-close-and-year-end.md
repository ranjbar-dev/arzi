_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 9.3 Year-end carry-forward — `EnteghalU.pas`

Form caption `'بستن حسابها'`; panel caption `'بستن حسابها و انتقال مانده به دوره بعد '`
("closing the accounts and carrying the balance forward to the next period") — `EnteghalU.dfm:5`,
`EnteghalU.dfm:148`.

**This creates four sets of entries across two fiscal years in one run.**

**Defaults on open** (`EnteghalU.pas:306-336`):

```pascal
    Sanad1.IntValue := dm.New_Sanad;                                     // closing voucher no, year N
    desc1.Text := 'اختتامیه - نقل مانده به دوره بعد';                     // "Closing entry - carrying the balance to the next period"
    Date1.Text := Dm.Base.FieldByName('ToDate').AsString;                 // last day of year N
    Sal1.text  := Co_name + ' ' + Co_Sub;
    Dm.CO_ID := Dm.CO_ID + 1;                                             // temporarily switch to year N+1
    Sanad2.IntValue := dm.New_Sanad;                                     // opening voucher no, year N+1
    desc2.Text := 'افتتاحیه - نقل مانده از دوره قبل';                     // "Opening entry - carrying the balance from the previous period"
    Date2.Text := Dm.Base.FieldByName('FromDate').AsString;               // first day of year N+1
    Dm.CO_ID := Dm.CO_ID - 1;
```

**The next fiscal year is always `CO_ID + 1`.** This hard-codes the year sequence — see §15.

**Two special accounts must be chosen:**

| Field | Persian label | English | Role |
|---|---|---|---|
| `A_Code1` | کد اختتامیه | Closing-entry account | The contra account in year N |
| `A_Code2` | کد افتتاحیه | Opening-entry account | The contra account in year N+1 |

Both are typed as `Ko-Mo-Ta1-Ta2` strings and resolved through `TarafU`
(`EnteghalU.pas:338-348`, `:69-83`). `.Tag` holds the resolved `S_SSN`.

**Validations, in order** (`EnteghalU.pas:85-233`):

| # | Condition | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `Base` has no row for `CO_ID+1` | `'  سال مالی آینده را ایجاد کنید  '` | "Create the next fiscal year" | `EnteghalU.pas:98` |
| 2 | any `Moein` row in year N has `M_tx < 2` | `'  برای بستن سال مالی جدید باید تمام اسناد قطعی شده باشند  '` | "To close the new fiscal year all vouchers must be finalised" | `EnteghalU.pas:108` |
| 3 | `Sanad1 = 0` | `'  سند اختتامیه را وارد کنید '` | "Enter the closing voucher" | `EnteghalU.pas:115` |
| 4 | `Sanad1` already exists in year N | `'  سند اختتامیه تکراری است  '` | "The closing voucher is duplicate" | `EnteghalU.pas:124` |
| 5 | `Sanad2 = 0` | `'  سند افتتاحیه را وارد کنید '` | "Enter the opening voucher" | `EnteghalU.pas:132` |
| 6 | `Sanad2` already exists in year N+1 | `'  سند افتتاحیه تکراری است  '` | "The opening voucher is duplicate" | `EnteghalU.pas:141` |
| 7 | `Date1` invalid | `'  تاریخ سند اختتامیه را وارد کنید '` | "Enter the closing voucher date" | `EnteghalU.pas:149` |
| 8 | `Date1` outside year N's `FromDate..ToDate` | `'  تاریخ سند اختتامیه در درنج مجاز نمیباشد '` | "The closing voucher date is not within the permitted range" *(typo: درنج for در رنج)* | `EnteghalU.pas:159` |
| 9 | `Date2` invalid | `'  تاریخ سند اختتامیه را وارد کنید '` *(wrong label — should say افتتاحیه)* | "Enter the closing voucher date" | `EnteghalU.pas:168` |
| 10 | `Date2` outside year N+1's range | `'  تاریخ سند افتتاحیه در درنج مجاز نمیباشد '` | "The opening voucher date is not within the permitted range" | `EnteghalU.pas:178` |
| 11 | `Trim(Desc1)` empty | `'  شرح بستن حسابهای سال جاری را وارد کنید  '` | "Enter the narration for closing the current year's accounts" | `EnteghalU.pas:186` |
| 12 | `Trim(Desc2)` empty | `'  شرح بستن حسابهای سال آینده را وارد کنید  '` | "Enter the narration for closing the next year's accounts" | `EnteghalU.pas:194` |
| 13 | `A_Code1.Tag = 0` | `'  کد اختتامیه را مشخص کنید  '` | "Specify the closing-entry code" | `EnteghalU.pas:202` |
| 14 | `A_Code2.Tag = 0` | `'  کد افتتاحیه را مشخص کنید  '` | "Specify the opening-entry code" | `EnteghalU.pas:213` |
| 15 | driving query returns no rows | `'  حسابها قبلا انتقال پیدا کرده اند  '` | "The accounts have already been carried forward" | `EnteghalU.pas:229` |
| 16 | final confirmation | `Format('انتقال %d حساب از سال مالی %d  به سال مالی  %d', [n, CO_ID, CO_ID+1])` | "Carry %d accounts from fiscal year %d to fiscal year %d" | `EnteghalU.pas:232` |

Validations 8 and 10 focus the *wrong* control (`Date2` for the `Date1` failure and vice versa) —
`EnteghalU.pas:160`, `:179`. Cosmetic.

**The driving query** `Q1` (`EnteghalU.dfm:659-695`, parameter `COID`):

```sql
Select 123456789 as Code, M_Ko, M_Mo, M_Ta1, M_Ta2
       , Sum(M_Bes-M_Bed) As M_Bes
       , Sum(M_Bed-M_Bes) As M_Bed
    into #R
    From Moein
    Where M_Coid=:COID and M_kind=1
    Group By M_Ko, M_Mo, M_Ta1, M_Ta2
    Order By M_Ko, M_Mo, M_Ta1, M_Ta2

Update #R Set M_Bed=0 Where M_Bed <0
Update #R Set M_Bes=0 Where M_Bes <0
Update #R Set Code = ( Select S_SSN from Sarfasl Where S_Ko=M_Ko
 and S_Mo=M_mo and S_Ta1=M_ta1 and S_Ta2=M_Ta2 )

Delete #R Where M_Bed=0 and M_Bes = 0

Select * From #R Order By M_ko, M_mo, M_ta1, M_ta2
```

(`123456789` is a placeholder that fixes the column's type; the third statement overwrites it.)

Result: one row per **leaf posting account with a non-zero net balance**, with `M_Bed` = net debit
(≥ 0), `M_Bes` = net credit (≥ 0), and `Code` = `Sarfasl.S_SSN`.

**⚠ Critical semantic:** this carries **every account with a balance**, including income-statement
accounts. There is **no filter for balance-sheet accounts.** The system expects the operator to have
already run `NewFinalu` (§9.2) to zero the P&L accounts. If they have not, revenue and expense
balances are carried into the next year. **This ordering dependency is not enforced anywhere.**
See §15.

**The four inserts, per source account** (`EnteghalU.pas:239-282`), all inside one
`Begin Transaction … Commit` **per account** (not one transaction for the whole run):

```pascal
// EnteghalU.pas:241-246 -- build the human-readable code suffix for the contra narration
      _Code3 := Q1['M_Mo']+'-'+Q1['M_Ko'];
      if Q1['M_ta1'] > 0  then _Code3 := Q1['M_Ta1'] + '-' + _Code3;
      if Q1['M_ta2'] > 0  then _Code3 := Q1['M_Ta2'] + '-' + _Code3;
      _Code3 := ' کد ' + _Code3;                       // " code <ta2-ta1-mo-ko>"
```

and the two contra-account value fragments built earlier (`EnteghalU.pas:206-219`):

```pascal
    Taraf.Set_FullCode(A_Code1.Text);
    _Code1 := inttostr(Taraf.Get_SSn) + ',' + Taraf.EKo.Text+',' + Taraf.EMo.Text + ',0' +
              Taraf.ETa1.Text+',0' + Taraf.ETa2.Text ;
```

— a comma-separated `M_Code, M_Ko, M_Mo, M_Ta1, M_Ta2` fragment. The `'0'` prefixes make an empty
level render as `0`.

The four statements, with the comment headers preserved from the source:

```sql
-- EnteghalU.pas:250-276

Begin Transaction

-- (1) بستن کد  ("closing the code") : REVERSE the balance in year N
 insert moein ( M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, M_kind, M_Tx, M_Code,
                M_Ko, M_Mo, M_ta1, M_ta2, M_id, M_link, M_User, Article)
  Values ( <CO_ID>, <Sanad1>, '<Date1>',
           <Q1.M_Bes>,          -- debit  := net CREDIT
           <Q1.M_Bed>,          -- credit := net DEBIT
           0, 1, 0,
           <Q1.Code>, <M_Ko>, <M_Mo>, <M_Ta1>, <M_Ta2>, 0, 0, 68,
           '<Desc1>' )

-- (2) بستن کد به اختتامیه  ("closing the code to the closing account")
 insert moein ( ... same column list ... )
  Values ( <CO_ID>, <Sanad1>, '<Date1>',
           <Q1.M_Bed>,          -- debit  := net DEBIT
           <Q1.M_Bes>,          -- credit := net CREDIT
           0, 1, 0,
           <_Code1>,            -- expands to: Code1SSN, Ko, Mo, 0Ta1, 0Ta2
           0, 0, 68,
           '<Desc1> کد <ta2-ta1-mo-ko>' )

-- (3) ایجاد کد  ("creating the code") : RESTORE the balance in year N+1
 insert moein ( ... )
  Values ( <CO_ID+1>, <Sanad2>, '<Date2>',
           <Q1.M_Bed>,          -- debit  := net DEBIT
           <Q1.M_Bes>,          -- credit := net CREDIT
           0, 1, 0,
           <Q1.Code>, <M_Ko>, <M_Mo>, <M_Ta1>, <M_Ta2>, 0, 0, 68,
           '<Desc2>' )

-- (4) بستن کد به افتتاحیه  ("closing the code to the opening account")
 insert moein ( ... )
  Values ( <CO_ID+1>, <Sanad2>, '<Date2>',
           <Q1.M_Bes>,          -- debit  := net CREDIT
           <Q1.M_Bed>,          -- credit := net DEBIT
           0, 1, 0,
           <_Code2>,
           0, 0, 68,
           '<Desc2> کد <ta2-ta1-mo-ko>' )

Commit
```

Pseudocode for the whole operation:

```
REQUIRE: fiscal year N+1 exists in `Base`
REQUIRE: every Moein line in year N has M_Tx >= 2   (all permanently posted)
REQUIRE: closing voucher number free in year N
REQUIRE: opening voucher number free in year N+1
REQUIRE: closing date within year N, opening date within year N+1
REQUIRE: closing account and opening account both resolve to leaf accounts

balances := leaf accounts of year N with non-zero clamped net balance

FOR each account A in balances:
    (d, c) := clamped_net(A)          -- at most one of d, c is > 0
    BEGIN TRANSACTION
      -- year N, closing voucher:
      POST  A            debit=c  credit=d      -- reverses A to zero
      POST  closing_acct debit=d  credit=c      -- absorbs the balance
      -- year N+1, opening voucher:
      POST  A            debit=d  credit=c      -- re-establishes A
      POST  opening_acct debit=c  credit=d      -- contra
    COMMIT

DMoein_Make(closing_voucher, Date1, Desc1)   in year N
DMoein_Make(opening_voucher, Date2, Desc2)   in year N+1
```

All four lines carry `M_kind = 1` and `M_Tx = 0`.

Header creation for both years (`EnteghalU.pas:285-290`):

```pascal
   Dm.DMoein_Make(Sanad1.IntValue, date1.Farsi_Date, Desc1.Text );
   Dm.CO_ID := Dm.CO_ID + 1;
   Dm.DMoein_Make(Sanad2.IntValue, date2.Farsi_Date, Desc2.Text );
   Dm.CO_ID := Dm.CO_ID - 1;
```

**The global `Dm.CO_ID` is mutated so `DMoein_Make` targets the right year, then restored.** If an
exception fires between the two lines, the whole application is left pointing at the wrong fiscal
year. **Do not port this pattern** — pass the year explicitly.

Progress dialog: `Waitf.initForm('... در حال  انتقال ', 1, Q1.RecordCount+2)` ("carrying forward …").
Success: `'انتقال انجام شد'` ("the carry-forward was performed") — `EnteghalU.pas:292`.

**What is locked afterwards: nothing.** Both vouchers are created with `M_Tx = 0` (draft). Nothing
sets `Base.IsActive = 0` on the closed year. The operator must archive the year by some means not
present in this codebase (§14).

**Hard-coded `M_User = 68`** on all four inserts (`EnteghalU.pas:255`, `:261`, `:268`, `:274`). Every
carry-forward line is attributed to user 68 regardless of who ran it. See §13/§14.

**Re-runnability:** validation 15 fires only when the driving query returns *zero rows*. After a
successful carry-forward, year N's accounts net to zero (the closing entries reversed them), so
`Delete #R Where M_Bed=0 and M_Bes=0` empties the set and a second run is correctly blocked.

---

_Prev: [03-09-a-period-close-and-year-end](03-09-a-period-close-and-year-end.md) | Next: [03-09-c-period-close-and-year-end](03-09-c-period-close-and-year-end.md)_
