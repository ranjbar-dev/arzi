_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 9.4 `FinalU.pas` — DEAD, documented for reference

**Not in `arzi.dpr`.** An earlier, single-Kol version of `NewFinalu`. Recorded because it contains two
rules the live version lost.

`Q2` (`FinalU.dfm:479-513`) is the per-leaf aggregation for one Kol, with a **typo bug**:

```sql
Select  M_Ko, M_Mo , M_Ta1, M_Ta2
       , Sum(M_Bed) As Bed , Sum(M_Bes) As Bes
       , Sum(M_Bed-M_Bes) As BedR
       , Sum(M_Bes-M_Bes) As BesR      -- <<< BUG: should be Sum(M_Bes-M_Bed)
into #R
From Moein
Where M_Kind=1 And M_COID=@Co
Group By M_Ko, M_Mo, M_Ta1, M_Ta2
Delete #R Where Bes - Bed = 0
Update #R Set Bed=0 Where Bed<0
Update #R Set Bes=0 Where Bes<0
Select *  From #R Order By M_Ko, M_Mo, M_Ta1, M_Ta2
```

`Sum(M_Bes-M_Bes)` is always 0, so `BesR` was always 0 — the unit was broken. Note also that the
`Where` clause omits the `M_Ko in (@Kol)` filter, so it aggregated **all** accounts.

Its validations (`FinalU.pas:50-108`) included **two rules `NewFinalu` lost**:

| Check | Persian message | English | Cite |
|---|---|---|---|
| Kol has no balance | `' کد کل مانده ندارد'` | "The general-ledger code has no balance" | `FinalU.pas:61` |
| **Destination is not a leaf** (`is_Sarfasl_Last_Deep`) | `'  سرفصل وارد شده باید در آخرین لایه باشد  '` | "The entered account must be at the last level" | `FinalU.pas:69` |
| Voucher number = 0 | `'   شماره سند را وارد کنید   '` | "Enter the voucher number" | `FinalU.pas:75` |
| **Destination Kol = source Kol** | `'حساب به خودش نمیتواند بسته شود  '` | "An account cannot be closed to itself" | `FinalU.pas:82` |
| Voucher exists → confirm | `'شماره سند تکراری است'` / `'آیا اطلاعات به سند قبلی اضافه شود'` | "The voucher number is duplicate" / "Shall the data be appended to the previous voucher?" | `FinalU.pas:94` |
| Voucher `M_Tx > 0` | `'   سند را در حالت تحریر قرار دهید  '` | "Put the voucher into draft state" | `FinalU.pas:106` |

It also **clamped the voucher date into the fiscal year** (`FinalU.pas:110-112`), which `NewFinalu`
does only at `init` time (`NewFinalu.pas:276-277`):

```pascal
     _D := DM.Current_Date;
     if _D > DM.To_Date then _D := DM.To_Date;
     if _D < dm.From_Date then _D := Dm.From_Date;
```

**Port the leaf-destination check and the "cannot close to itself" check into the rebuild** — they are
correct rules that were lost.

### 9.5 `BastanHesab.pas` — balance export, not a close

Menu: `B_CloseMoein` (`Mainu.pas:453-456`). Despite the name (بستن حساب = "closing the account"), this
**writes two files and changes nothing in the database**.

```pascal
// BastanHesab.pas:41-83
     SP1.Parameters.ParamByName('@COID').Value := DM.CO_ID;
     SP1.Open;                                          // stored proc Moein_All

     F1 := Tmyini.Create('D:\Bed.GGS');
     F2 := Tmyini.Create('D:\Bes.GGS');
     F1.WriteString('Base', 'Program', 'GreenGold');
     F1.WriteString('Base', 'Name', 'Sanad');
     ...
     for I := 1 to Sp1.RecordCount do
     Begin
        if Sp1.FieldByName('M_Bed').AsString >'0'  then      // <<< STRING comparison
         Begin
            J:=J+1;  S:= 'Line'+inttostr(J);
            F1.WriteString(S, 'Kol',   Sp1['M_Ko'] );
            F1.WriteString(S, 'Moein', Sp1['M_Mo'] );
            F1.WriteString(S, 'Taf1',  Sp1['M_Ta1'] );
            F1.WriteString(S, 'Taf2',  Sp1['M_Ta2'] );
            F1.WriteString(S, 'Mab',   Sp1['M_Bed'] );
         End Else Begin
            K:=K+1;  S:= 'Line'+inttostr(K);
            F2.WriteString(S, ... 'Mab', Sp1['M_Bes'] );
         End;
        Sp1.Next;
     End;
     F1.WriteInteger('Base', 'Size', J);
     F2.WriteInteger('Base', 'Size', K);
```

