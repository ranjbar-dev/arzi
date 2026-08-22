_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 7. `SahamdarConfig` — party account configuration

### 7.1 Purpose

A small, hand-seeded lookup table that answers: *"when a party card exists, under which control
accounts should it have a detail account, and which of those count toward its current-account
balance?"* It is the **backbone of the whole domain** — every party↔account resolution in §2, §4 and
§6 goes through it.

### 7.2 Columns

| Column | Type | Meaning | Evidence |
|---|---|---|---|
| `SC_K` | int | Kol (general-ledger) code of the control account | `Dmu.dfm:8632`; `CardJariU.dfm:6511` |
| `SC_M` | int | Moein (subsidiary) code of the control account | same |
| `SC_T` | int | Fixed Tafsil-1 code. `0` ⇒ the party card occupies Tafsil-1. `>0` ⇒ Tafsil-1 is fixed to `SC_T` and the party card occupies Tafsil-2. | `Dmu.dfm:8672-8673`; `SahamdarInfoU.pas:108` |
| `SC_Name` | varchar | Display name of the control account (shown in the `Coding` grid as `نام گروه` = "group name") | `SahamdarEditU.pas:101`; `SahamdarEditU.dfm:406` |
| `SC_1` | bit | Offer this control account for **natural persons** (`S_Kind=1`) | `SahamdarEditU.dfm:533` |
| `SC_2` | bit | Offer this control account for **legal entities** (`S_Kind=2`) | `CompanyEditU.dfm:417` |
| `SC_Add` | bit | Offer this row by default on the add/edit screen (combined with `SC_1`/`SC_2` by `AND`) | `SahamdarEditU.dfm:533`; `CompanyEditU.dfm:417` |
| `SC_Rem` | bit | Include this control account in the current-account balance (`Jari_Rem`) and sort it first in the account list (`QList`) | `Dmu.dfm:8670`; `CardJariU.dfm:6511`; `RoyatJU.pas:291` |
| `SC_Kind` | int | Alternative kind selector, used only by the unused `Dm.SahamdarConfig` query | `Dmu.dfm:8636` |
| `SC_Tik` | bit | **Transient scratch column.** Recomputed by a global `UPDATE` on every open (see §7.4) to mean "this party already has a detail account here". | `Dmu.dfm:8629-8634` |

### 7.3 Seeded control accounts

The `Coding` `TVirtualTable` in each editor carries a design-time snapshot of the expected
`SahamdarConfig` content. Decoded from the CP-1256 blob.

**Natural persons** (`SahamdarEditU.dfm:477-496`):

| `M_L` | `S_Ko` | `S_Mo` | `SC_Name` (Persian) | English |
|---|---|---|---|---|
| `103-001` | 103 | 1 | `حسابهاي دريافتني تجاري - اشخاص` | Trade accounts receivable — persons |
| `104-001` | 104 | 1 | `حسابهاي دريافتني - پرسنل` | Accounts receivable — personnel |
| `104-002` | 104 | 2 | `حسابهاي دريافتني - مستاجرين` | Accounts receivable — tenants |
| `109-001` | 109 | 1 | `اسناد تضامني دريافتني - اشخاص` | Guarantee notes receivable — persons |
| `301-001` | 301 | 1 | `حسابهاي پرداختني تجاري - اشخاص` | Trade accounts payable — persons |
| `301-003` | 301 | 3 | `اسناد پرداختني تجاري - اشخاص` | Trade notes payable — persons |

**Legal entities** (`CompanyEditU.dfm:363-380`):

| `M_L` | `S_Ko` | `S_Mo` | `SC_Name` (Persian) | English |
|---|---|---|---|---|
| `103-002` | 103 | 2 | `حسابهاي دريافتني تجاري - شرکتها` | Trade accounts receivable — companies |
| `104-002` | 104 | 2 | `حسابهاي دريافتني - شرکتها` | Accounts receivable — companies |
| `109-002` | 109 | 2 | `اسناد تضامني دريافتني - شرکتها` | Guarantee notes receivable — companies |
| `301-002` | 301 | 2 | `حسابهاي پرداختني تجاري - شرکتها` | Trade accounts payable — companies |
| `301-004` | 301 | 4 | `اسناد پرداختني تجاري - شرکتها` | Trade notes payable — companies |

All seeded rows have `S_Ta1 = 0`, `S_Ta2 = 0` — i.e. **all live configurations put the party card at
Tafsil-1**. Note `104-002` collides across the two sets (tenants for persons, companies for legal
entities); the seed blobs are stale relative to whatever the production table now holds.

**`M_L` format:** `Kol-Moein` with the Moein zero-padded to 3 digits, built at runtime as a plain
concatenation (which does *not* pad — the DFM snapshot was hand-authored):

```pascal
// SahamdarEditU.pas:99-100
     Coding.FieldByName('M_L').AsString := SC.FieldByName('SC_K').AsString + '-' +
                                           SC.FieldByName('SC_M').AsString ;
```

### 7.4 The three `SahamdarConfig` queries

**(a) Natural-person editor** (`SahamdarEditU.dfm:522-534`):

