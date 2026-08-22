_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

### 6.4 Party resolution in `CardJariU` (party-linkage aspects only)

> Reporting internals of this unit are owned by another agent. What follows is strictly the
> party-identity/lookup path.

`CardJariF.init(_Jari, _Coid)` (`CardJariU.pas:233-269`) opens the form for a given card and fiscal
year. `_Coid = 0` means "use the ambient year":

```pascal
// CardJariU.pas:263-265
    if _Coid=0 then COID.KeyValue := DM.CO_ID
    else coid.KeyValue := _Coid;
    if _jari>0 then S_CardExit(Self);
```

`LoadSahamdar` (`CardJariU.pas:271-376`) is the resolution routine:

1. `ClearForm` (`:278`, body at `:171-190`).
2. Look up the person record:
   ```pascal
   // CardJariU.pas:283-300
       Q1.SQL.Add('Select * From Sahamdar where S_card='+ inttostr(_Card) );
       Q1.Open;
       _IsInHesab := Q1.RecordCount>0;
       if _IsInHesab then
       Begin
         S_name.Text := Q1.FieldByName('S_Name').AsString;
         S_Famil.Text := Q1.FieldByName('S_Famil').AsString;
         S_Father.Text := Q1.FieldByName('S_Father').AsString;
         S_Tel.Text := Q1.FieldByName('S_Mobile').AsString;
         S_CMelli.Text := Q1.FieldByName('S_CodeMelli').AsString;
       End Else Begin
         N_Name.Text := 'جاری در قسمت اشخاص وارد نشده است';
       End;
   ```
   `جاری در قسمت اشخاص وارد نشده است` = "This current account has not been entered in the Persons
   section".
3. Look up the external share register (§1.7). Failure messages:
   `جاری در برنامه سهام به روز نشده` = "This current account is not up to date in the share
   program" (`CardJariU.pas:321`); `بروزرسانی انجام شود` = "Please run an update"
   (`CardJariU.pas:322`).
4. Load the scan (`CardJariU.pas:329-337`).
5. Lock check (`CardJariU.pas:342-347`, §4.4).
6. Balance (`CardJariU.pas:350-360`, §6.2).
7. Enumerate the party's accounts via `QList` and push each into the grid via `ADD_Code`
   (`CardJariU.pas:362-372`).

The card can also be chosen from the register:

```pascal
// CardJariU.pas:434-440
procedure TCardJariF.sSpeedButton1Click(Sender: TObject);
begin
    Sahamdar.init2;
    if Sahamdar.Tag=0 then exit;
    S_Card.IntValue := Sahamdar.Tag;
    S_CardExit(Self);
end;
```
(`init2` is the picker mode of the register — §10.3.)

**`QList` — the party's account set, verbatim** (`CardJariU.dfm:6503-6527`):

```sql
Declare @Jari int Set @Jari=:Jari

   if OBJECT_ID('tempdb..#R') is not null Drop Table #R

   Select  -Min(SC_Rem) As SC_Rem , SC_K, SC_M, SC_T as SC_T1, 0 As SC_T2, 0 as SC_Found
   into #R
   From SahamdarConfig
   Group By SC_K, SC_M, SC_T

   Update #R Set SC_T2=@Jari Where SC_T2=0 and SC_T1>0
   Update #R Set SC_T1=@Jari Where SC_T1=0

   Update #R Set SC_Found = ( Select Count(*) From Sarfasl Where SC_K=S_Ko and SC_M=S_Mo and SC_T1=S_Ta1 and SC_T2=S_Ta2 )
   Delete #R Where SC_found=0

   Select * From #R Order By SC_Rem, SC_K, SC_M, SC_T1, SC_T2
```

Differences from `Jari_Rem` that matter:

* **No `SC_Rem = 1` filter** — *every* configured control account is listed, not only the ones that
  count toward the balance.
* `-Min(SC_Rem)` is a sort key: rows with `SC_Rem = 1` become `−1` and therefore sort **first**.
  Balance-bearing accounts appear at the top of the grid.
* `Group By SC_K, SC_M, SC_T` collapses duplicate config rows.
* `SC_Found` filters to accounts that **actually exist** in `Sarfasl` for this card.

Then, per returned row:

```pascal
// CardJariU.pas:367-372
   for I := 1 to QList.RecordCount do
   Begin
     ADD_Code( QList.FieldByName('SC_K').AsInteger, QList.FieldByName('SC_M').AsInteger,
               QList.FieldByName('SC_T1').AsInteger,QList.FieldByName('SC_T2').AsInteger );
     Qlist.Next;
   End;
```

`ADD_Code` (`CardJariU.pas:124-164`) resolves the node in `Sarfasl`, then per-account debit/credit:

