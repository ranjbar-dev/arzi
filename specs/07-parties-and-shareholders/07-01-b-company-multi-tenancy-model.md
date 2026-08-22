_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

### 1.5 Creating a new fiscal year (`MakeNewU.pas`)

Reached from `Mainu.pas:496-499` (`MakeNew.init`).

Pre-fill (`MakeNewU.pas:70-81`): copies `Co_Name`, `Co_Sub`, `BackupDir` from the *current* row;
proposes `Co_ID + 1`; proposes `FromDate`/`ToDate` with the Jalali year incremented by one
(`MakeNewU.pas:75`, `:77`).

Duplicate check and insert:

```pascal
// MakeNewU.pas:104-126
    T1.Close;
    T1.TableName := 'Base';
    T1.Open;
    if T1.Locate('CO_ID', Trim(COID.Text) , [] ) Then
    Begin
       Application.MessageBox('شماره شناسايي تکراري است','Error');   // "Duplicate identifier"
       T1.Close;
       ActiveControl := COID;
       Exit;
    End;

    // Create Base
    T1.Append;
    For I:=0 to T1.FieldCount-1 Do
        T1.FieldByName( T1.Fields[i].FieldName ).AsString :=  DM.Base.FieldByName( T1.Fields[i].FieldName ).AsString ;
    T1.FieldByName('Co_ID').AsString := Coid.Text;
    T1.FieldByName('FromDate').AsString := FromDate.Text;
    T1.FieldByName('Todate').AsString := ToDate.Text;
    T1.FieldByName('BackupDir').AsString := Backup_Dir.Text;
    T1.FieldByName('Co_Name').AsString := CoName.Text;
    T1.FieldByName('Co_Sub').AsString := CoSuB.Text;
    T1.Post;
```

The new row is a **field-by-field clone** of the current row (including `C1081`, `C1082`,
`Kh1..Kh8`, `No_Ko..No_Ta2`, `ARM`), then five fields are overridden. Success message
`سال مالي جديد اضافه شد` = "New fiscal year added" (`MakeNewU.pas:151`).

> Note: `IsActive` is cloned, not forced. If the source year is archived the new year is created
> archived. Flagged in §12-Q4.

### 1.6 Fiscal-year rollover (`EnteghalU.pas`)

`انتقال` = "carry forward". Reached from `Mainu.pas:629-632`. This is where every party account's
balance moves from year *N* to year *N+1*, so it belongs to this domain.

Defaults (`EnteghalU.pas:~305-322`):

* Closing voucher number = `dm.New_Sanad` in year `CO_ID`, dated `Base.ToDate`,
  description `اختتامیه - نقل مانده به دوره بعد` ("Closing entry — carry balance to next period").
* Opening voucher number = `dm.New_Sanad` in year `CO_ID+1`, dated `Base.FromDate`,
  description `افتتاحیه - نقل مانده از دوره قبل` ("Opening entry — carry balance from previous period").

Validation chain, in order (all `EnteghalU.pas`):

| # | Condition | Persian message | English | Line |
|---|---|---|---|---|
| 1 | Next-year `Base` row must exist | `سال مالی آینده را ایجاد کنید` | "Create the next fiscal year first" | ~97 |
| 2 | No `Moein` row with `M_tx < 2` in current year | `برای بستن سال مالی جدید باید تمام اسناد قطعی شده باشند` | "To close the year all vouchers must be finalised" | ~106 |
| 3 | Closing voucher number entered | `سند اختتامیه را وارد کنید` | "Enter the closing voucher number" | ~113 |
| 4 | Closing voucher number unused in current year | `سند اختتامیه تکراری است` | "Closing voucher number already exists" | ~124 |
| 5 | Opening voucher number entered | `سند افتتاحیه را وارد کنید` | "Enter the opening voucher number" | ~131 |
| 6 | Opening voucher number unused in next year | `سند افتتاحیه تکراری است` | "Opening voucher number already exists" | ~141 |
| 7 | Closing date valid | `تاریخ سند اختتامیه را وارد کنید` | "Enter the closing voucher date" | ~148 |
| 8 | Closing date within current year's `FromDate..ToDate` | `تاریخ سند اختتامیه در درنج مجاز نمیباشد` | "Closing voucher date is out of the allowed range" *(sic: «درنج» is a typo for «رنج»/range)* | ~160 |
| 9 | Opening date valid | `تاریخ سند اختتامیه را وارد کنید` *(copy-paste: says "closing")* | "Enter the … voucher date" | ~168 |
| 10 | Opening date within next year's range | `تاریخ سند افتتاحیه در درنج مجاز نمیباشد` | "Opening voucher date is out of the allowed range" | ~176 |
| 11 | Closing description non-empty | `شرح بستن حسابهای سال جاری را وارد کنید` | "Enter the description for closing the current year's accounts" | ~184 |
| 12 | Opening description non-empty | `شرح بستن حسابهای سال آینده را وارد کنید` | "Enter the description for the next year's accounts" | ~192 |
| 13 | Closing control account selected | `کد اختتامیه را مشخص کنید` | "Specify the closing account code" | ~200 |
| 14 | Opening control account selected | `کد افتتاحیه را مشخص کنید` | "Specify the opening account code" | ~212 |
| 15 | Something left to carry | `حسابها قبلا انتقال پیدا کرده اند` | "The accounts have already been carried forward" | ~227 |

