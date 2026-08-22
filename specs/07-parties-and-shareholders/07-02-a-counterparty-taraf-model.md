_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 2. Counterparty (Taraf) model

### 2.1 `TarafU` — the counterparty **selector**, not a record

`TTaraf` (caption `طرف حساب` = "counterparty", `TarafU.dfm:5`) is a modal dialog with four numeric
edits (`EKo`, `EMo`, `ETa1`, `ETa2`), four read-only name displays (`SKo`, `SMO`, `STa1`, `STa2`),
four lookup buttons (`BKo`, `BMO`, `BTa1`, `BTa2`), and `OK` / `Exit`. It has **no Save**: both
`B_OK` and `B_Exit` are wired to `B_ExitClick` → `Close` (`TarafU.dfm:138`, `:153`;
`TarafU.pas:538-541`).

Its public contract (`TarafU.pas:67-76`):

| Member | Semantics | Line |
|---|---|---|
| `init(C_Ko, C_Mo, C_Ta1, C_Ta2)` | Reset, optionally preload each segment, `ShowModal` | 156-187 |
| `Set_FullCode(Code: String)` | Parse a dash-separated code `"Ko-Mo-Ta1-Ta2"` into the four boxes, driving `OnChange` validation | 189-230 |
| `Set_SSN(_SSN: integer)` | Load segments from `Sarfasl` by surrogate key | 232-252 |
| `Get_Valid: Boolean` | `F_Valid = 1`, i.e. the resolved node is a **leaf** (`S_Child = 0`) | 151-154 |
| `Get_SSn: integer` | `Sarfasl.S_SSN` of the resolved leaf, or `0` if invalid | 146-149 |
| `Get_FullCode: String` | `"Ko-Mo-Ta1-Ta2"`, only the non-zero segments | 104-113 |
| `Get_FullName: String` | `"KolName/MoeinName/Ta1Name/Ta2Name"` | 126-135 |
| `Get_FullCodeName: String` | Multi-line `"<code> : <name>"` per level | 115-124 |
| `Get_LastName: String` | `"<deepest code> <deepest name>"`, `''` when invalid | 137-144 |

**The four lookup queries** (`TarafU.dfm`), one per level, all against `Sarfasl`:

```sql
-- TarafU.dfm:288  (Q1 — Kol list)
Select * From Sarfasl Where S_Ko>0 And S_Mo=0
```
```sql
-- TarafU.dfm:304  (Q2 — Moein under a Kol)
Select * From Sarfasl Where S_Ko=:Ko And S_Mo>0 and S_Ta1=0
```
```sql
-- TarafU.dfm:329-330  (Q3 — Tafsil1 under a Moein)
Select * From Sarfasl Where S_Ko=:Ko And S_Mo=:Mo and S_Ta1>0 and S_Ta2=0
```
```sql
-- TarafU.dfm:363-364  (Q4 — Tafsil2 under a Tafsil1)
Select * From Sarfasl Where S_Ko=:Ko And S_Mo=:Mo and S_Ta1=:Ta1 and S_Ta2>0
```
```sql
-- TarafU.pas:241  (QS — resolve by surrogate key; note the string concatenation)
Select * From Sarfasl Where S_SSN=<n>
```

`QS` is declared in the DFM (`TarafU.dfm:368-401`) with three unused parameters and an SQL body that
is *overwritten at runtime* by `TarafU.pas:240-241`. Dead DFM configuration.

**Cascade / validation semantics** (the heart of the widget). Each `OnChange` handler resets
everything downstream, then re-resolves:

```pascal
// TarafU.pas:341-365  (Kol level)
procedure TTaraf.EKoChange(Sender: TObject);
var S:string;
begin
   EKo.Brush.Color := Clwindow;
   EKo.Repaint;
   F_Valid:=0;
   F_SSN:=0;
   EKo.Tag:=0;
   Sko.Text:='';
   EMo.Text:='';
   ETa1.Text:='';
   ETa2.Text:='';
   S:= Trim(EKo.Text);
   if Length(S)=0 then exit;
   EKo.Brush.Color := RGB(255,248,220);      // cornsilk  = typed but unresolved
   EKo.Repaint;
   Q1.Close;
   Q1.ConnectionString:=Dm.Ado.ConnectionString;
   Q1.Open;
   if Not Q1.Locate('S_Ko', S, [loCaseInsensitive]) then exit;
   SKo.Text:=Q1.FieldByName('S_Name').AsString;
   EKo.Tag:=Q1.FieldByName('S_Ko').AsInteger;
   EKo.Brush.Color := RGB(175,238,238);      // pale turquoise = resolved
   EKo.Repaint;
end;
```

Colour code, identical at all four levels
(`TarafU.pas:344,355,363` / `370,380,396` / `404,413,430` / `438,446,463`):

| Colour | RGB | Meaning |
|---|---|---|
| `clWindow` | system | empty |
| cornsilk | `255,248,220` | text entered, not resolved to an account |
| pale turquoise | `175,238,238` | resolved |

**Leaf detection.** `F_Valid` is set to 1 **only** at the Moein/Tafsil1/Tafsil2 levels, and only when
the located row has `S_Child = 0`:

```pascal
// TarafU.pas:391-395 (Moein)
   if Q2.FieldByName('S_Child').AsInteger=0 then
   begin
      F_Valid := 1;
      F_SSN:= Q2.FieldByName('S_SSN').AsInteger;
   end;
```
Identical blocks at `TarafU.pas:425-429` (Tafsil1) and `TarafU.pas:458-462` (Tafsil2).

> A Kol-level account can therefore **never** be `Valid`, even if it is childless
> (`TarafU.pas:341-365` never sets `F_Valid`). Postings must be at Moein level or deeper.

**Focus / read-only chain** — a deeper box is only editable when its parent resolved:

```pascal
// TarafU.pas:270-282
procedure TTaraf.EMoEnter(Sender: TObject);
begin
    if EKo.Tag=0 then
    Begin
      EMo.ReadOnly:=True;
      ActiveControl:=EKo;
      Exit;
    End;
    EMo.ReadOnly:=False;
    ...
```
```pascal
// TarafU.pas:283-300  (Tafsil1 — also short-circuits to OK when the Moein is already a valid leaf)
    if EMo.Tag=0 then ... ActiveControl := EMo; Exit;
    if (F_Valid=1) and (ETa1.Tag=0) then
    Begin
      ActiveControl := B_OK;
      Exit;
    End;
```
Same pattern for Tafsil2 at `TarafU.pas:467-484`.

**Keyboard:** digits and backspace only; `Enter` advances to the next level when the current one
resolved (`TarafU.pas:254-261`, `316-323`, `332-339`, `493-500`).

**Deep-link buttons** open `SelectSarfasl` filtered to the appropriate level
(`TarafU.pas:502-536`); `SelectSarfasl.pas:85-151` builds the four `Select Code=S_X, Sarfasl.*`
queries. Window geometry is persisted to the INI (`TarafU.pas:87-102`).

### 2.2 What a counterparty **is**

There is no `Taraf` table. A counterparty is:

1. **A node in `Sarfasl`** (chart of accounts) at Moein, Tafsil-1 or Tafsil-2 level, with
   `S_Child = 0`; plus
2. **optionally** a row in `Sahamdar` (the person/legal-entity register), joined by card number.

`Sarfasl` complete column list (union of `S_KolU.dfm:139-264`, `Sarfasl_TakmilU.pas:65-84`,
`ListSarfaslu.pas:311-317`, `Mainu.pas:824-838`, `SNewu.pas`):

