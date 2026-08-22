_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 3.7 Voucher numbering

**Per fiscal year, user-assignable, gaps allowed, no reservation.**

`Dm.New_Sanad` (`Dmu.pas:1242-1251`) proposes the next number:

```sql
-- Dmu.pas:1247
  Select isnull(Max( M_sanad ),0 )+1 as NewSanad From moein Where M_COid=<CO_ID>
```

Notes:
- Scoped by `M_COid` only — **numbering restarts every fiscal year**.
- Reads `Moein` (the lines), not `DMoein`. A header with no lines is invisible to the counter.
- **Not atomic.** Two concurrent users get the same number; the duplicate check at save time is the
  only protection.
- The number is a plain editable field (`S_Sanad: TEditInt`). Users routinely override it — every
  screen re-validates for duplicates.

An alternative for journal vouchers reads `DMoein` instead (`MoeinToRU.pas:264`):

```sql
  Select isnull(Max(DM_Sanad) , 0)+1 As N From DMoein Where DM_Coid=<CO_ID>
```

**Note the two counters are not synchronised**, and journal vouchers (`DM_Kind=2`) share the same
number space as Moein vouchers (`DM_Kind=1`).

#### Date-based number reuse for generated vouchers

`Dm.Get_NewSanad_DateID(F_Date, IDList)` (`Dmu.pas:1461-1477`) is the numbering rule for
system-generated vouchers. It **groups all documents of the same day and the same source class onto
one voucher**:

```pascal
function TDM.Get_NewSanad_DateID( F_Date:String; IDList:String ): Integer;
begin
    Result := New_Sanad;                               // default: next free number
    Dm.Q1.SQL.Add(' Select isnull( Max(M_Sanad) , 0 ) as S  ');
    Dm.Q1.SQL.Add('   From Moein ');
    Dm.Q1.SQL.Add('   Where M_Tx=0 and M_Coid='+ inttostr(Dm.CO_ID) );
    Dm.Q1.SQL.Add('   and M_ID in( ' + IDList+ ')' );
    Dm.Q1.SQL.Add('   and M_Date='+ QuotedStr(F_Date) );
    Dm.Q1.Open;
    if Dm.Q1.FieldByName('S').AsInteger >0  then
            Result := Dm.Q1.FieldByName('S').AsInteger; // reuse the existing draft voucher
end;
```

Called with `IDList = '31,32,33,34,35,36,37,38,39'` at `MakeSanadU.pas:314`, `:449`, `:583`, `:716`.
Effect: all inventory documents posted for a given Jalali date land on the same voucher, **provided
that voucher is still in draft (`M_Tx = 0`)**. Once approved, the next document opens a new voucher.

The companion validator `Dm.Get_SanadDateID_Valid` (`Dmu.pas:1494-1521`) is defined but has no
callers in this repository.

### 3.8 Fiscal year gate

Almost every mutating accounting action is preceded by:

```pascal
    if DM.Is_New_Sanad_Valid( Dm.CO_ID)=False Then Exit;
```

`Dm.Is_New_Sanad_Valid` (`Dmu.pas:997-1015`):

```pascal
   if Base.Locate('CO_ID', Coid, [loCaseInsensitive]) = false then
     MessageDlg('   سال مالی پیدا نشد  ', mterror, [mbok], 0);       // "fiscal year not found"
   if Base.FieldByName('IsActive').asinteger <> 1  then
     MessageDlg('          سال مالی مورد نظر بایگانی شده است                '+#13#10+
                '   اجازه تغییر در این سال و صدور فاکتور و سند را ندارید    ', mterror, [mbok], 0);
```

Second message: "The selected fiscal year has been archived / You are not permitted to make changes
in this year or to issue invoices and vouchers."

`Base.IsActive` (0/1) is the **year-archived flag**. It is read here and **nowhere else**; no screen
in this repository writes it. See §14.

Call sites: `SanadViewU.pas:154, 180, 200, 326, 382, 438, 513, 661, 707`; `Mainu.pas:678, 690, 1003`;
`SodoorSanadU.pas:145, 172, 216`.

Fiscal-year date range: `Base.FromDate` / `Base.ToDate` (Jalali strings). Accessors
`Dm.From_Date` (`Dmu.pas:1137-1142`) and `Dm.To_Date` (`Dmu.pas:1144-1149`).

### 3.9 Jalali date handling

Dates are stored as **`varchar(10)` Jalali strings in `YYYY/MM/DD` format** and compared
lexicographically (which is correct for zero-padded ISO-like Jalali).

`Dm.IsDate` (`Dmu.pas:883-900`) — syntactic validation:

```pascal
   if (Length(d1)<>8) and (Length(D1)<>10 ) Then Exit;   // accepts YY/MM/DD too
   if length(d1)=8 then d1 := '13'+D1;                   // 2-digit year => 13xx
   if (Copy(D1,5,1)<>'/') or (Copy(D1,8,1)<>'/') Then Exit;
   ...
   if (M>12) or (M<1) then Exit;
   if (R>31) or (R<1) then Exit;
   if (M>6) and (R>30) then Exit;     // months 7..12 have at most 30 days
   if (S>1420) or (S<1300) then Exit; // year must be 1300..1420
```

Note: does **not** validate Esfand (month 12) against 29/30 days, nor leap years. A date of
`1403/12/30` in a non-leap year passes.

`Dm.isValidDate` (`Dmu.pas:911-919`) adds the fiscal-year range check:

```pascal
    if Not IsDate(D1) then Exit;
    if Length(D1)=8 then D1 := '13'+D1;
    if D1 < From_Date then Exit;
    if D1 > To_Date then Exit;
```

`Dm.MiladiToShamsi` (`Dmu.pas:362-437`) converts Gregorian → Jalali; used to seed date fields with
"today".

**Rebuild note:** store a real `DATE` plus a generated Jalali string, or store the Jalali components.
Do not carry `varchar` dates forward. Preserve the lexicographic-comparison semantics in the
migration, and validate Esfand properly.

---

_Prev: [03-03-b-voucher-sanad-model](03-03-b-voucher-sanad-model.md) | Next: [03-04-voucher-validation-rules](03-04-voucher-validation-rules.md)_
