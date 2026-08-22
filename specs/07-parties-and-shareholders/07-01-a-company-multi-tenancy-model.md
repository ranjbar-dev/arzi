_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 1. Company / multi-tenancy model

### 1.0 Terminology warning

The word "company" is used for **three unrelated things** in the legacy code. Do not conflate them
in the rebuild.

| Legacy artifact | What it really is | Proposed English name |
|---|---|---|
| `Base` table, `CO_ID`, `Co_Name`, `Co_Sub` (`Dmu.dfm:26-33`, `TanzimU.pas:120-144`) | The operating entity **and** the fiscal year, in one row | `fiscal_period` (+ denormalised `organization_profile`) |
| `CompanyEditU.pas` / `TCompanyEdit` | Editor for **legal-entity counterparties** (`S_Kind=2`) — customers/suppliers that are companies | `LegalEntityPartyEditor` |
| `Saham.Dbo` external database (`Dmu.pas:758-780`) | A separate share-registry product | `share_registry` (external system) |

### 1.1 What a "company" is here

A row of `Base`. `Dmu.pas:745-749` opens it at startup; `Dmu.pas:113` declares the ambient
`CO_ID : integer`.

```pascal
// Dmu.pas:745-749
   Base.Active:=False;
   Base.ConnectionString := Ado.ConnectionString;
   Base.Active := True;
   RegName := ''; //Base.Fieldbyname('Co_Name').AsString ;
   RegSal  := ''; //Dm.Base.FieldByName('CO_Sub').AsString ;
```

`Base` is a plain `TADOTable` on `TableName = 'Base'` with deletion blocked
(`BeforeDelete = QCheckBeforeDelete` → `Abort`, `Dmu.dfm:29`, `Dmu.pas:701-704`).

`Base_Q` (`Dmu.dfm:362-375`) is the display query used by the login screen:

```sql
-- Dmu.dfm:367-372
Select *,  LTrim(RTrim(Co_Name)) + '  =  ' +  LTrim(RTrim(Co_Sub))   As CO_DESC
From Base
Order By Co_ID
```

### 1.2 Complete `Base` field list

Derived by union of every `Base.FieldByName(...)` reference in the project
(`TanzimU.pas:121-143`, `MakeNewU.pas:71-124`, `Dmu.pas:1008-1223`, `EnteghalU.pas:305-322`,
`AnbarFactorU.pas:767`).