```sql
Declare @Card int Set @Card=:Card

Update SahamdarConfig Set SC_Tik=0
Update SahamdarConfig Set SC_Tik= 1 Where Exists(
    Select * From Sarfasl Where Sarfasl.S_Ko=SahamdarConfig.SC_K and Sarfasl.S_Mo=SahamdarConfig.SC_M and Sarfasl.S_Ta1=@Card and Sarfasl.S_Ta2=0)

Select * From SahamdarConfig Where (SC_1=1 and SC_Add=1) or SC_Tik=1
```

**(b) Legal-entity editor** (`CompanyEditU.dfm:406-418`) — identical except the last line:

```sql
Select * From SahamdarConfig Where (SC_2=1 and SC_Add=1) or SC_Tik=1
```

**(c) `Dm.SahamdarConfig`** (`Dmu.dfm:8625-8636`) — same preamble, then:

```sql
Select * From SahamdarConfig Where SC_Kind=@Kind or SC_Tik=1
```

Selection rule, stated plainly: **offer the default set for this party kind, plus every control
account where the party already has an account** (so an existing, non-default account is never
silently dropped from the screen).

> **Serious concurrency defect to fix in the rebuild.** `SC_Tik` is a *global* column mutated by
> `UPDATE SahamdarConfig Set SC_Tik=0` on **every** open of the editor. Two users editing two
> different parties simultaneously will overwrite each other's `SC_Tik`, and the tick marks shown to
> one user reflect the other user's party. The rebuild must compute this per-request, not persist it.
> §12-Q16 / §13-I1.

**Query (c) is dead.** `SahamdarU.pas:217-218` and `:249-250` only assign its connection string and
close it; its `Card` and `Kind` parameters are never set and it is never `Open`ed:

```pascal
// SahamdarU.pas:215-219
procedure TSahamdar.B_NewClick(Sender: TObject);
begin
     Dm.SahamdarConfig.Close;
     Dm.SahamdarConfig.ConnectionString := Dm.Ado.ConnectionString;
```

### 7.5 How the editor builds the tick grid

```pascal
// SahamdarEditU.pas:85-110  (identical at CompanyEditU.pas:76-101)
procedure TSahamdarEdit.Open_Coding;
var i:integer;
begin
   Coding.Close;
   if SCard.IntValue=0 then Exit;
   SC.Close;
   SC.ConnectionString := Dm.Ado.ConnectionString;
   SC.Parameters.ParamByName('Card').Value := SCard.IntValue;
   SC.Open;
   Coding.Open;
   While Coding.RecordCount >0 Do Coding.Delete;
   for I := 1 to SC.RecordCount do
   Begin
     Coding.Append;
     Coding.FieldByName('M_L').AsString := SC.FieldByName('SC_K').AsString + '-' +
                                           SC.FieldByName('SC_M').AsString ;
     Coding.FieldByName('S_Name').AsString := SC.FieldByName('SC_Name').AsString ;
     Coding.FieldByName('S_Ko').AsString := SC.FieldByName('SC_K').AsString ;
     Coding.FieldByName('S_Mo').AsString := SC.FieldByName('SC_M').AsString ;
     Coding.FieldByName('S_Ta1').AsString := SC.FieldByName('SC_T').AsString ;
     Coding.FieldByName('S_Found').Value := SC.FieldByName('SC_Tik').Asinteger = 1  ;
     Coding.Post;
     SC.Next;
   End;
   SC.Close;
end;
```

Then the tick is **recomputed row by row**, and — crucially — forced to `False` for a brand-new party:

```pascal
// SahamdarEditU.pas:169-184  (identical at CompanyEditU.pas:156-172)
    Q1.SQL.Add('Select * From Sarfasl Where S_Ko=:KO and S_Mo=:Mo and S_Ta1=:Ta1 and S_Ta2=:Ta2');

    for I := 1 to Coding.RecordCount do
    begin
       Q1.Close;
       Q1.Parameters.ParamByName('Ko').Value := Coding.FieldValues['S_Ko'];
       Q1.Parameters.ParamByName('Mo').Value := Coding.FieldValues['S_Mo'];
       Q1.Parameters.ParamByName('Ta1').Value := SCard.IntValue;
       Q1.Parameters.ParamByName('Ta2').Value := 0;
       Q1.Open;
       Coding.Edit;
       Coding.FieldByName('S_Found').AsBoolean := (Q1.RecordCount =1) and (_Card>0) ;
       Coding.Post;
       Coding.next;
    end;
```

Note `Ta1 := SCard.IntValue` and `Ta2 := 0` are hard-coded here too — `Coding.S_Ta1` (loaded from
`SC_T`) is ignored, consistent with §2.4's finding.

> Because the tick is *derived from existence*, unticking a row and saving does **not** delete the
> account — the save loop only ever *adds* (`SahamdarEditU.pas:318-329`). Deletion is impossible
> from this screen. §12-Q17.

---


---

[← Previous](07-06-b-party-current-account-jari.md) · [Index](00-index.md) · [Next →](07-08-accounting-integration.md)
