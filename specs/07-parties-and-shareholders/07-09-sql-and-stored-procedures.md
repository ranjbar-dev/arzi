_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 9. SQL and stored procedures — verbatim catalogue

### 9.1 `Sarfasl_ADD` (stored procedure)

**Called from:** `SahamdarEditU.pas:338-339`, `CompanyEditU.pas:309-310` (ad-hoc `Exec`), and
`SNewu.pas:630-641` (typed `TADOStoredProc`, `SNewu.dfm:908-956`).

```pascal
// SahamdarEditU.pas:338-339
   Q1.SQL.Add('Exec Sarfasl_Add  '+ inttostr(_Ko)+ ', '+ inttostr(_Mo)+ ', '+
               inttostr(_Ta1)+ ', '+ inttostr(_Ta2)+ ', '+ QuotedStr(_Name) );
```

**Signature** (from `SNewu.dfm:911-953`):

| Parameter | Type | Meaning |
|---|---|---|
| `@RETURN_VALUE` | int, return | unused by callers |
| `@Ko` | int | Kol code (default in DFM: `514`) |
| `@Mo` | int | Moein code |
| `@Ta1` | int | Tafsil-1 code — **the party card number** when called from the party editors |
| `@Ta2` | int | Tafsil-2 code — always `0` from the party editors |
| `@Name` | varchar(100) | Account display name |

**Result set:** `_Error` (int; `>0` = failure) and `_Desc` (message text), consumed at
`SNewu.pas:640-643`. Called with `ExecSQL` (result discarded) from the party editors.

**Intent:** idempotently create the chart-of-accounts node, maintain `S_Child` on ancestors, and
report a human-readable error. **Body not present in the repository** — see §12-Q19.

### 9.2 `Sahamdar_Seek` (stored procedure)

`Dmu.dfm:150-172`, `Dmu.pas:18`.

| Parameter | Type | Meaning |
|---|---|---|
| `@RETURN_VALUE` | int, return | — |
| `@S_card` | int | Card number to look up |

Returns a `Sahamdar` row set. Wrapper:

```pascal
// Dmu.pas:1372-1383
function TDM.Sahamdar_Seek_Card(Card: integer): Boolean;
begin
    Sahamdar_Seek.Close;
    if Card <=0 then
    Begin
       Result := False;
       Exit;
    End;
    Sahamdar_Seek.Parameters.ParamByName('@S_Card').Value := Card ;
    Sahamdar_Seek.Open;
    Result := Sahamdar_Seek.RecordCount > 0 ;
end;
```

`Sahamdar_Seek_Card` is itself never called; `Sahamdar_Seek` is used only from the dead
`SahamdarP.pas:165-188`, which reads `S_Name, S_Famil, S_Father, S_BDate, S_BPlace, S_SDate,
S_SPlace, S_IDNO, S_Address, S_CodeMelli, S_CodePosti, S_Mobile, S_Phone, S_Siba, S_kind`.

### 9.3 `Sahamdar_Edit` (stored procedure)

`Dmu.dfm:173-300`, `Dmu.pas:19`. Full parameter list:

| Parameter | Type | Size |
|---|---|---|
| `@RETURN_VALUE` | int (return) | |
| `@Card` | int | |
| `@kind` | word (tinyint) | |
| `@Name` | string | 50 |
| `@Famil` | string | 50 |
| `@Father` | string | 50 |
| `@BDate` | string | 8 |
| `@BPlace` | string | 20 |
| `@SDate` | string | 8 |
| `@SPlace` | string | 20 |
| `@IDNO` | int | |
| `@Address` | string | 100 |
| `@CodeMelli` | string | 10 |
| `@CodePosti` | string | 10 |
| `@Mobile` | string | 12 |
| `@Phone` | string | 12 |
| `@Siba` | string | 13 (default `' '`) |

Only caller is `SahamdarP.pas:133-153`, which is unreachable (§4.6). **Superseded by the inline
INSERT/UPDATE in `SahamdarEditU`/`CompanyEditU`; do not port.**

> These sizes are the best available evidence for the `Sahamdar` column widths, since no DDL is in
> the repo. Note `@BDate`/`@SDate` are 8 chars (`YY/MM/DD`) while the live editors write up to 10
> (`YYYY/MM/DD`, cf. `Dmu.pas:887-888` accepting both). §12-Q20.

### 9.4 `Sahamdar_Show` (stored procedure)

`Dmu.dfm:568-590`, `Dmu.pas:37`. One parameter `@Id : int`. **Zero call sites** in the entire
project. Purpose unknown — plausibly the vestigial equity/shareholding report. §12-Q21.

### 9.5 `SahamdarConfig` query

See §7.4 for all three variants, quoted verbatim.

### 9.6 `Jari_Rem` query

See §6.2, quoted verbatim from `Dmu.dfm:8658-8688`.

### 9.7 `QList` query (party's account set)

See §6.4, quoted verbatim from `CardJariU.dfm:6503-6527`.

### 9.8 `SahamdarInfo` query

See §6.6, quoted verbatim from `SahamdarInfoU.dfm:271-278`.

