_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 10. SQL and stored procedures

### 10.0 There are no treasury stored procedures

`Dmu.dfm` declares thirteen `TADOStoredProc` components; **none of them touches `DCheck`,
`DCheck2`, `DFish`, `CheckMaster`, `CheckDetail`, `TankhahMaster` or `TankhahDetail`**. Every
treasury statement is inline SQL assembled at runtime by `TADOQuery.SQL.Add`.

Two server-side objects are nevertheless referenced:

- **`dbo.Noto3(<bigint>)`** — **body now confirmed** (`Full_Script_14050527.sql`, schema-only dump,
  reviewed in `02-data-model/02-12-a.md` §12.4): it does **not** spell the number in words. It is a
  thousands-separator grouper — `CAST(@INP AS varchar(20))` with commas inserted every 3 digits from
  the right, i.e. `1234567` → `1,234,567`. The "spells a number in Persian words" description above
  was a guess and is **corrected**: that job belongs entirely to `TUtil.Str2String`/`TUtil.No2String`
  (`Utility.pas:483-522`), which really does convert to Persian words. So a voucher narration built
  with `dbo.Noto3` gets **grouped digits** (e.g. `"1,234,567 ریال"`), while a report built with
  `TUtil.No2String` gets **spelled-out words** (e.g. `"یک میلیون و دویست..."`) — these are two
  genuinely different presentations, not two implementations of the same one. No rounding logic
  either way is needed on the `Noto3` side (input is `bigint`, already whole rial). One real quirk:
  `Noto3` has no sign-aware branch, so a negative amount's leading `-` shifts the comma grouping by
  one character versus the same-magnitude positive amount.
- **`MakeSanad_CheckDaryafti <ssn>`** — **confirmed to exist on the server; full body captured.**
  Guards: cheque must exist and be `S_State = 1`; refuses if a draft posting already exists under
  `M_Id=21` (must be finalized first); deletes prior `M_Id=21` lines for the cheque, then inserts
  exactly 2 `Moein` rows (debit `S_BedSSN`, credit `S_ZSSN`) with a narration built from
  `dbo.Noto3(S_Mab)` and `S_DateS` — matching the pattern of the commented-out call, so if this
  procedure were wired back in, its output shape is now known and matches what the commented-out
  code expected.

Quoted below verbatim as assembled, with the concatenation resolved and the `Q.SQL.Add` line numbers
given.

### 10.1 Cheque receipt — load a cheque (`CheckDaryaftU.pas:138`, `:156`)

```sql
Select * From CheckMaster Where CM_SSN=<SSN>          -- CheckEditU.pas:138
Select * From CheckDetail Where CD_CMSSN=<SSN>        -- CheckEditU.pas:156
```

Straight point reads. `CheckDaryaftU` instead uses the shared `Dm.QDCheck` dataset
(`Select * from DCheck`, `Dmu.dfm:907-916`) and locates the row client-side with
`Locate('S_SSN', …)` (`CheckDaryaftU.pas:111-117`) — i.e. **it downloads the entire `DCheck` table to
find one row**, on every open-for-edit. The same pattern appears in `Delete_Check`
(`CheckDaryaftU.pas:415-417`) and in `FISHDaryaftU.Delete_Fish` (`:162-164`).

### 10.2 Cheque receipt — the posting (`CheckDaryaftU.pas:320-348`)

```sql
Declare @SSn int Set @ssn=<new DCheck.S_SSN>
Declare @Coid int set @coid = <CO_ID>
Declare @Sanad int set @Sanad = <S_Sanad>
Declare @SBed varchar(200)
 Set @SBed = ( Select 'بابت دریافت  چک شماره ' + S_CheckNo+ ' مبلغ ' + dbo.Noto3(S_Mab) + ' سررسید '+ S_DateS + ' توسط '+ S_BesName  from Dcheck Where S_SSn=@SSN)
 Begin transaction
 Delete moein where M_link=@ssn and M_id=21

   insert moein ( M_kind, M_Coid, M_Sanad, M_date, M_Bed, M_Bes, M_Ted, M_Tx, Article, M_Ko, M_Mo, M_Ta1, M_Ta2,M_Id, M_Link, M_User, M_Code )
   Select 1, @Coid, @Sanad, DCheck.S_Date, DCheck.S_Mab, 0, 0, 0,@SBed, S.S_Ko,S.S_Mo,S.S_Ta1,S.S_Ta2,  21, DCheck.S_SSN, DCheck.S_UserID, DCheck.S_BedSSN
   from DCheck
   left join sarfasl as s on S.S_SSN=DCheck.S_BedSSN
   Where DCheck.S_SSn=@SSN

   insert moein ( M_kind, M_Coid, M_Sanad, M_date, M_Bed, M_Bes, M_Ted, M_Tx, Article, M_Ko, M_Mo, M_Ta1, M_Ta2,M_Id, M_Link, M_User, M_Code )
   Select 1, @Coid, @Sanad, DCheck.S_Date, 0, DCheck.S_Mab, 0, 0,@SBed, S.S_Ko,S.S_Mo,S.S_Ta1,S.S_Ta2,  21, DCheck.S_SSN, DCheck.S_UserID, DCheck.S_BesSSN
   from DCheck
   left join sarfasl as s on S.S_SSN=DCheck.S_BesSSN
   Where DCheck.S_SSn=@SSN
 commit
```

The cleanest posting in the module: the narration is built server-side from the row itself, both
lines are `INSERT … SELECT` so the amounts and the account hierarchy come from the database rather
than from Delphi variables, and the `Delete` makes it idempotent.