```sql
-- CardJariU.pas:147-150
 Select isnull(Sum(M_Bed),0) As Bed, isnull(Sum(M_Bes),0) As Bes  From moein
  Where M_ko=<K> and M_Mo=<M> and M_Ta1=<T1> and M_ta2=<T2> and M_COID=<COID.KeyValue>
```
and the per-row net split:
```pascal
// CardJariU.pas:157-160
    Vt1.FieldValues['R_Bed'] := 0;
    Vt1.FieldValues['R_Bes'] := 0;
    if _Bed>_Bes then Vt1.FieldValues['R_Bed'] := _Bed-_Bes;
    if _Bes>_Bed then Vt1.FieldValues['R_Bes'] := _Bes-_Bed;
```
i.e. a one-sided residual balance per account (never both). Note this per-row view is
**debit-positive-in-`R_Bed`**, while the aggregate `Jari_Rem` is credit-positive — opposite
conventions in the same screen. §12-Q14.

### 6.5 Deposits, withdrawals, interest

* **Deposits and withdrawals are ordinary vouchers.** They are created by the treasury modules
  (`FISHDaryaftU`, `CheckDaryaftU`, `CheckEditU`, `Asnad_Daryaft_NewU`, …), which post `Moein` rows
  against the party's detail account. Nothing in this domain creates them.
* **There is no interest or profit-crediting logic.** No rate field, no accrual routine, no
  day-count convention exists anywhere (see §5.1). Any interest credited to a party today is a
  hand-entered voucher.

### 6.6 Party bank accounts — `SahamdarInfo`

`SahamdarInfoU.pas` / `.dfm`, caption `حسابهای بانکی اشخاص` = "Bank accounts of persons"
(`SahamdarInfoU.dfm:5`).

Query (identical in `Q1` and `QS`, `SahamdarInfoU.dfm:271-278` and `:303-310`):

```sql
IF OBJECT_ID('tempdb..#R') IS NOT NULL  DROP TABLE #R

Select * into #R
   From SahamdarInfo Where SI_Card= :Card and SI_ID=1

Select * From #R Order By SI_SSN
```

| Column | Grid caption | English | Evidence |
|---|---|---|---|
| `SI_SSN` | *(sort only)* | Surrogate key | `SahamdarInfoU.dfm:278` |
| `SI_Card` | *(filter)* | FK → `Sahamdar.S_Card` | `SahamdarInfoU.dfm:276` |
| `SI_ID` | *(filter, `=1`)* | Record type discriminator — only type 1 is ever read | `SahamdarInfoU.dfm:276` |
| `SI_St1` | `شماره کارت-شبا-حساب` | Card / IBAN / account number | `SahamdarInfoU.dfm:128` |
| `SI_St2` | `صاحب حساب` | Account holder | `SahamdarInfoU.dfm:135` |
| `SI_St3` | `نام بانک` | Bank name | `SahamdarInfoU.dfm:142` |
| `SI_St4` | `توضیحات` | Notes | `SahamdarInfoU.dfm:149` |

Selection contract (`SahamdarInfoU.pas:35-39`, `:120-130`):

```pascal
procedure TSahamdarInfo.B_SelectClick(Sender: TObject);
begin
   if Q1.Active=false  then exit;
   if Q1.RecordCount=0 then exit;
   Is_Select:=True;
   Get_St1:=Q1.FieldByName('SI_St1').AsString;
   Get_St2:=Q1.FieldByName('SI_St2').AsString;
   Get_St3:=Q1.FieldByName('SI_St3').AsString;
   Get_St4:=Q1.FieldByName('SI_St4').AsString;
   Close;
end;
```

Consumer (cheque entry autofills bank + account holder from the selected party's bank account):

```pascal
// CheckEditAddU.pas:142-149
procedure TCheckEditAddF.SB2Click(Sender: TObject);
begin
   SahamdarInfo.init_CodeStr(CD_Code.Text);
   if SahamdarInfo.Is_Select=false then exit;
   CD_BankNo.Text :=  SahamdarInfo.Get_St1;
   CD_Jari.Text := SahamdarInfo.Get_St2;
   ActiveControl := CD_Mab;
end;
```

> `SahamdarInfo` is **read-only in this application**. Its New / Edit / Delete buttons
> (`SahamdarInfoU.dfm:162-191`, `:238-253`) have **no `OnClick` handlers**. Rows must be maintained
> elsewhere (probably the external Saham product). §12-Q15.

---


---

[← Previous](07-06-a-party-current-account-jari.md) · [Index](00-index.md) · [Next →](07-07-sahamdarconfig-party-account-configuration.md)
