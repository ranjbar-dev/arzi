_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 4. Person / legal-entity ("Sahamdar") model

### 4.1 The table

`Sahamdar` — a `TADOTable` on the shared connection, no year scope (`Dmu.dfm:561-567`,
`Dmu.pas:36`).

| Column | Type (inferred from `Sahamdar_Edit` params, `Dmu.dfm:173-297`) | Persian label | English | Evidence |
|---|---|---|---|---|
| `S_Card` | int, **business key** | `ش.شناسايي` | Card / identification number | `SahamdarEditU.dfm:171`; `SahamdarU.dfm:68` |
| `S_Kind` | tinyint | — | `1` = natural person (`اشخاص`), `2` = legal entity (`شرکتها`) | `SahamdarEditU.pas:291`; `CompanyEditU.pas:263`; `SahamdarU.dfm:465` |
| `S_Name` | varchar(50) | `نام` / `نام شخصیت` | Given name / entity name | `SahamdarEditU.dfm:160`; `CompanyEditU.dfm:90` |
| `S_Famil` | varchar(50) | `نام خانوادگي` / `نام مدير يا نماينده` | Surname / representative | `SahamdarEditU.dfm:149`; `CompanyEditU.dfm:99` |
| `S_Father` | varchar(50) | `نام پدر` | Father's name (persons only) | `SahamdarEditU.dfm:138` |
| `S_IDNO` | int | `ش.شناسنامه` | Birth-certificate / ID document number | `SahamdarEditU.dfm:171` label at `:127` |
| `S_BDate` | varchar(8/10), Jalali | `تاريخ تولد` / `تاريخ تاسيس` | Birth date / incorporation date | `SahamdarEditU.dfm:94`; `CompanyEditU.dfm:27` |
| `S_BPlace` | varchar(20) | `محل تولد` / `محل تاسيس` | Birth place / place of incorporation | `SahamdarEditU.dfm:83`; `CompanyEditU.dfm:36` |
| `S_SDate` | varchar(8/10), Jalali | `تاريخ صدور` | ID issue date (persons only) | `SahamdarEditU.dfm:72` |
| `S_SPlace` | varchar(20) | `محل صدور` | ID issue place (persons only) | `SahamdarEditU.dfm:61` |
| `S_CodeMelli` | varchar(10) | `کد ملي` / `شناسه ملي` | National ID (person) / legal-entity national ID | `SahamdarEditU.dfm:50`; `CompanyEditU.dfm:45` |
| `S_CodePosti` | varchar(10) | `کدپستي` | Postal code | `SahamdarEditU.dfm:28` |
| `S_CodeSabt` | varchar(12) | `کد ثبت` | Registration code | `SahamdarEditU.dfm:39,424` |
| `S_Address` | varchar(100) | `آدرس` | Address | `SahamdarEditU.dfm:105` |
| `S_Mobile` | varchar(12) | `موبايل` | Mobile number | `SahamdarEditU.dfm:116` |
| `S_Phone` | varchar(12) | `تلفن` | Landline — in `Sahamdar_Edit` and `SahamdarP` only, **absent from both live editors** | `Dmu.dfm:284-290`; `SahamdarP.pas:147` |
| `S_Siba` | varchar(13) | `سيبا` | SIBA (Bank Melli) account number — **legacy, unused by live forms** | `Dmu.dfm:291-297`; `SahamdarP.pas:148` |
| `S_Shanas` | varchar | — | **dead** (only inside a commented-out block) | `SahamdarP.pas:98,113` |
| `S_MaliatState` | int (0..4) | `وضعيت مالياتي` | Tax status (enum, §4.2) | `SahamdarEditU.pas:159,309` |
| `S_Lock` | int (0/1) | — | Current-account lock | `SahamdarU.pas:85-92`; `Dmu.pas:979` |

### 4.2 `S_MaliatState` — tax status enum

`TComboBox` with `Style = csDropDownList`, index stored directly
(`SahamdarEditU.pas:159` read, `:309` write; identical in `CompanyEditU.pas:147,280`).

| Index | Persian (`SahamdarEditU.dfm:436-441`) | English |
|---|---|---|
| 0 | *(empty)* | Not specified |
| 1 | `مودی مشمول ثبت نام در نظام مالیاتی` | Taxpayer required to register in the tax system |
| 2 | `مشمولین حقیقی ماده 81` | Natural persons covered by Article 81 |
| 3 | `اشخاصی که مشمول ثبت نام در نظام مالیاتی نیستند` | Persons not required to register in the tax system |
| 4 | `مصرف کننده نهایی` | Final consumer |

### 4.3 Relationship between a person and a counterparty/account

There are **two** links, and they are not kept consistent by the code:

**Link 1 — positional, by construction.** For every ticked `SahamdarConfig` row, a `Sarfasl` node is
created with `S_Ta1 = S_Card` under `(SC_K, SC_M)` (§2.4). This is the *primary* link and the one all
balance queries use:

```sql
-- Dmu.dfm:8632-8634  (SahamdarConfig existence probe)
Select * From Sarfasl Where Sarfasl.S_Ko=SahamdarConfig.SC_K
  and Sarfasl.S_Mo=SahamdarConfig.SC_M
  and Sarfasl.S_Ta1=@Card and Sarfasl.S_Ta2=0
```

**Link 2 — explicit FK `Sarfasl.S_Card`.** Written by exactly one place in the whole project, a
manual "attach this account to a person by national ID" tool:

```pascal
// ListSarfaslu.pas:295-322
    S := '';
    if not GetString('Input','کدملي', 10 , S ) Then Exit;
    Dm.Sahamdar.Close;
    Dm.Sahamdar.Open;
    if Not Dm.Sahamdar.Locate('S_CodeMelli', String(S) , [LoCaseInsensitive]) Then
    Begin
       Application.MessageBox('پيدا نشد','Error');           // "Not found"
       ActiveControl := G1;
       Exit;
    End;
    I := Sp1.fieldByName('S_SSN').AsInteger;
    J := Dm.Sahamdar.FieldByName('S_Card').AsInteger;
    Dm.Sarfasl.Close;
    Dm.Sarfasl.Open;
    Dm.Sarfasl.Locate('S_SSN' , inttostr(i) , [LoCaseInsensitive]);
    Dm.Sarfasl.Edit;
    Dm.Sarfasl.FieldByName('S_Card').AsInteger := J;
    Dm.Sarfasl.Post;
```
(Persian literals decoded from CP-1256; success message at `:325` is `ذخيره انجام شد` = "Saved".)

Reverse resolution (account → person contact string) uses **Link 1 only**:

```pascal
// Dmu.pas:1385-1441  (abridged)
function TDM.Get_Jari_Code(SSN: integer): String;
...
   K  := Q2.FieldByName('S_Ko').AsInteger;
   M  := Q2.FieldByName('S_Mo').AsInteger;
   T1 := Q2.FieldByName('S_Ta1').AsInteger;
   T2 := Q2.FieldByName('S_Ta2').AsInteger;

   if (Jari=0) and (T2>0) then Begin Jari:=T2; T2:=0; End;
   if (Jari=0) and (T1>0) then Begin Jari:=T1; T1:=0; End;
   if (Jari=0) and (M>0)  then Begin Jari:=M;  M:=0;  End;
   ...
   Q2.SQL.Add( 'Select * From SahamdarConfig Where SC_K='+inttostr(K)+ ' and SC_M='+ inttostr(M)
               + ' and SC_T='+ inttostr(T1) );
   Q2.Open;
   if Q2.RecordCount=0 then Begin Q2.Close; Result :=''; Exit; End;
   ...
   Q2.SQL.Add(' Select * From Sahamdar Where S_Card='+ inttostr(Jari) );
   ...
   if Length(Trim(Q2.FieldByName('S_CodeMelli').AsString)) > 0  then
      S:= S + '  کد ملی : ' + Trim(Q2.FieldByName('S_CodeMelli').AsString) ;
   if Length(Trim(Q2.FieldByName('S_CodePosti').AsString)) > 0  then
      S:= S + '  کد پستی : ' + Trim(Q2.FieldByName('S_CodePosti').AsString) ;
   if Length(Trim(Q2.FieldByName('S_Mobile').AsString)) > 0  then
      S:= S + '  شماره تماس : ' + Trim(Q2.FieldByName('S_Mobile').AsString) ;
   Result := S;
```

**The card-extraction rule, stated precisely** (`Dmu.pas:1407-1409`, mirrored at
`SahamdarInfoU.pas:94-104`): take the *deepest non-zero* of `Ta2`, `Ta1`, `Mo` as the card number and
zero it out; the remainder identifies the control account. So a party may live at Tafsil-2, at
Tafsil-1, or (theoretically) at Moein level.

The same rule in the bank-account picker:

```pascal
// SahamdarInfoU.pas:88-112
procedure TSahamdarInfo.init_CodeStr(_Code: string);
var K,M,T1,T2, _Card :integer;
begin
   Is_Select:=False;
   Dm.Split_Code(_Code, K,M,T1,T2);
   _Card:=0;
   if T2>0 then
   Begin
      _Card:=T2;
      T2:=0;
   End;
   if (_Card=0) and (T1>0)  then
   Begin
     _Card:=T1;
     T1:=0;
   End;
   if _Card=0 then Exit;
   QS.Close;
   Qs.ConnectionString:=Dm.Ado.ConnectionString;
   Qs.SQL.Clear;
   Qs.SQL.Add(Format( ' Select * From SahamdarConfig Where SC_K=%d and SC_M=%d and SC_T=%d' , [K,M,T1])  );
   Qs.Open;
   if Qs.RecordCount=0 then Exit;
   Init( _Card );
end;
```


---

[← Previous](07-03-counterparty-person-crud-validations.md) · [Index](00-index.md) · [Next →](07-04-b-person-legal-entity-sahamdar-model.md)
