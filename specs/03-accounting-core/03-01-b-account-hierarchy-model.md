_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 1.6 Leaf-ness: `S_Child` and the postability rule

`S_Child` is the **denormalised count of direct children**. It is the single mechanism by which the
system decides whether an account may be posted to.

It is recomputed wholesale by `Dm.Update_Sarfasl_Child` (`Dmu.pas:300-318`):

```sql
-- Dmu.pas:303-314, verbatim (statements concatenated by the Delphi code)
Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D
  Where sarfasl.S_Ko=D.S_Ko and D.S_Mo>0 and S_Ta1=0) Where S_Ko>0 and S_Mo=0

Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D
  Where sarfasl.S_Ko=D.S_Ko and Sarfasl.S_MO=D.S_Mo and D.S_Ta1>0 and S_Ta2=0)
   Where S_Ko>0 and S_Mo>0 and S_Ta1=0

Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D
  Where sarfasl.S_Ko=D.S_Ko and Sarfasl.S_MO=D.S_Mo and Sarfasl.S_Ta1=D.S_Ta1 and D.S_Ta2>0 )
   Where S_Ko>0 and S_Mo>0 and S_Ta1>0 and S_Ta2=0

Update sarfasl Set S_Child = 0 where S_Ta2>0
```

Explanation, statement by statement:
1. For every Kol node, count its Moein children (rows sharing `S_Ko`, having `S_Mo>0` and `S_Ta1=0`).
2. For every Moein node, count its Tafsil-1 children.
3. For every Tafsil-1 node, count its Tafsil-2 children.
4. Tafsil-2 nodes are always leaves.

**Note the correlated-subquery bug:** the `S_Ta1=0` / `S_Ta2=0` predicates inside the subqueries are
unqualified and therefore bind to the *outer* `sarfasl` row, not to the alias `D`. Statement 1 counts
rows where the *outer* row has `S_Ta1=0`, which is always true given the outer `WHERE`, so it happens
to work. Statements 2 and 3 are similarly saved by their outer `WHERE` clauses. The result is correct
but only by coincidence — do not transliterate this SQL.

`Update_Sarfasl_Child` is invoked from exactly one place: after deleting an account
(`SNewu.pas:192`). It is **not** invoked after `Sarfasl_ADD` (the stored procedure presumably
maintains `S_Child` itself — unverified, see §14).

**The postability rule** — an account may be used on a voucher line **only if it has no children**:

```pascal
// EditArticleMoeinU.pas:129-141
function TEditArticleMoein.Get_SSn: integer;
begin
   Result:=0;
   Qs.SQL.Add(' Select * From Sarfasl Where S_Ko='+ inttostr(EKo.Tag) );
   Qs.SQL.Add('  and S_Mo='+ inttostr(EMo.Tag) );
   Qs.SQL.Add('  and S_Ta1='+ inttostr(ETa1.Tag) );
   Qs.SQL.Add('  and S_Ta2='+ inttostr(ETa2.Tag) );
   QS.Open;
   if QS.FieldByName('S_Child').AsInteger > 0  then exit;   // <-- leaf check
   Result := Qs.FieldByName('S_SSN').AsInteger;
end;
```

A non-leaf returns `SSN = 0`, which the caller reports as an invalid account code
(`EditArticleMoeinU.pas:366-371`). The account pickers apply the same filter server-side:
`Sarfasl_SelectU.pas:207` (`Where S_Mo> 0 and S_Child=0`), `Sarfasl_SelectU.pas:86`,
`Sarfasl_SelectU.pas:97`, `Sarfasl_SelectU.pas:110`.

`TarafU` implements the same rule incrementally: `F_Valid` is set to 1 (and `F_SSN` captured) only
when the currently-entered level has `S_Child = 0` — `TarafU.pas:391-395` (Moein),
`TarafU.pas:425-429` (Tafsil-1), `TarafU.pas:458-462` (Tafsil-2). Note that a **Kol node is never
directly postable** in `TarafU` — `EKoChange` (`TarafU.pas:341-365`) never sets `F_Valid`. Journal
(Rooznameh) lines are the exception; they post at Kol level via a different form (§8).

An **alternative, equivalent** leaf test exists on the data module and is used by the dead `FinalU`:

