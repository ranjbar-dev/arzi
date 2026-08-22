_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 6. Date handling

### 6.1 Summary

Every **business** date in arzi is a **Jalali (Shamsi/Persian) calendar date stored as a
fixed-width ASCII string**, never as a SQL `date`/`datetime`. The canonical form is

```
YYYY/MM/DD      e.g. '1399/05/27'      char/varchar(10), zero-padded
```

A legacy short form `YY/MM/DD` (8 characters, century implicit `13`) is still accepted on input and
still leaks into the database and into comparisons. The mixture of the two widths is the single
most dangerous thing in the date layer (§6.5).

The only true `datetime` values in the schema are **server-side audit stamps** written with SQL
Server's `GetDate()` — they are Gregorian, they are never shown to the user, and they are never
compared against a business date:

| Column | Table | Written at |
|---|---|---|
| `DM_CDate` | `DMoein` (voucher header) | `Dmu.pas:831`, `Dmu.pas:835` |
| `M_CDate` (last positional arg `GetDate()`) | `Moein` (voucher line) | `MakeSanadU.pas:93`, `SanadEditU.pas:630`, `CheckEditU.pas:486`, `TankhahEdit.pas:463`, `FactorPesteh_U.pas:224,226` |
| `AJ_DateTime` | `Anbar_Jens` (item master) | `AnbarCalaAddU.pas:170,175` |

### 6.2 Where "today" comes from

**Not from the client clock.** `TDM.Current_Date` (`Dmu.pas:1232-1239`) calls the server-side
stored procedure `XNew`:

```pascal
function TDM.Current_Date: String;
begin
   DM.XNEW.Close;
   DM.XNEW.Parameters.ParamByName('@COID').Value := DM.CO_ID;
   DM.XNEW.Open;
   Result := Dm.XNEW.FieldByName('CurrentDate').AsString;
   DM.XNEW.Close;
end;
```

`XNew` is declared at `Dmu.dfm:418-437` with one input parameter `@COID integer` and a return
value; it yields a result set containing at least a column `CurrentDate`. `Current_Date` is the
**only** consumer of `XNew` in the entire codebase (grep: `Dmu.pas:1234-1238` are the sole hits).

**Consequence: the live Gregorian→Jalali conversion happens inside a stored procedure whose body is
not in this repository.** It must be scripted out of a live database before the rebuild — see §3
and §12. Everything in §6.3 below is compiled into the executable but *not called*.

`Current_Date` seeds the date editor when no date is supplied (`GetD.pas:38-39`).

### 6.3 The two Jalali algorithms in the Delphi source

Both are present, both compile, and **neither has a single call site** anywhere in the project
(repo-wide grep for `FarsiDate` returns only `Utility.pas:47,435`; for `MiladiToShamsi` only
`Dmu.pas:140,362`). They are documented here because (a) they show what the author believed the
calendar rules to be, and (b) one of them is almost certainly the ancestor of the T-SQL inside
`XNew`, so the rebuild team must be able to recognise its output.

#### 6.3.1 Algorithm A — `TUtil.DecodedateF` / `TUtil.FarsiDate` (`Utility.pas:413-442`)

```pascal
Procedure TUtil.DecodedateF( Adate:TDateTime;var Ayear,Amonth,Aday:Word);
var R,I :integer;
Begin
  Ayear:=1279; Amonth:=1;Aday:=1;
  R := (Round(int(real(ADate))))-80;
  for i:=1 to 1000 Do Begin
     if I mod 4 = 0 Then Dec( R , 366 ) Else Dec( R , 365 );
     inc( AYear );
     if R < 365 Then Break;
  End;
  for I:=1 to 6 do
     if R > 31 then Begin inc(AMonth) ; Dec(R,31); End else Begin Aday:=R; Exit; end;
  for I:=1 to 6 do
     if R > 30 then Begin inc(AMonth) ; Dec(R,30); End else Begin Aday:=R; Exit; end;
End;

Function TUtil.FarsiDate( Adate:TdateTime):String;
Begin
  DecodeDateF( ADate , Ayear, Amonth, Aday);
  Result := inttostr(Ayear mod 100 )+'/' +
  Char(Amonth div 10 +48)+Char(Amonth mod 10 +48)+'/'+
  Char(ADay div 10 +48)+Char(ADay mod 10 +48);
End;
```