Confirmation prompt (`EnteghalU.pas:~230`):
`انتقال %d حساب از سال مالی %d  به سال مالی  %d`
= "Carry forward %d accounts from fiscal year %d to fiscal year %d".

The closing/opening control accounts are chosen through the **`TarafU` picker** (§2.1) and encoded
as a 5-tuple `SSN, Ko, Mo, Ta1, Ta2`:

```pascal
// EnteghalU.pas:~203-206
    Taraf.Set_FullCode(A_Code1.Text);
    _Code1 := inttostr(Taraf.Get_SSn) + ',' + Taraf.EKo.Text+',' + Taraf.EMo.Text + ',0' +
              Taraf.ETa1.Text+',0' + Taraf.ETa2.Text ;
```

> **Latent bug worth reproducing consciously or fixing:** the `',0' + Taraf.ETa1.Text` concatenation
> prefixes a literal `0` to the Tafsil digits rather than defaulting an empty box to `0`. For an
> empty `ETa1` it yields `,0`, which is correct by accident; for `ETa1='22'` it yields `,022`, which
> SQL parses as `22`. It is fragile, not wrong. Flagged in §12-Q6.

**Posting rules per carried account** (`EnteghalU.pas:~245-275`). For each row returned by the
carry-forward query, four `Moein` rows are inserted inside one `Begin Transaction … Commit`:

| # | Year | Voucher | Account | Debit (`M_Bed`) | Credit (`M_Bes`) | Description |
|---|---|---|---|---|---|---|
| 1 | `CO_ID` | closing | the account itself | previous `M_Bes` | previous `M_Bed` | `Desc1` |
| 2 | `CO_ID` | closing | closing control account (`_Code1`) | previous `M_Bed` | previous `M_Bes` | `Desc1 + ' کد ' + <code>` |
| 3 | `CO_ID+1` | opening | the account itself | previous `M_Bed` | previous `M_Bes` | `Desc2` |
| 4 | `CO_ID+1` | opening | opening control account (`_Code2`) | previous `M_Bes` | previous `M_Bed` | `Desc2 + ' کد ' + <code>` |

All four rows use `M_kind=1`, `M_Tx=0`, `M_id=0`, `M_link=0`, `M_User=68`, `M_Ted=0`.

> `M_User=68` is **hard-coded**, not `Dm.userId`. See §12-Q7.

Voucher headers are then materialised via `Dm.DMoein_Make` — note the ugly temporary mutation of the
ambient `CO_ID`:

```pascal
// EnteghalU.pas:~283-289
   Dm.DMoein_Make(Sanad1.IntValue, date1.Farsi_Date, Desc1.Text );
   Waitf.Gotonextposition;
   Dm.CO_ID := Dm.CO_ID + 1;
   Dm.DMoein_Make(Sanad2.IntValue, date2.Farsi_Date, Desc2.Text );
   Dm.CO_ID := Dm.CO_ID - 1;
```

Final message `انتقال انجام شد` = "Carry-forward completed".