Defects: `Delete moein where M_link=@ssn and M_id=21` **omits `M_Coid`**, so editing a cheque in one
fiscal year deletes the receipt lines of a cheque with the same `S_SSN` in *every* year. (`DCheck`
has one identity sequence across all years, so a collision needs two rows with the same id, which
cannot happen — but the predicate is still wrong, and `Delete_Check` at `:436-437` writes the
`M_Coid` filter as `'…'+ inttostr(DM.CO_ID) + 'and M_ID=21…'` with a **missing space**, producing
`M_Coid=1403and M_ID=21` and a syntax error at runtime.) The two `left join`s silently produce NULL
hierarchy columns if the account id does not resolve.

### 10.3 Cheque deposit to bank (`CheckDaryaft2U.pas:187-224`)

```sql
 Begin Transaction
 Declare @tag int
 Update DCHeck Set S_State=2, S_StateName='چک موعدی در بانک'
     Where S_SSN=<SSN>

 insert DCheck2 ( S_Link, S_Coid, S_Sanad, S_Date, S_Mab, S_State, S_StateName, S_BedSSN, S_BesSSN, S_Desc, S_UserID  )
     Values ( <SSN>, <CO_ID>, <S_Sanad2> , '<S_Date2>', <S_Mab>, 2, 'چک موعدی در بانک' , <S_Bed.Tag>, <S_Bes.Tag>, 'انتقال چک به بانک  <note>', <userId>)
 Commit
 Set @Tag = @@Identity

 Begin Transaction
  Insert Moein ( M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, ArTicle, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2, M_ID, M_Link, M_User, M_Kind, M_Code)
  Values ( <CO_ID>, <S_Sanad2>, '<S_Date2>', <S_Mab>, 0, 0, '<narration>', 0,
     <S_Ko>, <S_Mo>, <S_Ta1>, <S_Ta2>, 22, @tag ,  <userId>, 1 , <S_SSN of the debit account> )

  Insert Moein ( … )
  Values ( <CO_ID>, <S_Sanad2>, '<S_Date2>', 0, <S_Mab>, 0, '<narration>', 0,
     <S_Ko>, <S_Mo>, <S_Ta1>, <S_Ta2>, 22, @tag ,  <userId>, 1 , <S_SSN of the credit account> )
 Commit
```

Three things to note.

1. **`Set @Tag = @@Identity` is executed *after* `Commit`.** It still returns the `DCheck2` identity
   because `@@IDENTITY` is session-scoped, but it is outside the transaction that produced it. Worse,
   `@@IDENTITY` (rather than `SCOPE_IDENTITY()`) picks up the identity of *any* insert the session
   last performed, including one fired by a trigger.
2. **The state change and the posting are in two separate transactions in the same batch.** A failure
   in the second leaves the cheque marked "at bank" with no accounting entry (§8.5 defect 6).
3. **The account hierarchy is string-pasted from the client**: `Dm.Sarfasl_SSN_CODEName(<id>)` is
   called to position the shared `Dm.Sarfasl` dataset, then `Dm.Sarfasl.FieldByName('S_Ko').AsString`
   is concatenated into the text (`CheckDaryaft2U.pas:201-210`). If the lookup fails, the *previous*
   row's values are pasted in silently.

`CheckBargashtu.pas:207-244` and `CheckEsterdadU.pas:186-223` are the same statement with the state,
label, `M_Id` and account assignments changed; `CheckVosoolU.pas:220-256` is the same again minus
`S_BesSSN` from the `DCheck2` column list.

### 10.4 Cheque bounce (`CheckBargashtu.pas:209-215`) — the state/history mismatch

```sql
 Update DCHeck Set S_State=1, S_StateName='چک برگشت شده از بانک'
     Where S_SSN=<SSN>

 insert DCheck2 ( S_Link, S_Coid, S_Sanad, S_Date, S_Mab, S_State, S_StateName, S_BedSSN, S_BesSSN, S_Desc, S_UserID  )
     Values ( <SSN>, <CO_ID>, <S_Sanad2> , '<S_Date2>', <S_Mab>, 2, 'برگشت از بانک' , <S_Bed.Tag>, <S_Bes.Tag>, 'برگشت چک از بانک  <note>', <userId>)
```

The master row goes to `1`; the history row records `2`. See §2.1.

### 10.5 Account resolution used by the bounce/collect screens

```sql
Select * From Sarfasl Where S_SSN=<the cheque's payer account>        -- CheckBargashtu.pas:126
Select * From Sarfasl Where S_ko=<Jaryan_K> and S_mo=<Jaryan_M> and S_ta1=<Ta1> and S_ta2=0   -- :134
Select * From Sarfasl Where S_ko=<Sandoogh_K> and S_mo=<Sandoogh_M> and S_ta1=<Ta1> and S_ta2=0 -- :145
```

The pattern: take the counterparty's **Tafsil-1** segment, then find the treasury control account
that carries the same Tafsil-1 under the configured Kol/Moein pair. `Jaryan_K/M` = "notes in course
of collection" and `Sandoogh_K/M` = "notes on hand", both read from the `Base` settings row
(`Dmu.pas:1080-1135`; see `docs/03-accounting-core.md` §keys `C1081`/`C1082` and
`docs/08-platform-and-security.md`). **No `RecordCount` check follows any of these opens** — if the
per-counterparty control account does not exist, `FieldByName('S_SSN').AsInteger` yields `0` and the
posting is written against account id 0.

`CheckVosoolU.pas:128-138` is the same, resolving only the collection account.


---

[← 9. Validation rules](06-09-validation-rules.md) | [index](00-index.md) | [10. SQL and stored procedures (part b) →](06-10-b-sql-and-stored-procedures.md)