```pascal
// Dmu.pas:1017-1033
function TDM.is_Sarfasl_Last_Deep(Ko, Mo, Ta1, Ta2 : integer): Boolean;
   ...
   Q1.SQL.Add(' Select * From sarfasl Where s_Ko='+ inttostr(ko) );
   if mo>0 then Q1.SQL.Add( ' And s_mo='+ inttostr(mo) );
   if ta1>0 then Q1.SQL.Add( ' And s_ta1='+ inttostr(ta1) );
   if ta2>0 then Q1.SQL.Add( ' And s_ta2='+ inttostr(ta2) );
   Q1.Open;
   result := Q1.RecordCount = 1;
```

i.e. "the code prefix matches exactly one row" ⇒ it is a leaf. `Dmu.pas:1035-1064`
(`is_Sarfasl_Last_Deep_SSN`) is the by-id variant.

### 1.7 Company / fiscal-year scoping of the chart of accounts

**The chart of accounts is global, not per fiscal year.** This is the single most important structural
finding in this document.

Evidence:
- `Sarfasl` has an `S_COID` column, but **no live query filters on it**. Every account query in
  `SNewu.pas`, `SelectSarfasl.pas`, `Sarfasl_SelectU.pas`, `EditArticleMoeinU.pas`, `TarafU.pas`,
  `TajmiU.pas`, `EnteghalU.pas` and `NewFinalu.pas` omits `S_COID` entirely.
- The only two references are in dead or disabled code: `Sarfasl_ListU.pas:44` (unit compiled but
  the form is only reachable from nowhere) and `MakeNewU.pas:129-150`, where the block that *would*
  have copied the chart of accounts into a new fiscal year is **commented out**.
- `MakeNewU.sBitBtn1Click` (`MakeNewU.pas:97-154`) creates a new fiscal year by cloning only the
  `Base` row. The chart is shared.

Consequently: `Base.CO_ID` is a **fiscal year identifier**, and `Sarfasl` rows are shared across all
years. `Moein` and `DMoein` are scoped by `M_COID` / `DM_Coid`.

**Rebuild decision required.** Preserving this as-is means `accounts` has no `company_id`/`fiscal_year_id`.
That is almost certainly the intent (a stable chart across years), but the vestigial `S_COID` column
suggests the original author considered per-year charts. See §15.

### 1.8 Account types / natures (asset / liability / …)

**There is no account-type or account-nature classification in this system.**

The task brief asked for `Kind_Table`. `Kind_Table` exists on the data module (`Dmu.pas:20`) and maps
to `TableName = 'Kinds'` (`Dmu.dfm:301-304`), but it is **not an account-type table**. Its only
consumer is `PestehD_U.pas:94-95`, the pistachio grading screen, where it supplies product grades.
Grep confirms zero other references.

`Sarfasl.S_Kind` exists in the schema (`FactorPrint3U.dfm:3316`) and is rendered as a grid column in
the dead unit `S_KolU.dfm:207`, but **no code anywhere writes it**. (The `S_Kind` writes at
`SahamdarU.pas:149` and `SahamdarU.pas:170` target the `Sahamdar` table, a different table with a
same-named column, used to mark shareholder vs. non-shareholder.)

Account nature is expressed **implicitly, by Kol number range**. The system relies on convention:
- `103-1` and `104-1`, `104-2`, `303-1`, `303-3` are the person current-account subtrees
  (`Sarfasl_SelectU.pas:108-110`).
- `109-1`, `109-2` are another special group (`Sarfasl_SelectU.pas:97`).
- Which Kol accounts are income-statement accounts is a **runtime user choice** on the closing screen
  (`NewFinalu.pas`, §9.2) — the user ticks them by hand each year.

The debit/credit natural side is likewise never declared; balances are computed as signed sums and
clamped (`Sum(M_Bed-M_Bes)` and `Sum(M_Bes-M_Bed)`, then negatives set to zero — see §9).

**Rebuild recommendation:** this is a genuine functional gap, not a design choice worth preserving.
See §15.

### 1.9 Special-role account registry: `base_config`

Because there is no account-type column, accounts that play a *system role* are registered in a
side table `base_config` (proposed: **`account_roles`**).