| Column | Type (inferred) | Meaning (Persian → English) | Where read / written |
|---|---|---|---|
| `Co_ID` | int, PK | Fiscal-year identifier. In practice the Jalali year number (`1391`, `1397`, `1403` — see default parameter values in `Dmu.dfm:739`, `Dmu.dfm:790`). | `ChangesU.pas:50,78`; `MakeNewU.pas:73,119` |
| `Co_Name` | varchar(100) | `نام شرکت` — legal/trading name of the operating entity | `TanzimU.pas:121,164-168` |
| `Co_Sub` | varchar(100) | `نام سيستم` — subtitle/label; used as the fiscal-year caption (`RegSal`) | `TanzimU.pas:123,186-190`; `ChangesU.pas:79` |
| `Co_Address` | varchar(100) | `آدرس شرکت` — address | `TanzimU.pas:122,175-179` |
| `Co_Tel` | varchar(20) | `تلفن` — telephone | `TanzimU.pas:125,208-212` |
| `Co_Fax` | varchar(20) | `فاکس` — fax | `TanzimU.pas:126,197-201` |
| `Co_Web` | varchar(30) | `وب سايت` — website | `TanzimU.pas:127,219-223` |
| `Co_EMail` | varchar(30) | `پست الکترونيک` — e-mail | `TanzimU.pas:128,240-244` |
| `Co_Sabt` | varchar(20) | `شماره ثبت` — commercial registration number | `TanzimU.pas:137,320-324` |
| `Co_Melli` | varchar(20) | `شناسه ملی` — national legal-entity ID | `TanzimU.pas:138,342-346` |
| `Co_Egh` | varchar(20) | `کد اقتصادي` — economic (tax) code | `TanzimU.pas:139,331-335` |
| `Co_Post` | varchar(20) | `کد پستي` — postal code | `TanzimU.pas:140,353-357` |
| `ARM` | image | Letterhead logo/emblem, bound to `TDBImage` (`TanzimU.pas:62, 226-234`; `Mainu.dfm:4390` binds `DM.DS_Base`) | `TanzimU.pas:228-233` |
| `FromDate` | varchar(10), Jalali `YYYY/MM/DD` | Fiscal-year start | `Dmu.pas:1137-1142`, `TanzimU.pas:251-256` |
| `ToDate` | varchar(10), Jalali | Fiscal-year end | `Dmu.pas:1144-1149`, `TanzimU.pas:263-268` |
| `IsActive` | int (0/1) | 0 = archived year: no new vouchers/invoices allowed | `Dmu.pas:1008-1013` |
| `BackupDir` | varchar(100) | `مسير پشتيبان` — backup directory | `TanzimU.pas:136,275-279` |
| `No_Ko` | int | Display width (zero-pad length) of the Kol segment | `Dmu.pas:1200` |
| `No_Mo` | int | Display width of the Moein segment | `Dmu.pas:1206` |
| `No_Ta1` | int | Display width of the Tafsil-1 segment | `Dmu.pas:1213` |
| `No_Ta2` | int | Display width of the Tafsil-2 segment | `Dmu.pas:1220` |
| `Int_Len` | int | Integer display length (grid formatting) | referenced in `DKolU.pas`/`DMoein.pas` |
| `Real_Len` | int | Decimal display length | `TanzimU.pas:135` (commented out) |
| `C1081` | int (`Sarfasl.S_SSN`) | Pointer to the **cash/`صندوق`** account | `Dmu.pas:1073,1095`; `TanzimU.pas:285` |
| `C1081C` | varchar | Human code string for `C1081` | `Dmu.pas:1086`; `TanzimU.pas:287-288` |
| `C1082` | int (`Sarfasl.S_SSN`) | Pointer to the **in-transit / `جاريان`** account | `Dmu.pas:1108,1130`; `TanzimU.pas:286` |
| `C1082C` | varchar | Human code string for `C1082` | `Dmu.pas:1121`; `TanzimU.pas:289-290` |
| `Kh1_Code` … `Kh8_Code` | int | Eight configurable "quick account" slots | `Mainu.pas`, `FISHDaryaftU.pas` |
| `Kh1_Desc` … `Kh8_Desc` | varchar | Captions for those eight slots | same |

**No base-currency column exists.** All amounts are `Bigint` and implicitly Iranian Rial
(`Factorprint2U.pas:98` hard-codes the literal `' ریال '`). This is an open question — see §12-Q3.

### 1.3 How `CO_ID` scopes data

`CO_ID` is a **column stamp**, never a database or schema boundary. Evidence:

* One `TADOConnection` for the whole app, `Initial Catalog=Arzi89` (`Dmu.dfm:6-17`). Every form's
  `TADOQuery` copies `Dm.Ado.ConnectionString` verbatim.
* Journal lines: `Moein.M_COID` (`Dmu.pas:828`, `Dmu.pas:1163`, `Dmu.pas:1247`).
* Voucher headers: `DMoein.DM_Coid` (`Dmu.pas:830-835`).
* Inventory invoices: `Anbar_Factor.AF_COID` (`AnbarFactorU.pas:606`, `Dmu.pas:1258`).
* Invoice lines: `Anbar_FactorD.AFD_Coid` (`Dmu.dfm:667-701`).
* Cheques: `DCheck.S_COID` (`CheckDaryaftU.pas:282`), `Check_M.CM_Coid` (`CheckEditU.pas:427`).

**Not scoped by `CO_ID`:**

* `Sarfasl` (chart of accounts) — `TADOTable` with `TableName='Sarfasl'` and no filter
  (`Dmu.dfm:376-381`). A commented-out block in `MakeNewU.pas:129-150` shows an *abandoned* attempt
  to copy `Sarfasl` per year using a `S_COID` column; that column is dead.
* `Sahamdar` (person register) — `Dmu.dfm:561-567`, no filter.
* `SahamdarConfig`, `SahamdarInfo` — no year column anywhere.

**Consequence for the rebuild:** counterparties and persons are *global* master data; balances and
documents are *period* data. Preserve that split exactly.

### 1.4 How the user selects the active company / year

Two entry points.

**(a) At login** — `GetPassu.pas`. A `TDBLookupComboBox` named `CO_ID_IN` bound to `DM.Base_Q`:

