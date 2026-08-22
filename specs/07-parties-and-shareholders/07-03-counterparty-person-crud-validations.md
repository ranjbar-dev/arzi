_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 3. Counterparty / person CRUD validations — exhaustive

### 3.1 Where CRUD happens

| Entity | Create/Update form | Delete |
|---|---|---|
| Natural person (`S_Kind=1`) | `SahamdarEditU.pas` | **not implemented** |
| Legal entity (`S_Kind=2`) | `CompanyEditU.pas` | **not implemented** |
| Chart-of-accounts node | `SNewu.pas` → `Sarfasl_ADD` | `SNewu.pas` (guarded, §2.5) |
| Party extended attributes | `Sarfasl_TakmilU.pas` | n/a |

Delete is blocked at the dataset level:

```pascal
// SahamdarU.pas:198-201
procedure TSahamdar.Q1BeforeDelete(DataSet: TDataSet);
begin
   Abort;
end;
```
and the Delete button (`B_Delete`, `SahamdarU.dfm:422-437`) has **no `OnClick` handler at all**,
even though `SahamdarU.pas:109` makes it visible for users holding permission 1108.

### 3.2 Natural-person validations (`SahamdarEditU.pas:218-331`)

> The Persian literals in `SahamdarEditU.pas` are stored in **Windows-1256**. Reading the file as
> UTF-8/Latin-1 shows mojibake (`'  ���� ������� ����  '`). All strings below are the decoded
> CP-1256 text.

Pre-trim (`SahamdarEditU.pas:222-226`): `SCodeMelli`, `SCodePosti`, `SName`, `SFamil`, `SFather` are
`Trim`-ed in place before any check.