```pascal
// SNewu.pas:692-735 -- Add_Tanzim
   if ID=11 then S:= 'صدور چک نقدی';        // "issue cash cheque"
   if ID=12 then S:= 'صدور چک موعدی';       // "issue post-dated cheque"
   if ID=13 then S:= 'اسناد پرداختنی';      // "notes payable"
   if ID=14 then S:= 'اسناد دریافتنی در صندوق';  // "notes receivable in cash box"
   if ID=15 then S:= 'اسناد در جریان وصول'; // "notes in course of collection"
   SSN := Q1.FieldByName('S_SSN').AsInteger;
   ...
   Q2.SQL.Add('  Select * From base_config Where BC_ID= '+ inttostr(ID) );
   Q2.SQL.Add('   and BC_SSN='+ inttostr(SSN) );
   Q2.Open;
   if Q2.RecordCount>0 then
   begin
     MessageDlg('  قبلا در لیست ثبت شده است  ', mterror, [mbok], 0 );  // "already registered in the list"
     exit;
   end;
   ...
   Q2.SQL.Add('  insert base_config (BC_ID, BC_SSN, BC_Name, BC_Enabled, BC_Default )');
   Q2.SQL.Add('  Values( '+ inttostr(ID)+', '+ inttostr(SSN)+ ', '+ QuotedStr(S)+ ', 1, 0) ' );
```

Guards, in order (`SNewu.pas:698-707`):
1. Current level is Kol (`State = 1`) → `'در سطح کل مورد تایید نمیباشد'` ("not accepted at the general-ledger level").
2. Node has children (`SNo > 0`) → `' سطح آخر حساب نیست'` ("this is not the last account level").
3. Already registered for this `BC_ID` → `'  قبلا در لیست ثبت شده است  '`.

Known `BC_ID` values and their readers:

| `BC_ID` | Persian role | English | Reader |
|---|---|---|---|
| 11 | صدور چک نقدی | Bank account for cash-cheque issue | `Sarfasl_SelectU.pas:264-280` (`init_Bank`) |
| 12 | صدور چک موعدی | Post-dated cheque issue | (registered; reader in treasury module) |
| 13 | اسناد پرداختنی | Notes payable | `Sarfasl_SelectU.pas:246-262` (`init_AsnadParDakhti`) |
| 14 | اسناد دریافتنی در صندوق | Notes receivable on hand | `Sarfasl_SelectU.pas:233-244` |
| 15 | اسناد در جریان وصول | Notes in course of collection | `Sarfasl_SelectU.pas:220-231` |

Note `init_Bank` and `init_AsnadParDakhti` auto-select when exactly one row matches
(`Sarfasl_SelectU.pas:256-259`, `Sarfasl_SelectU.pas:274-277`).

### 1.10 Two more single-account settings on `Base`

Independently of `base_config`, two accounts are configured directly on the fiscal-year row:

| `Base` column | Label (`TanzimU.dfm:741`, `TanzimU.dfm:724`) | English | Accessor |
|---|---|---|---|
| `C1081` (id) / `C1081C` (code string) | اسناد نزد صندوق | Notes on hand in the cash box | `Dm.SanDoogh_k` / `Dm.SanDoogh_M` / `Dm.Sandoogh_KM` (`Dmu.pas:1067-1100`) |
| `C1082` (id) / `C1082C` (code string) | اسناد در جریان وصول | Notes in course of collection | `Dm.Jaryan_K` / `Dm.Jaryan_M` / `Dm.Jaryan_KM` (`Dmu.pas:1102-1135`) |

Saved by `TanzimU.pas:283-292`. `C1081`/`C1082` hold `Sarfasl.S_SSN`; `C1081C`/`C1082C` hold the
typed code string for display. The accessors resolve the id back to `(S_Ko, S_Mo)`.

This duplicates `base_config` IDs 14 and 15. Both mechanisms are live. See §15.

### 1.11 Party linkage: `SahamdarConfig`

`Dm.Get_Jari_Code` (`Dmu.pas:1385-1441`) determines whether an account is a *person current account*
and, if so, returns that person's identity string:

```pascal
// Dmu.pas:1402-1413
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
```

Algorithm: strip the **deepest non-zero code component** off the account and call it `Jari` (the
party card number). Then check whether the remaining prefix `(K, M, T1)` is registered in
`SahamdarConfig` as a party-account subtree. If yes, look up `Sahamdar.S_Card = Jari` and return
national ID / postal code / mobile.

So: **the leaf code number of a party account *is* the party's card number.** This is a hard coupling
between the account tree and the party master. It is documented in the parties module; recorded here
because the accounts table cannot be understood without it.

---

_Prev: [03-01-a-account-hierarchy-model](03-01-a-account-hierarchy-model.md) | Next: [03-02-a-account-crud-rules](03-02-a-account-crud-rules.md)_
