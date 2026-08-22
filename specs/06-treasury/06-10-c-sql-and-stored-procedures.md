_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

### 10.8 The draft-state guards

```sql
 Select isnull( max(M_Tx),0)  as _TX From moein
    Where M_Id=26 and M_link=<SSN> and M_Coid=<CO_ID>     -- CheckEditU.pas:360-361
    -- and M_Id=41 …                                       -- TankhahEdit.pas:339-340
```

```sql
Select Max(DM_TX) as TX from Dmoein Where DM_Sanad=<N> and DM_Coid=<CO_ID>
    -- CheckListU.pas:168-169, :218-219; TankhahList.pas:133-134, :190-191
```

```sql
 Select isnull( Max(M_Tx),0) as TX From Moein
   Where M_Coid=<CO_ID>
   and M_sanad=<N>                                        -- Dmu.pas:1544-1546 (Get_SanadMaxTX)
```

Three different formulations of "is this voucher still editable": one over `Moein` scoped by
document (`M_Id`+`M_Link`), one over `DMoein` scoped by voucher, one over `Moein` scoped by voucher.
They can disagree — `DMoein.DM_TX` is the header's cached state and, like `DM_Mab`, is
drift-prone.

### 10.9 The list queries

```sql
 Select * From DCheck
 Where 1=1
 -- And S_State=<n>                                        (unreachable, CheckListDU.pas:325)
 -- And ( ( S_BesCR Like '%<name>%' ) Or ( S_BesName Like '%<name>%' ) )   (unreachable, :326)
 -- And ( S_DateS<= '<date>' ) and (S_State<4)             (unreachable, :327)
 Order By S_Dates                                          -- CheckListDU.pas:323-329
```

```sql
Select * From DCheck2
    Where S_Link=<S_SSN>
  Order By S_SSN                                           -- CheckListDU.pas:162-164
```

```sql
 Select * From DFish
 Where 1=1
 Order By S_Dates                                          -- FishListD.pas:225-231
```

```sql
Select * From CheckMaster Where CM_Coid=:Coid              -- CheckListU.dfm:517
Select * From TankhahMaster Where TM_Coid=:Coid Order By TM_Date  -- TankhahList.dfm:448
```

The `Like '%…%'` search (had it been reachable) is unindexable and would scan; and note `DCheck` and
`DFish` are queried **without a `S_COID` predicate** (§3.4).

### 10.10 The delete statements

```sql
 Delete From DCheck Where S_SSn= <SSN>
 Delete from moein Where M_Sanad=<Sanad>
 and M_Coid=<CO_ID>and M_ID=21 and M_Link=<SSN>            -- CheckDaryaftU.pas:435-437
```

**Broken**: `inttostr(DM.CO_ID) + 'and M_ID=21'` produces `M_Coid=1403and M_ID=21`. The method is
unreachable anyway (§2.3 T3).

```sql
 Delete From DCheck Where S_SSN=<SSN>
 Delete From moein Where M_Sanad=<Sanad>
    and M_id=21 And M_Coid=<CO_ID>
    and M_link = <SSN>                                     -- CheckListDU.pas:478-482 (dead code)
```

```sql
 Delete From DFish Where S_SSN=<SSN>
 Delete From moein Where M_Sanad=<Sanad>
    and M_id=25 And M_Coid=<CO_ID>
    and M_link = <SSN>                                     -- FishListD.pas:285-289 (live)
```

```sql
 Begin transaction
 Delete CheckMaster Where CM_SSN=<SSN>
 Delete CheckDetail Where CD_CMSSN=<SSN>
 Delete From moein Where M_ID=26 and M_Link=<SSN> and M_Sanad = <Sanad>
 Commit                                                    -- CheckListU.pas:187-191 (live)
```

```sql
  Begin Transaction
  Delete TankhahMaster where TM_SSN=<SSN>
  Delete TankhahDetail where TD_TMSSN=<SSN>
  Delete Moein Where M_ID=41 and M_Coid=<CO_ID> and M_Link=<SSN>
  Commit                                                   -- TankhahList.pas:152-158 (live)
```

**No delete anywhere touches `DCheck2`.** Deleting a cheque (if it were possible) would orphan its
entire event history and every `M_Id ∈ {22,23,24}` posting that history points at.

### 10.11 Cross-module: the invoice settlement view (`Dmu.dfm:1032-1075`)

The only query that reads both treasury tables at once, and the only place a `DCheck`/`DFish` row is
surfaced outside the treasury module:

```sql
Declare @Sal int Set @Sal = :Sal
Declare @Sanad int
Declare @Factor int Set @Factor =:Factor
Declare @Jari int
Set @Jari =  ( Select min(AF_Customer) From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )
set @SAnad = ( Select min(AF_Sanad)    From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )

Select 22 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_FishNo As _Link  , S_Mab As _Mab ,
         S_Desc As _Desc , '' As _DSar
--   From DFish Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DFish Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor

union

Select 21 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_CheckNo As _Link  , S_Mab As _Mab ,
         S_Desc As _Desc , S_DateS as _DSar
--   From DCheck Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DCheck Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor
```

`@Jari` and `@Sanad` are computed and then **not used** — both predicates that referenced them are
commented out. The literals `21` and `22` in the `_ID` column are a **third, incompatible numbering
scheme**: here `21` means "a cheque" and `22` means "a deposit slip", whereas in `Moein.M_Id` `21` is
a cheque *receipt* and `22` is a cheque *deposit/bounce*. Do not conflate them.

### 10.12 Systemic notes

- Every treasury statement runs on an ad-hoc connection string: most screens set
  `Q.ConnectionString := Dm.Ado.ConnectionString` per call rather than sharing `Dm.Ado`, so each
  query opens its own pooled connection and **no two statements in a "transaction" are guaranteed to
  be on the same connection** unless they are in the same batch text.
- `Begin Transaction` / `Commit` appear as literal text inside the batch. There is no `Rollback`
  anywhere in the treasury module and no `TRY…CATCH`, `SET XACT_ABORT`, or error handling of any
  kind. A mid-batch failure leaves an open transaction on that connection until the pool recycles it.
- Amounts are interpolated from `TEditInt.Inttext`, which is the raw digit string. A blank field
  yields an empty string and hence a SQL syntax error rather than a validation message.


---

[← 10. SQL and stored procedures (part b)](06-10-b-sql-and-stored-procedures.md) | [index](00-index.md) | [11. Screen specifications (part a) →](06-11-a-screen-specifications.md)