**What rollover does to parties:** every party detail account with a non-zero balance appears in the
carry set, so a party's opening balance in year *N+1* is its closing balance in year *N*. The party
*master* record is untouched — it is global.

### 1.7 The external `Saham` share-registry database

```pascal
// Dmu.pas:757-780
// Check for databases
    Saham_DB := 'Saham.Dbo';
    Saham_F := '\\pesteh\SahamData\';
    Anbar_DB := 'Anbar.Dbo';
    Basc_DB := 'Rppc_Solution.Dbo';

    Q1.Close;
    Q1.ConnectionString := Ado.ConnectionString;
    Q1.SQL.Add('Declare @Saham varchar(20) Set @Saham=''Saham.Dbo'' ');
    Q1.SQL.Add('Declare @Anbar varchar(20) Set @Anbar=''Anbar.Dbo'' ');
    Q1.SQL.Add('Declare @Basc  varchar(20) Set @Basc=''RPPC_Solution.Dbo'' ');

    Q1.SQL.Add(' if DB_ID(''Saham'') is null Set @Saham='''' ');
    Q1.SQL.Add(' if DB_ID(''Anbar'') is null Set @Anbar='''' ');
    Q1.SQL.Add(' if DB_ID(''Rppc_Solution'') is null Set @Basc='''' ');

    Q1.SQL.Add('   Select @Saham As Saham, @Basc As Basc, @Anbar As Anbar ');
    Q1.Open;

    Saham_DB := Q1.FieldByName('Saham').AsString;
    Anbar_DB := Q1.FieldByName('Anbar').AsString;
    Basc_DB := Q1.FieldByName('Basc').AsString;

    if Length(Saham_DB)=0 then Saham_F := '';
```

* `Saham_DB` (`Dmu.pas:106`) — cross-database qualifier, `'Saham.Dbo'` or `''` if that database is
  absent on the server. Feature-detected at startup by `DB_ID()`.
* `Saham_F` (`Dmu.pas:107`) — UNC root of the scanned-document store, `\\pesteh\SahamData\`.
  Blanked whenever `Saham_DB` is blank.

Both are consumed **only** by `CardJariU.pas` (party-linkage aspect, §6.4):

```pascal
// CardJariU.pas:304-312
    if Length(DM.Saham_DB) > 0  then
    Begin
      Q1.Close;
      Q1.SQL.Clear;
      Q1.SQL.Add(' Select * From '+ DM.Saham_DB +'.NSaham ' );
      Q1.SQL.Add(' Where N_Card='+ inttostr(_Card) );
      Q1.Open;
      _IsInSaham := Q1.RecordCount>0;
    End;
```

```pascal
// CardJariU.pas:329-337
   S:= DM.Saham_F+ inttostr(_Card)+'\certificate_id.jpeg';
   if not FileExists(S) then
      S:= DM.Saham_F+ inttostr(_Card)+'\'+inttostr(_Card)+'_KartMelli.JPG';
   if FileExists(S) then
   Begin
     MyJPEG := TJPEGImage.Create;
     MyJPEG.LoadFromFile (S);
     S_AKS.Picture.Assign(MyJPEG);
    End;
```

`Saham.Dbo.NSaham` columns actually read: `N_Card`, `N_Name`, `N_Famil`, `N_Father`, `N_Mobile`,
`N_CodeMelli` (`CardJariU.pas:315-319`). **Read-only, display-only, join key = the same card
number.** No share counts, no percentages, no equity figures are read.

Section visibility toggles off entirely when the database is missing:

```pascal
// CardJariU.pas:237-244
    if Length( DM.Saham_DB)=0 then
    Begin
      G_Saham.Visible := False;
//      MessageDlg('  سیستم سهام نصب نشده است امکان اجرا بدون سیستم سهام نمی باشد', mtError, [mbok], 0);
//      Exit;
    End else begin
      G_Saham.Visible := true;
    end;
```

(commented-out message: `سیستم سهام نصب نشده است امکان اجرا بدون سیستم سهام نمی باشد`
= "The share system is not installed; running without it is not possible")

---


---

[← Previous](07-01-a-company-multi-tenancy-model.md) · [Index](00-index.md) · [Next →](07-02-a-counterparty-taraf-model.md)