Mechanics and defects, precisely:

1. **Epoch.** `R := trunc(TDateTime) - 80`. Delphi day 0 is 1899-12-30, so `R = 1` corresponds to
   Gregorian 1900-03-21, i.e. Jalali 1279/01/01. `AYear` is seeded at 1279.
2. **Leap rule is keyed off the loop counter, not the year.** `if I mod 4 = 0` gives 366 days to the
   4th, 8th, 12th … iteration of the loop. Because `Inc(AYear)` happens *after* the subtraction, the
   366-day years are the ones whose iteration index is a multiple of 4 — a fixed phase chosen by the
   epoch, unrelated to the actual Jalali leap sequence. The real Jalali leap years follow a 33-year
   cycle (leap when `year mod 33 ∈ {1,5,9,13,17,22,26,30}`), commonly approximated by
   `(year+11) mod 33 < 8` or, crudely, `year mod 4 = 3` (1375, 1379, …, 1399, 1403 are leap). The
   phase produced by this loop does **not** match either. **This is arithmetically wrong** and drifts
   by ±1 day for large parts of any year, growing with distance from the epoch.
3. **Off-by-one at the year boundary.** The loop breaks *after* subtracting, on `R < 365`. `R` can
   therefore land on `0`, which the month decomposition turns into `AMonth = 1, ADay = 0` —
   an impossible `xx/01/00`.
4. **Negative `R` for dates before the epoch** falls straight through both month loops into
   `Aday := R` where `Aday` is a `Word`; a negative value wraps to a huge unsigned number.
5. **Month lengths.** Six 31-day months then six 30-day months. Esfand (month 12) is therefore
   always 30 days — Esfand 29 in a common year is unrepresentable — and because the second loop can
   run six times starting from month 7, `AMonth` can reach **13**.
6. **Two-digit year, not zero-padded.** `inttostr(AYear mod 100)`. For Jalali years 1400–1409 this
   yields `'0'`…`'9'` — a **one**-character year — producing strings like `'0/01/01'` (7 chars) or
   `'3/05/27'`. Every downstream assumption of an 8-character `YY/MM/DD` breaks, including
   `TUtil.IsFarsiDate` (`Utility.pas:526-541`), which begins `if Length(D)<>8 ... Exit(False)`.
   **This is a hard Y1400 (Gregorian 2021) failure** in this code path.

#### 6.3.2 Algorithm B — `TDM.MiladiToShamsi` (`Dmu.pas:362-437`)

Standard day-of-year algorithm: compute the Gregorian day-of-year (with a correct
`IsLeapYear(Year)` adjustment for `month > 2`), then

- if `day_year <= 79` (1 Jan – 20 Mar): add `11` when `(Year - 1) mod 4 = 0` else `10`,
  `Year := Year - 622`, and split into 30-day blocks yielding months 10, 11, 12 (`Dmu.pas:386-402`);
- otherwise `Year := Year - 621`, `day_year := day_year - 79`, then months 1–6 as 31-day blocks
  (`day_year <= 186`) and months 7–12 as 30-day blocks (`Dmu.pas:403-427`).

Output is **`YYYY/MM/DD`, four-digit year, zero-padded** (`Dmu.pas:430-434`) — i.e. the canonical
storage format, unlike Algorithm A.

This is the near-correct one. Residual defects:

1. The Jalali leap year is inferred from `(Year - 1) mod 4 = 0` in the 1 Jan – 20 Mar window only.
   That approximation is right for the whole practical range of this product but fails at Gregorian
   century boundaries: for `Year = 2101`, `(2101-1) mod 4 = 0` so it adds 11, but 2100 is not a
   Gregorian leap year. Irrelevant before 2101; record it so nobody re-derives it.
2. There is **no inverse function anywhere in the codebase.** Repo-wide grep finds no
   `ShamsiToMiladi`, no Jalali→Gregorian conversion of any kind. A stored Jalali date can never be
   turned back into a real date by the application. Every piece of date arithmetic, ordering and
   range filtering in arzi is therefore **string manipulation** (§6.5).