Output: `D:\Bed.GGS` (debit-balance accounts) and `D:\Bes.GGS` (credit-balance accounts), in the `.GGS`
format of §5.9. Message: `'فایل خروجی در مسیر d:\‌ساخته شد'` ("The output file was created in the path
d:\") — `BastanHesab.pas:81`. Note the embedded zero-width non-joiner (U+200C) before `ساخته`.

**Two defects:**
1. `Sp1.FieldByName('M_Bed').AsString > '0'` is a **string** comparison. `'10' > '0'` is true and
   `'0' > '0'` is false, so it happens to work for non-negative decimal strings. Fragile.
2. The output path `D:\` is hard-coded, and the `'Size'` key is written twice
   (`BastanHesab.pas:51-52` writes 1, then `:77-78` overwrites with the real count).

Purpose in practice: hand the two files to `SanadMoeinu.InFileClick` (§9.6) or `SanadEditU.Import`
(§5.9) to build an opening voucher in another database. A poor-man's inter-database carry-forward,
superseded by `EnteghalU`.

### 9.6 `SanadMoeinu.InFileClick` — import a `.GGS` file into a voucher

(`SanadMoeinu.pas:281-336`) Precondition: `SanadState > 0` → `'سند در حالت تحرير نيست'`
("The voucher is not in draft state") — `SanadMoeinu.pas:289`.

Reads the file section by section and appends `Moein` rows directly. The user chooses on the `InFileF`
dialog whether the amounts go to the debit or the credit side:

```pascal
// SanadMoeinu.pas:314-319
       dm.Moein.FieldByName('M_Bed').AsString := '0';
       dm.Moein.FieldByName('M_Bes').AsString := '0';
       if infilef.RBed.Checked
            then  dm.Moein.FieldByName('M_Bed').AsString := mab
            else  dm.Moein.FieldByName('M_Bes').AsString := mab;
```

All imported lines get `M_Kind = 1`, `M_Tx = 0`, `M_ID = 0`, `M_Code = 0`, `M_CoID = Dm.CO_ID`, and a
single shared description from `infilef.Desc.Text`.

**Note `M_Code = 0`** — imported lines have no account id, only the tuple. See §14.

### 9.7 `SanadMoeinu.fillsanad` — fill a voucher with reversed balances

Popup menu items `N3` (`fillsanad(1)`) and `N5` (`fillsanad(2)`) on the legacy voucher screen
(`SanadMoeinu.pas:429-436`). A manual carry-forward helper.

```sql
-- SanadMoeinu.pas:446-450
 Select M_Ko, M_Mo, M_Ta1, M_Ta2
      , Sum(M_Bed-M_Bes) As BedR , Sum( M_Bes-M_Bed) As BesR
 From Moein
 Where M_kind=1 and M_Coid=<CO_ID>
 Group By M_Ko, M_Mo, M_Ta1, M_Ta2
```

Then (`SanadMoeinu.pas:454-486`), for each leaf account, clamp negatives to zero and:

```pascal
        if ( (Bed1Bes2=1) and (NBed>0) ) or ( (Bed1Bes2=2) and (Nbes>0) ) then
        Begin
          Dm.Moein.Append;
          ...
          dm.Moein.FieldByName('M_Bed').AsString := SBes;   -- NOTE: swapped
          dm.Moein.FieldByName('M_Bes').AsString := SBed;
          dm.Moein.FieldByName('Article').AsString := 'انتقال مانده به سال بعد';  -- "carrying the balance to the next year"
          ...
        End;
```

`fillsanad(1)` emits reversing entries for accounts with a **net debit**; `fillsanad(2)` for accounts
with a **net credit**. Debit and credit are swapped on write, so the entries zero out the accounts. No
contra account is generated — the user must add it manually, which is why the two directions are
separate menu items.

---

_Prev: [03-09-b-period-close-and-year-end](03-09-b-period-close-and-year-end.md) | Next: [03-10-aggregation-consolidation-tajmiu-pas](03-10-aggregation-consolidation-tajmiu-pas.md)_