### 9.9 Party register list query

```sql
-- SahamdarU.dfm:462-466  (Q1, and the identical QS at :504-508)
Select *
From Sahamdar
Where S_kind = :HaHo
order by S_Card
```
`:HaHo` = 1 (`حقيقي` natural) or 2 (`حقوقي` legal).

### 9.10 Person-existence probe used by the current-account screen

```sql
-- CardJariU.pas:286
Select * From Sahamdar where S_card=<card>
```

### 9.11 External share-register probe

```sql
-- CardJariU.pas:308-309
 Select * From <Saham_DB>.NSaham
 Where N_Card=<card>
```

### 9.12 Uniqueness probes in the editors

```sql
-- SahamdarEditU.pas:251 / CompanyEditU.pas:223
Select * from sahamdar Where S_Card=<card>
```
```sql
-- SahamdarEditU.pas:263 / CompanyEditU.pas:235
Select * From Sahamdar Where S_CodeMelli=<quoted national id>
```

### 9.13 Load-one-party probe

```sql
-- SahamdarEditU.pas:118 / CompanyEditU.pas:114
Select * From Sahamdar Where S_Card=<card>
```

### 9.14 Account-existence probe (tick recomputation)

```sql
-- SahamdarEditU.pas:170 / CompanyEditU.pas:158
Select * From Sarfasl Where S_Ko=:KO and S_Mo=:Mo and S_Ta1=:Ta1 and S_Ta2=:Ta2
```

### 9.15 Chart-of-accounts pickers

```sql
-- SelectSarfasl.pas:93-95
 Select Code=S_Ko , Sarfasl.* from Sarfasl
 Where S_Mo=0
 Order By S_Ko
```
```sql
-- SelectSarfasl.pas:110-112
 Select Code=S_Mo , Sarfasl.* from Sarfasl
 Where S_ko=<Ko> and S_Mo>0 and S_Ta1=0
 Order By S_Mo
```
```sql
-- SelectSarfasl.pas:127-129
 Select Code=S_Ta1 , Sarfasl.* from Sarfasl
 Where S_ko=<Ko> and S_Mo=<Mo> and S_ta1>0 and S_Ta2=0
 Order By S_Ta1
```
```sql
-- SelectSarfasl.pas:144-146   (note the missing space before "and S_Ta2>0")
 Select Code=S_Ta2 , Sarfasl.* from Sarfasl
 Where S_ko=<Ko> and S_Mo=<Mo> and S_ta1=<Ta1>and S_Ta2>0
 Order By S_Ta2
```

### 9.16 Fiscal-year queries

```sql
-- ChangesU.pas:65 ; CardJariU.dfm:6541 ; Dmu.dfm:367-372 (with CO_DESC)
 Select * From Base Order By Co_ID
```

### 9.17 Rollover queries

`EnteghalU.pas` `QS` probes (all built by concatenation):

```sql
Select * From Base   Where co_id=<CO_ID+1>                                   -- :~97
Select * From moein  Where M_tx <2 and M_coid=<CO_ID>                        -- :~106
Select * From moein  Where M_Sanad=<Sanad1> and M_coid=<CO_ID>               -- :~121
Select * From moein  Where M_Sanad=<Sanad2> and M_coid=<CO_ID+1>             -- :~138
Select * From Base   Where co_id=<CO_ID>                                     -- :~155
Select * From Base   Where co_id=<CO_ID+1>                                   -- :~172
```
Plus the four `insert moein (…)` statements quoted in §1.6, and `Q1` (the carry-set query,
parameterised on `Coid`, defined in `EnteghalU.dfm`).

### 9.18 Party-account lock probes

```sql
-- Dmu.pas:929-930, 939-940, 949-950, 959-960
 Select * From sarfasl Where S_Ko=<ko> and S_Mo=<mo> and S_ta1=<ta1> and S_Ta2=<ta2>
```
```sql
-- Dmu.pas:975
Select * From Sahamdar Where S_Card=<jari>
```
```sql
-- Dmu.pas:990
Select * From DMoein Where DM_sanad=<sanad> and DM_coid=<coid>
```

### 9.19 Party↔account reverse resolution

```sql
-- Dmu.pas:1393
Select * From Sarfasl Where S_SSN=<ssn>
```
```sql
-- Dmu.pas:1412-1413
Select * From SahamdarConfig Where SC_K=<k> and SC_M=<m> and SC_T=<t1>
```
```sql
-- Dmu.pas:1423
 Select * From Sahamdar Where S_Card=<jari>
```
```sql
-- SahamdarInfoU.pas:108
 Select * From SahamdarConfig Where SC_K=%d and SC_M=%d and SC_T=%d
```

### 9.20 Party register kind flip

```sql
-- SahamdarU.pas:149
UpDate Sahamdar Set S_Kind=2 Where S_Card=<card>
```
```sql
-- SahamdarU.pas:170
UpDate Sahamdar Set S_Kind=1 Where S_Card=<card>
```

---


---

[← Previous](07-08-accounting-integration.md) · [Index](00-index.md) · [Next →](07-10-screen-by-screen-ui-specification.md)