| Column | Meaning | Evidence |
|---|---|---|
| `S_SSN` | Surrogate key (identity) | `TarafU.pas:241`; `Dmu.pas:1044` |
| `S_Ko` | Kol (general ledger) segment | `TarafU.dfm:288` |
| `S_Mo` | Moein (subsidiary) segment | `TarafU.dfm:304` |
| `S_Ta1` | Tafsil-1 (analytic/detail) segment | `TarafU.dfm:329` |
| `S_Ta2` | Tafsil-2 segment | `TarafU.dfm:363` |
| `S_Name` | Account / party display name | `Sarfasl_TakmilU.pas:66` |
| `FullName` | Denormalised `"Kol/Moein/Ta1/Ta2"` path | `Dmu.pas:284-295` (rebuild SQL, commented out); `SelectSarfasl.pas:53` |
| `M_L`, `M_R` | Nested-set / materialised code strings, built by `Dbo.Make_L` / `Dbo.Make_R` | `Dmu.pas:274-278` (commented) ; `S_KolU.dfm:222,227` |
| `S_Child` | Count of children; `0` ⇒ postable leaf | `TarafU.pas:391`; `Dmu.pas:300` |
| `S_Count` | Transaction count | `S_KolU.dfm:152`; `Mainu.pas:828` |
| `S_Bed` | Cached total debit | `S_KolU.dfm:192` |
| `S_Bes` | Cached total credit | `S_KolU.dfm:197` |
| `S_Remi` | Cached balance | `S_KolU.dfm:202` |
| `S_Active` | Active flag | `S_KolU.dfm:217` |
| `S_A` | Legacy active flag (import path) | `Mainu.pas:832` |
| `S_Lock` | Per-node lock; blocks non-admin posting | `Dmu.pas:934,944,954,964`; `SNewu.pas:358-365` |
| `S_Kind` | Account kind — **column exists, never written by any Pascal code** | `S_KolU.dfm:207` only |
| `S_Card` | FK to `Sahamdar.S_Card` | `ListSarfaslu.pas:317`; `S_KolU.dfm:212` |
| `S_Address` | `آدرس` — address | `Sarfasl_TakmilU.pas:67,136` |
| `S_Tel` | `تلفن` — telephone | `Sarfasl_TakmilU.pas:69,142` |
| `S_Fax` | `فاکس` — fax | `Sarfasl_TakmilU.pas:70,141` |
| `S_Sabt` | `شماره ثبت` — commercial registration number | `Sarfasl_TakmilU.pas:68,137` |
| `S_Melli` | `شناسه ملی` / `کد ملی` — national ID | `Sarfasl_TakmilU.pas:73,138` |
| `S_Egh` | `کد اقتصادي` — economic (tax) code | `Sarfasl_TakmilU.pas:71,139` |
| `S_Post` | `کد پستي` — postal code | `Sarfasl_TakmilU.pas:72,140` |
| `S_IS_Check` | Eligible for cheque transactions | `S_KolU.dfm:242`; `Sarfasl_TakmilU.pas:75-76` (commented) |
| `S_IS_Fish` | Eligible for deposit-slip transactions | `S_KolU.dfm:247` |
| `S_IS_APArdakhti` | Eligible for notes payable | `S_KolU.dfm:252` |
| `S_IS_ADaryafti` | Eligible for notes receivable | `S_KolU.dfm:257` |
| `NeedUpdate` | Housekeeping flag | `S_KolU.dfm:262` |

> **There is no credit-limit column and no credit-limit logic anywhere in the project.** Grep for
> `Limit`, `Sagf`, `سقف` returns nothing in a party context. §12-Q8.

> **There is no grouping/classification table for counterparties.** Classification is *positional*:
> a party's Kol/Moein control account determines whether it is a trade receivable, a payable, a
> tenant, an employee, etc. (see the `SahamdarConfig` seed data in §7.3). `Kinds` (`Dmu.dfm:301-307`,
> `Dmu.pas:20`) is declared but **never referenced** by any unit.


---

[← Previous](07-01-b-company-multi-tenancy-model.md) · [Index](00-index.md) · [Next →](07-02-b-counterparty-taraf-model.md)
