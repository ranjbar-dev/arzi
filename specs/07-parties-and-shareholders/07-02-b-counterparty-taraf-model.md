_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

### 2.3 Counterparty extended-attribute editor (`Sarfasl_TakmilU.pas`)

Caption fields: `نام سرفصل` (account name), address, registration number, national ID, economic code,
postal code, phone, fax. Reached from `SNewu.pas:600-615` on the currently selected **leaf** node.

```pascal
// SNewu.pas:600-615
procedure TSNew.BitBtn4Click(Sender: TObject);
var SSN:integer;
begin
    if Q1.Active=false then exit;
    if Q1.RecordCount=0 then exit;
    if Q1.FieldByName('SNo').AsInteger >0 then
    begin
      MessageDlg(' در سطح آخر حساب نیست', mterror, [mbok], 0);   // "Not a leaf-level account"
      exit;
    end;
    SSN := Q1.FieldByName('S_SSN').AsInteger ;
    Sarfasl_Takmil.init( SSN );
```

Load (`Sarfasl_TakmilU.pas:128-149`) and save (`:57-87`) are a straight 1-1 field mapping onto the
`Sarfasl` row via `TADOTable.Edit/Post`.

### 2.4 Auto-creation of the linked account

Yes — and it is the single most important integration point in this domain.

When a person or legal entity is saved (§3.4, §4.4), the editor iterates the `Coding` grid and calls
`Sarfasl_Add` for every ticked control account:

```pascal
// SahamdarEditU.pas:317-329   (identical shape at CompanyEditU.pas:288-300)
   Coding.First;
   for I := 1 to Coding.RecordCount do
   Begin
      if Coding.FieldValues['S_Found']=true then
      Begin
         // add
         Sarfasl_Add( Coding.FieldValues['S_Ko'],
                      Coding.FieldValues['S_Mo'],
                      SCard.IntValue, 0 ,
                      SName.Text+' '+ SFamil.Text+ '-'+ SFather.Text );
      End;
      Coding.Next;
   End;
```

```pascal
// SahamdarEditU.pas:333-341
procedure TSahamdarEdit.Sarfasl_Add(_Ko, _Mo, _Ta1, _Ta2: integer;
  _Name: String);
begin
   Q1.Close;
   Q1.SQL.Clear;
   Q1.SQL.Add('Exec Sarfasl_Add  '+ inttostr(_Ko)+ ', '+ inttostr(_Mo)+ ', '+
               inttostr(_Ta1)+ ', '+ inttostr(_Ta2)+ ', '+ QuotedStr(_Name) );
   Q1.ExecSQL;
end;
```

**Therefore the party's Tafsil-1 code IS its card number.** `_Ta1 := SCard.IntValue` always;
`_Ta2 := 0` always.

Generated account name:

| Editor | Formula | Example |
|---|---|---|
| Natural person (`SahamdarEditU.pas:326`) | `S_Name + ' ' + S_Famil + '-' + S_Father` | `علی رضایی-حسن` |
| Legal entity (`CompanyEditU.pas:297`) | `S_Name + ' ' + S_Famil` | `پسته سبز کرمان` |

> **Discrepancy to resolve before porting.** `SahamdarConfig` supports parking a party at Tafsil-2
> under a fixed Tafsil-1 (`SC_T > 0`), and both `Jari_Rem` (`Dmu.dfm:8672-8673`) and
> `CardJariU.dfm:6517-6518` implement that. But `Sarfasl_Add` is *always* called with the card at
> Tafsil-1 and `0` at Tafsil-2 (`SahamdarEditU.pas:325`, `CompanyEditU.pas:296`). Any config row
> with `SC_T > 0` will therefore have its detail account created in the wrong slot and will never
> resolve. §12-Q1.

### 2.5 Manual chart-of-accounts node creation (`SNewu.pas`)

Manual creation calls the same stored procedure through a `TADOStoredProc`:

```pascal
// SNewu.pas:617-653
procedure TSNew.B_AddClick(Sender: TObject);
var st:String;
    Code:integer;
begin
    ActiveControl := G1;
    St:='';
    Code := NextCode;
    if Not GetCodeName('ایجاد کد جدید', 'کد حساب', 'نام حساب', 6, 50, Code, St, 2)
    Then  Exit;

    Sp_ADD.Connection := DM.Ado;
    SP_Add.Close;
    SP_Add.Parameters.ParamByName('@Ko').Value := Kol;
    SP_Add.Parameters.ParamByName('@Mo').Value :=  Moein;
    SP_Add.Parameters.ParamByName('@Ta1').Value := Taf1;
    SP_Add.Parameters.ParamByName('@Ta2').Value :=  Taf2;
    SP_Add.Parameters.ParamByName('@Name').Value :=  ST;

    if State=1 then SP_Add.Parameters.ParamByName('@Ko').Value := Code;
    if State=2 then SP_Add.Parameters.ParamByName('@Mo').Value := Code;
    if State=3 then SP_Add.Parameters.ParamByName('@Ta1').Value := Code;
    if State=4 then SP_Add.Parameters.ParamByName('@Ta2').Value := Code;
    SP_Add.Active := True;
    if SP_Add.FieldByName('_Error').AsInteger>0 Then
    Begin
         MessageDlg( Sp_Add.FieldByName('_Desc').AsString  , mterror, [mbok] , 0);
    End Else Begin
         Q1.Close;
         Q1.Open;
    End;
```