#### 6.3.3 Discrepancies between A and B, side by side

| Property | A — `FarsiDate` (`Utility.pas:435`) | B — `MiladiToShamsi` (`Dmu.pas:362`) |
|---|---|---|
| Output format | `Y/MM/DD` or `YY/MM/DD` (7 or 8 chars) | `YYYY/MM/DD` (10 chars, always) |
| Year digits | `AYear mod 100`, **not** zero-padded | full 4-digit, from `Year - 621/622` |
| Leap-year source | loop-counter phase `I mod 4 = 0` — **wrong** | Gregorian `IsLeapYear` + `(Year-1) mod 4` — right in range |
| Esfand length | always 30 | 30 only in the `day_year <= 79` branch, consistent with real leap years |
| Month can exceed 12 | **yes** (up to 13) | no |
| Pre-epoch dates | garbage (`Word` underflow) | works (pure arithmetic on the Gregorian y/m/d) |
| Behaviour from Jalali 1400 | **breaks** (1-char year) | correct |
| Call sites | none | none |

If a value shaped `'0/07/14'` or `'xx/13/xx'` is ever found in production data, it came from
Algorithm A or an equivalent, and the row's date is not trustworthy.

### 6.4 Validation on the way in

Two layers, both string-based, both in `Dmu.pas`:

`Dmu.pas:883-900` — syntactic:

```pascal
function TDM.IsDate(D1: String): Boolean;
begin
   Result := False;
   if (Length(d1)<>8) and (Length(D1)<>10 ) Then Exit;
   if length(d1)=8 then d1 := '13'+D1;
   if (Copy(D1,5,1)<>'/') or (Copy(D1,8,1)<>'/') Then Exit;
   D1 := Copy(D1,1,4) + Copy(D1,6,2) + Copy(D1,9,2);
   if Not isInteger( D1 ) Then Exit;
   S := Strtoint( Copy(D1,1,4) );  M := Strtoint( Copy(D1,5,2) );  R := Strtoint( Copy(D1,7,2) );
   if (M>12) or (M<1) then Exit;
   if (R>31) or (R<1) then Exit;
   if (M>6) and (R>30) then Exit;
   if (S>1420) or (S<1300) then Exit;
   Result := True;
end;
```

- Accepts **either** width; 8-char input is widened to 10 by prefixing the literal `'13'`.
- Month 1–12; day 1–31 for months 1–6, 1–30 for months 7–12. **Leap years are never checked**:
  `1400/12/30` passes validation although 1400 is not a Jalali leap year and Esfand 1400 has 29 days.
- Hard-coded year window **1300–1420** (`Dmu.pas:898`). The application stops accepting dates on
  1421/01/01 (Gregorian 2042-03-21). A latent expiry.

`Dmu.pas:911-919` — semantic: the date must fall inside the open fiscal year.

```pascal
function TDM.isValidDate(D1: String): Boolean;
begin
    Result := False;
    if Not IsDate(D1) then Exit;
    if Length(D1)=8 then D1 := '13'+D1;
    if D1 < From_Date then Exit;
    if D1 > To_Date then Exit;
    Result := True;
end;
```

`From_Date` / `To_Date` (`Dmu.pas:1137-1149`) read `Base.FromDate` / `Base.ToDate` for the current
`CO_ID` — the fiscal-year bounds (see §1.4). The comparison is a plain **string** comparison.

Only four call sites gate on it: `GetD.pas:49`, `Get2D.pas:67`, `Sanad_NDU.pas:60,83`. Direct
edits through the many `TsEdit` / `TEditDate` fields elsewhere are **not** validated —
e.g. `AnbarFactorU.pas:593` feeds `AF_Date.Farsi_Date` straight into voucher-number allocation.

`TUtil.IsFarsiDate` (`Utility.pas:526-541`) is a third, stricter, 8-char-only validator with the
same missing-leap-year hole (`((R>30) And (M>6))`). It has no call sites either.


---

[← 02-05-keys-identity-and-document-numbering.md](02-05-keys-identity-and-document-numbering.md) | [02-06-b-date-handling-arithmetic-and-model.md →](02-06-b-date-handling-arithmetic-and-model.md)