| # | Rule | Persian message | English | `file:line` |
|---|---|---|---|---|
| V1 | `SName` (first name) must be non-empty | `ورود اطلاعات ناقص` | "Incomplete data entry" | `SahamdarEditU.pas:228-233` (msg at `:230`) |
| V2 | `SFamil` (surname) must be non-empty | `ورود اطلاعات ناقص` | "Incomplete data entry" | `SahamdarEditU.pas:235-240` (msg at `:237`) |
| V3 | `SFather` (father's name) must be non-empty | `ورود اطلاعات ناقص` | "Incomplete data entry" | `SahamdarEditU.pas:242-247` (msg at `:244`) |
| V4 | On **create** (`SCard.Tag = 1`): card number must not already exist in `Sahamdar` | `شماره شناسايي تکراري است` | "Identification number is duplicated" | `SahamdarEditU.pas:249-259` (msg at `:256`) |
| V5 | On **create**: `S_CodeMelli` must not already exist | `کدملي تکراري است` | "National ID is duplicated" | `SahamdarEditU.pas:261-271` (msg at `:268`) |
| V6 | On **update** (`SCard.Tag = 2`): if `S_CodeMelli` exists it must belong to this same card | `کدملي تکراري است` | "National ID is duplicated" | `SahamdarEditU.pas:273-279` (msg at `:276`) |

Focus is moved to the offending control in every case (`ActiveControl := SName` / `SFamil` /
`SFather` / `SCard` / `SCodeMelli`).

The two uniqueness probes verbatim:

```sql
-- SahamdarEditU.pas:251
Select * from sahamdar Where S_Card=<SCard.Inttext>
```
```sql
-- SahamdarEditU.pas:263
Select * From Sahamdar Where S_CodeMelli=<QuotedStr(SCodeMelli.Text)>
```

> **V5/V6 have a hole:** an empty national ID is not exempted. The first person saved with a blank
> `S_CodeMelli` makes every subsequent blank-ID create fail with "National ID is duplicated".
> §12-Q10.

**Not validated (deliberately noted, because a naive rebuild would add them):**
national-ID checksum, postal-code format, mobile-number format, birth/issue date being a real
Jalali date (only the *keystroke filter* `['/','0'..'9',#8]` at `SahamdarEditU.pas:364-370` applies),
address length, `S_CodeSabt` (max length 12 enforced only by the control,
`SahamdarEditU.dfm:424`).

The DM does own IBAN and bank-card validators, but **they are not wired into these forms**:

```pascal
// Dmu.pas:196-214  — IBAN / SHABA mod-97 check
function TDM.IsValidShaba(S: String): Boolean;
Var i, j: integer;
begin
  S:=UpperCase(S);
  Result := False;
  if Copy(S, 1, 2) <> 'IR' Then Exit;
  if Length(S) <> 26 then Exit;
  S := Copy(S, 5, 22) + '1827' + Copy(S, 3, 2);
  j := 0;
  For i := 1 to Length(S) Do
  Begin
    j := j * 10 + strtoint(Copy(S, i, 1));
    j := j mod 97;
  End;
  Result := j = 1;
end;
```

```pascal
// Dmu.pas:216-240  — Luhn check on a 16-digit card, first digit ∈ {4,5,6}
function TDM.IsValidKart(S: String): Boolean;
...
  j := j mod 10;
  Result := j = 0;
end;
```

### 3.3 Legal-entity validations (`CompanyEditU.pas:198-302`)

Same shape, **three** differences: no father's-name rule, and the national-ID message is worded for
a legal entity.

| # | Rule | Persian message | English | `file:line` |
|---|---|---|---|---|
| V1 | `SName` must be non-empty | `ورود اطلاعات ناقص` | "Incomplete data entry" | `CompanyEditU.pas:207-212` (msg `:209`) |
| V2 | `SFamil` must be non-empty | `ورود اطلاعات ناقص` | "Incomplete data entry" | `CompanyEditU.pas:214-219` (msg `:216`) |
| V3 | On create: card number unique | `شماره شناسايي تکراري است` | "Identification number is duplicated" | `CompanyEditU.pas:221-231` (msg `:228`) |
| V4 | On create: `S_CodeMelli` unique | `شناسه ملی تکراري است` | "Legal-entity national ID is duplicated" | `CompanyEditU.pas:233-243` (msg `:240`) |
| V5 | On update: `S_CodeMelli` must belong to this card | `شناسه ملی تکراري است` | "Legal-entity national ID is duplicated" | `CompanyEditU.pas:245-251` (msg `:248`) |

For a legal entity, `SName` = `نام شخصیت` ("entity name", `CompanyEditU.dfm:90`) and `SFamil` =
`نام مدیر یا نماینده` ("manager / representative name", `CompanyEditU.dfm:99`).

### 3.4 The write itself

**Create** (only when `SCard.Tag = 1`) inserts a stub with five columns:

```pascal
// SahamdarEditU.pas:288-293  (natural person → S_kind = 1)
   if SCard.Tag=1 then
   Begin  // append
      Q1.SQL.Add('insert sahamdar (S_kind, S_Card, S_Name, S_Famil, S_Father)') ;
      Q1.SQL.Add(' values(1, '+ SCard.Inttext+', '+ QuotedStr(SName.Text)+', '+ QuotedStr(SFamil.Text)+', '+ QuotedStr(SFather.Text)+' )' );
      Q1.ExecSQL;
   End;
```
```pascal
// CompanyEditU.pas:260-265  (legal entity → S_kind = 2, empty father)
      Q1.SQL.Add('insert sahamdar (S_kind, S_Card, S_Name, S_Famil, S_Father)') ;
      Q1.SQL.Add(' values(2, '+ SCard.Inttext+', '+ QuotedStr(SName.Text)+', '+ QuotedStr(SFamil.Text)+', '''' )' );
```

**Update** always runs afterwards, for both create and edit paths:

```sql
-- SahamdarEditU.pas:297-312  (assembled)
Update Sahamdar Set S_Name=<name>
   , S_Famil=<surname>
   , S_Father=<father>
   , S_Mobile=<mobile>
   , S_BDate=<birth date>
   , S_SDate=<ID issue date>
   , S_BPlace=<birth place>
   , S_SPlace=<ID issue place>
   , S_Address=<address>
   , S_CodeMelli=<national id>
   , S_CodePosti=<postal code>
   , S_CodeSabt=<registration code>
   , S_MaliatState=<tax status index>
   , S_IDNO=<ID document number>
 Where S_Card=<card>
```

```sql
-- CompanyEditU.pas:269-283  (assembled) — note the forced blanks
Update Sahamdar Set S_Name=<entity name>
   , S_Famil=<representative>
   , S_Father='' 
   , S_Mobile=<mobile>
   , S_BDate=<incorporation date>
   , S_SDate='' 
   , S_BPlace='' 
   , S_Address=<address>
   , S_CodeMelli=<legal national id>
   , S_CodePosti=<postal code>
   , S_CodeSabt=<registration code>
   , S_MaliatState=<tax status index>
   , S_IDNO=0
 Where S_Card=<card>
```

> `CompanyEditU.pas:275` blanks `S_BPlace` even though the form has a visible
> `محل تاسیس` ("place of incorporation") box (`CompanyEditU.dfm:36,152-161`) that is loaded at
> `CompanyEditU.pas:144`. **The field is displayed, editable, and silently discarded on save.**
> §12-Q11.

After the update, `SCard.Tag := 0` and the account auto-creation loop runs (§2.4), then `Close`.

> **All of this is string-concatenated SQL.** `QuotedStr` escapes single quotes for text fields, but
> `SCard.Inttext` and `SIDNO.Inttext` are interpolated raw. The rebuild must use bound parameters
> throughout.

---


---

[← Previous](07-02-b-counterparty-taraf-model.md) · [Index](00-index.md) · [Next →](07-04-a-person-legal-entity-sahamdar-model.md)
