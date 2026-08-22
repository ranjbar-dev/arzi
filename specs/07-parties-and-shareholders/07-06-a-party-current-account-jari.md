_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 6. Party current account (Jari)

### 6.1 Concept

A party's *current account* (`جاری`) is not one account. It is the **set** of chart-of-accounts leaf
nodes derived from `SahamdarConfig` for that party's card number. Its balance is the net of that set.

### 6.2 `Jari_Rem` — the balance query, verbatim

`Dmu.dfm:8640-8691`. Two parameters: `Jari` (card number) and `Sal` (fiscal year = `CO_ID`).

```sql
-- Dmu.dfm:8658-8688
Declare @Jari int Set @Jari=:Jari
Declare @Sal int Set @Sal=:Sal

   if OBJECT_ID('tempdb..#R') is not null Drop Table #R

   Select  SC_K, SC_M, SC_T as SC_T1, 0 As SC_T2  ,  Cast(0 as Bigint) As Bed, Cast(0 as Bigint) As Bes,  Cast(0 as Bigint) As Rem
   into #R
   From SahamdarConfig
   Where SC_Rem = 1

   Update #R Set SC_T2=@Jari Where SC_T2=0 and SC_T1>0
   Update #R Set SC_T1=@Jari Where SC_T1=0

   Update #R Set Bes = isnull( ( Select Sum(M_Bes) From Moein Where M_Ko=#R.SC_K and M_Mo=#R.SC_M and M_Ta1=#R.SC_T1 and M_Ta2=#R.SC_T2 and M_Coid=@Sal) , 0)
   Update #R Set Bed = isNull( ( Select Sum(M_Bed) From Moein Where M_Ko=#R.SC_K and M_Mo=#R.SC_M and M_Ta1=#R.SC_T1 and M_Ta2=#R.SC_T2 and M_Coid=@Sal) , 0)
   Update #R Set Rem = Bes-Bed
   Delete #R where Bed=0 and Bes=0

   Select isnull( Sum(Rem) , 0) as Remind From #R
```

**Step-by-step semantics:**

1. Take every `SahamdarConfig` row flagged `SC_Rem = 1` — the set of control accounts that
   participate in the current-account balance.
2. Slot the card number: if `SC_T > 0` the card goes into **Tafsil-2** under fixed Tafsil-1 `SC_T`;
   if `SC_T = 0` the card goes into **Tafsil-1** and Tafsil-2 stays `0`.
3. For each resulting coordinate, sum `M_Bes` (credit) and `M_Bed` (debit) from `Moein`,
   restricted to the requested fiscal year.
4. `Rem = Bes − Bed`, i.e. **credit-positive**.
5. Drop untouched accounts (both sums zero) — cosmetic, does not change the total.
6. Return `Remind = Σ Rem`.

**Sign convention:** `Remind > 0` ⇒ the entity **owes** the party (party is a creditor).
`Remind < 0` ⇒ the party owes the entity (party is a debtor). This is confirmed by the caller:

```pascal
// CardJariU.pas:350-360
      Dm.Jari_Rem.Close;
      Dm.Jari_Rem.Parameters.ParamByName('Jari').Value := S_Card.IntValue;
      Dm.Jari_Rem.Parameters.ParamByName('Sal').Value :=  COID.KeyValue; //Dm.CO_ID;
      Dm.Jari_Rem.Open;
      _Bed:=0;
      if Dm.Jari_Rem.RecordCount>0 then
         _Bed := Dm.Jari_Rem.FieldByName('Remind').AsVariant;
      S_Rem.IntValue := _Bed;
      T_REm.Caption :='';
      if _Bed <0 then T_Rem.Caption := 'بدهکار' ;
```
`بدهکار` = "debtor". Note the balance is computed for the **fiscal year chosen in the form's own
combo** (`COID.KeyValue`), not necessarily the ambient `Dm.CO_ID` — the form lets the user inspect
any year (`CardJariU.pas:260-264`).

`Jari_Rem` is prepared once at startup (`Dmu.pas:751-752`):

```pascal
   Jari_Rem.Close;
   Jari_Rem.ConnectionString := Ado.ConnectionString;
```

### 6.3 Worked arithmetic

Assume `SahamdarConfig` contains these `SC_Rem = 1` rows (the natural-person seed set, §7.3):

| `SC_K` | `SC_M` | `SC_T` | Meaning |
|---|---|---|---|
| 103 | 1 | 0 | Trade accounts receivable — persons |
| 104 | 1 | 0 | Accounts receivable — personnel |
| 301 | 1 | 0 | Trade accounts payable — persons |

Party card `@Jari = 52506`, fiscal year `@Sal = 1397`.

Step 2 produces the coordinates (all `SC_T = 0`, so the card lands in Tafsil-1):

```
(103, 1, 52506, 0)
(104, 1, 52506, 0)
(301, 1, 52506, 0)
```

Suppose `Moein` for `M_Coid = 1397` holds:

| Coordinate | Σ `M_Bed` (debit) | Σ `M_Bes` (credit) |
|---|---|---|
| `(103,1,52506,0)` | 50,000,000 | 12,000,000 |
| `(104,1,52506,0)` | 0 | 0 |
| `(301,1,52506,0)` | 3,000,000 | 20,000,000 |

Step 4:

```
Rem(103-1) = 12,000,000 − 50,000,000 = −38,000,000
Rem(104-1) =          0 −          0 =           0
Rem(301-1) = 20,000,000 −  3,000,000 = +17,000,000
```

Step 5 deletes `(104,1,…)` (both sums zero).

Step 6:

```
Remind = (−38,000,000) + (+17,000,000) = −21,000,000
```

Result: `S_Rem.IntValue = −21,000,000` and `T_Rem.Caption = 'بدهکار'` — the party owes 21,000,000
(Rial) net.

**Second worked case, exercising the Tafsil-2 slot.** Add a config row `SC_K=103, SC_M=9, SC_T=77,
SC_Rem=1`. Step 2's first `UPDATE` fires (`SC_T2=0 and SC_T1>0`), producing the coordinate
`(103, 9, 77, 52506)`. If that account has Σ`M_Bed` = 0 and Σ`M_Bes` = 5,000,000 then
`Rem = +5,000,000` and the grand total becomes `−16,000,000`.

> **This second case is currently unreachable** because `Sarfasl_Add` never creates a node with a
> non-zero Tafsil-2 for a party (§2.4). The SQL supports it; the write path does not. §12-Q1.


---

[← Previous](07-05-shareholder-equity-profit-distribution.md) · [Index](00-index.md) · [Next →](07-06-b-party-current-account-jari.md)