```pascal
// GetPassu.pas:48-57
   DM.Base_Q.Active := False;
   DM.Base_Q.Active := True;
//   CO_ID_IN.KeyValue := 1;
   CO_ID_IN.KeyValue := DM.Base_Q.FieldByName('CO_ID').AsInteger;
```

```pascal
// GetPassu.pas:65-67
   if CO_ID_IN.KeyValue<1 then
   ...
      ActiveControl := CO_ID_IN;
```

```pascal
// GetPassu.pas:96-103
   dm.CO_ID := CO_ID_IN.KeyValue;
   ...
   dm.Base.close;
   dm.Base.Open;
   Dm.Base.Locate('Co_Id', inttostr(dm.CO_ID), [locaseinsensitive] );
   DM.RegName := Dm.Base.Fieldbyname('Co_Name').AsString ;
   DM.RegSal  := Dm.Base.FieldByName('CO_Sub').AsString ;
```

The last-used `CO_ID` is persisted to the local INI (`GetPassu.pas:125` read, `:136` write).

**(b) Mid-session switch** — `ChangesU.pas` (`TChangeS_F`), reached from `Mainu.pas:423`
(`Changes_F.init`).

```pascal
// ChangesU.pas:56-69
procedure TChangeS_F.init;
begin
    ...
    Q1.SQL.Add(' Select * From Base Order By Co_ID ');
    Q1.Open;
    Q1.Last;
    ShowModal;
end;
```

```pascal
// ChangesU.pas:48-54  (double-click a row)
procedure TChangeS_F.G1DblClick(Sender: TObject);
begin
    DM.CO_ID:=Q1.FieldByName('Co_ID').AsInteger;
    Close;
end;
```

```pascal
// ChangesU.pas:76-81  (the "Exit" button *also* commits the selection — a bug)
procedure TChangeS_F.B_ExitClick(Sender: TObject);
begin
    DM.CO_ID:=Q1.FieldByName('Co_ID').AsInteger;
    Dm.RegSal := Q1.FieldByName('CO_Sub').AsString;
    Close;
end;
```

> **Behavioural quirk to preserve or fix explicitly:** the *Cancel/Exit* button applies the change
> (`ChangesU.pas:78`) while *double-click* applies it without updating `RegSal`
> (`ChangesU.pas:50`). Deleting rows is blocked (`ChangesU.pas:71-74` → `Abort`).

After the switch the caller refreshes the main window caption:

```pascal
// Mainu.pas:423-429
   Changes_F.init;
   Dm.Base.Close;
   Dm.Base.Open;
   Dm.Base.Locate('Co_ID', Dm.CO_ID, [loCaseInsensitive]);
   B_Company.Caption := Dm.Base.FieldByName('CO_Name').AsString + #13#10
                        + Dm.Base.FieldByName('CO_Sub').AsString;
   Dm.RegSal := Dm.Base.FieldByName('Co_Sub').AsString;
```

Guard used before any new document is created in the active year:

```pascal
// Dmu.pas:997-1015
function TDM.Is_New_Sanad_Valid(COID: integer): Boolean;
begin
   Result := False;
   Base.Close;
   Base.ConnectionString := Ado.ConnectionString;
   Base.Open;
   if Base.Locate('CO_ID', Coid, [loCaseInsensitive]) = false then
   Begin
     MessageDlg('   سال مالی پیدا نشد  ', mterror, [mbok], 0);
     Exit;
   End;
   if Base.FieldByName('IsActive').asinteger <> 1  then
   Begin
     MessageDlg('          سال مالی مورد نظر بایگانی شده است                '+#13#10+
                '   اجازه تغییر در این سال و صدور فاکتور و سند را ندارید    ', mterror, [mbok], 0);
     Exit;
   End;
   Result := True;
end;
```

| Persian | English |
|---|---|
| `سال مالی پیدا نشد` | "Fiscal year not found" |
| `سال مالی مورد نظر بایگانی شده است` | "The selected fiscal year has been archived" |
| `اجازه تغییر در این سال و صدور فاکتور و سند را ندارید` | "You are not allowed to modify this year or issue invoices/vouchers" |


---

[← Previous](07-00-executive-summary.md) · [Index](00-index.md) · [Next →](07-01-b-company-multi-tenancy-model.md)