`ایجاد کد جدید` = "Create new code"; `کد حساب` = "account code"; `نام حساب` = "account name";
max lengths 6 (code) and 50 (name).

`Sarfasl_ADD` is therefore a **result-set-returning** procedure exposing `_Error` (int) and `_Desc`
(nvarchar). The procedure body is not in the repo — see §9.

Other `SNewu.pas` guards on chart nodes (all Persian → English):

| Persian | English | Line |
|---|---|---|
| `این کد زیر شاخه دارد و قابل حذف نیست` | "This code has children and cannot be deleted" | 169 |
| `بر روی این کد سند صادر شده است و قابل حذف نیست` | "Vouchers exist for this code; it cannot be deleted" | 175 |
| `بر روی این کد سند صادر شده است و قابلیت افزودن زیر شاخه ندارد` | "Vouchers exist for this code; children cannot be added" | 214, 230 |
| `این کد زیر شاخه دارد و قابل تغییر نیست` | "This code has children and cannot be renumbered" | 251 |
| `بر روی این کد سند صادر شده است و قابل تغییر نیست` | "Vouchers exist for this code; it cannot be renumbered" | 257 |
| `کد داده شده تکراری است و تغییر غیر قابل اجرا است` | "The given code already exists; the change cannot be applied" | 268 |
| `در سطح آخر حساب نیست` | "Not a leaf-level account" | 608 |
| `در سطح کل مورد تایید نمیباشد` | "Not permitted at Kol level" | 700 |
| `سطح آخر حساب نیست` | "Not a leaf-level account" | 705 |
| `قبلا در لیست ثبت شده است` | "Already registered in the list" | 726 |

### 2.6 Node-level locking

```pascal
// Dmu.pas:921-966  (abridged — the same block repeats for Kol, Moein, Ta1, Ta2)
function TDM.Is_Admin_Or_Valid_Daftar(_Ko, _Mo, _Ta1, _Ta2: integer): Boolean;
begin
   Result := Admin;
   if Dm.Admin then Exit;
   ...
   Q1.SQL.Add(' Select * From sarfasl Where S_Ko='+ inttostr(_Ko) +
                ' and S_Mo=0 and S_ta1=0 and S_Ta2=0 ');
   Q1.Open;
   if Q1.RecordCount=0 then Begin Result:=True; exit; end;
   if Q1.FieldByName('S_Lock').AsInteger=1 then Begin Result :=false; Exit; end;
   if _Mo=0 then begin result := True; exit; end;
   ... (repeat for Moein, Ta1, Ta2) ...
   result := True;
end;
```

Semantics: an admin bypasses everything; otherwise the node is blocked if **any ancestor or itself**
carries `S_Lock = 1`; a missing ancestor is treated as unlocked (`Result := True; exit`).

### 2.7 Code formatting for display

```pascal
// Dmu.pas:1180-1229
function TDM.Sarfasl_SSN_CODEName(SSN: integer): String;
...
// Kol
    L  := Base.FieldByName('NO_Ko').AsInteger;
    S1 := '00000000'+Sarfasl.FieldByName('S_Ko').asstring ; S1:= Copy( S1, Length(S1)-L+1 , L );
    S :=  S1;
// Moein
    if Sarfasl.FieldByName('S_Mo').AsInteger > 0 Then
    Begin
       L  := Base.FieldByName('NO_Mo').AsInteger;
       S1 := '00000000'+Sarfasl.FieldByName('S_Mo').asstring ; S1:= Copy( S1, Length(S1)-L+1 , L );
       S := S1 + '-'+ S;
    End;
...
    S := S +' ' + Trim( Sarfasl.FieldByName('S_Name').asstring );
```

Each segment is left-zero-padded to the width configured on the **active fiscal year**
(`No_Ko`, `No_Mo`, `No_Ta1`, `No_Ta2`) and segments are prepended, producing
`Ta2-Ta1-Mo-Ko <name>` — i.e. **deepest segment first**, RTL reading order. Unknown SSN yields the
literal `'! Unkhnown !'` (sic, `Dmu.pas:1186`, `:1192`).

Contrast `TarafU.Get_FullCode` (`TarafU.pas:104-113`), which produces the **opposite** order
`Ko-Mo-Ta1-Ta2` with no padding, and `Dm.Split_Code` (`Dmu.pas:510-545`), which parses that order.
Two incompatible code-string conventions coexist. §12-Q9.

---


---

[← Previous](07-02-a-counterparty-taraf-model.md) · [Index](00-index.md) · [Next →](07-03-counterparty-person-crud-validations.md)
