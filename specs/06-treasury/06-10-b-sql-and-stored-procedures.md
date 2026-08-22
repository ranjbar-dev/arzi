_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

### 10.6 Deposit slip save (`FISHDaryaftU.pas:415-484`)

```sql
 Begin Transaction
 Declare @SSN int Set @SSN=0
 Declare @Coid int Set @Coid=<CO_ID>
 Declare @State int Set @State=<combo index + 1>
 Declare @Sanad int Set @Sanad=<S_Sanad>
 Declare @Mab bigint Set @Mab=<S_Mab>
 Declare @BedSSN bigint Set @BedSSN=<bank account id>
 Declare @BesSSN bigint Set @BesSSN=<payer account id>
 Declare @UserID bigint Set @UserID=<userId>
 Declare @LinkPRG bigint Set @LinkPRG=<link prg>
 Declare @LinkSSN bigint Set @LinkSSN=<link ssn>
 Declare @BedCR varchar(25) Set @BedCR='<bank code>'
 Declare @BesCR varchar(25) Set @BesCR='<payer code>'
 Declare @BedName varchar(200) Set @BedName='<bank name>'
 Declare @BesName varchar(200) Set @BesName='<payer name>'
 Declare @Date varchar(10) Set @Date='<jalali date>'
 Declare @Desc varchar(200) Set @Desc='<description>'
 Declare @StateN varchar(30) Set @StateN='<channel label>'
 Declare @FishNo varchar(20) Set @FishNo='<slip no>'

-- new:
Insert DFish ( S_Coid, S_State, S_StateName, S_FishNo, S_Sanad, S_Date, S_Mab, S_Desc, S_BesSSN,
               S_BesCR, S_BesName, S_BankSSN, S_BankCR, S_BankName, S_UserID, S_LinkPRG, S_LinkSSN )
      Values ( @Coid, @State, @StateN, @FishNo, @Sanad, @Date, @Mab, @Desc, @BesSSN,
               @BesCR, @BesName, @BedSSN, @BedCR, @BedName, @UserID, @LinkPRG, @LinkSSN )
 commit
  Set @SSN= @@Identity

-- edit:
 Set @SSN=<Edit_SSN>
Update DFish Set S_Coid=@Coid, S_State=@State, S_StateName=@StateN, S_FishNo=@FishNo,
       S_Sanad=@Sanad, S_Date=@Date, S_Mab=@Mab, S_Desc=@Desc, S_BesSSN=@BesSSN,
       S_BesCR=@BesCR, S_BesName=@BesName, S_BankSSN=@BedSSN, S_BankCR=@BedCR,
       S_BankName=@BedName, S_UserID=@UserID, S_LinkPRG=@LinkPRG, S_LinkSSN=@LinkSSN
 Where S_SSN=@SSN
 commit

-- both paths continue:
 Begin Transaction
 Delete moein where M_coid=@Coid and M_link=@SSN and M_ID=25
 Declare @SBes varchar(200) Set @SBes='بابت <channel> شماره <fishNo> به <bankName> <desc>'
 Declare @SBed varchar(200) Set @SBed='بابت <channel> شماره <fishNo> توسط <payerName> <desc>'
 insert moein ( M_kind, M_Coid, M_Sanad, M_date, M_Bed, M_Bes, M_Ted, M_Tx, Article,
      M_Ko, M_Mo, M_Ta1, M_Ta2,M_Id, M_Link, M_User, M_Code )
 Select 1, D.S_Coid, D.S_Sanad, D.S_Date, D.S_Mab, 0, 0, 0,@SBed, S.S_Ko,S.S_Mo,S.S_Ta1
     ,S.S_Ta2, 25, D.S_SSN, D.S_UserID, D.S_BankSSN
 from DFish As D
 left join sarfasl as s on S.S_SSN=D.S_BankSSN
 Where D.S_SSn=@SSN
 insert moein ( … )
 Select 1, D.S_Coid, D.S_Sanad, D.S_Date, 0,D.S_Mab, 0, 0,@SBes, S.S_Ko,S.S_Mo,S.S_Ta1
     ,S.S_Ta2, 25, D.S_SSN, D.S_UserID, D.S_BesSSN
 from DFish As D
 left join sarfasl as s on S.S_SSN=D.S_BesSSN
 Where D.S_SSn=@SSN
 commit
 Select @SSN As _IDN
```

The `Begin Transaction` before the `Declare` block is closed by a `commit` inside the branch —
so on the **edit** path the declares and the `UPDATE` are in one transaction and on the **new** path
the declares and the `INSERT` are, which is accidental rather than designed. The trailing
`Select @SSN As _IDN` is why the whole batch is run with `Qs.Open` rather than `ExecSQL`.

`@BedCR`/`@BesCR` are declared `varchar(25)` while the corresponding form fields and the `DCheck`
equivalents are 50 — a **silent truncation** of any account code longer than 25 characters.

Note `@SBed`/`@SBes` are attached to the wrong sides (§8.5 defect 2).

### 10.7 Issued-cheque batch save (`CheckEditU.pas:412-511`)

The only place in treasury that uses **bound parameters**:

```sql
  insert CheckMaster ( CM_Coid, CM_No,   CM_Sanad,  CM_Date,  CM_Mab,  CM_Desc,  CM_Tittle,  CM_Code,  CM_CodeCR,  CM_CodeName,  CM_Count,  CM_UserID)
              Values (:CM_Coid, :CM_No, :CM_Sanad, :CM_Date, :CM_Mab, :CM_Desc, :CM_Tittle, :CM_Code, :CM_CodeCR, :CM_CodeName, :CM_Count, :CM_UserID)
SELECT @@IDENTITY as _SSN
```

or, in edit mode:

```sql
  Update CheckMaster Set CM_Coid=:CM_Coid, CM_Sanad=:CM_Sanad, CM_Date=:CM_Date, CM_Mab=:CM_Mab, CM_Desc=:CM_Desc, CM_Tittle=:CM_Tittle
      ,CM_Code=:CM_Code, CM_CodeCR=:CM_CodeCR, CM_CodeName=:CM_CodeName,  CM_Count=:CM_Count, CM_UserID=:CM_UserID, CM_No=:CM_No
  Where CM_SSN=<SSN>
SELECT @@IDENTITY as _SSN
```

then the lines, deleted and re-inserted in a client-side loop:

```sql
  Delete CheckDetail Where CD_CMSSN= <SSN>

 INSERT INTO CheckDetail ( CD_CMSSN, CD_Coid, CD_Bed, CD_BedCR, CD_BedName,  CD_Mab, CD_Desc, CD_BankNo, CD_Jari)
        Values  ( :CD_CMSSN, :CD_Coid, :CD_Bed, :CD_BedCR, :CD_BedName,  :CD_Mab, :CD_Desc, :CD_BankNo, :CD_Jari)
```

then the postings:

```sql
 Delete moein
  Where M_Coid=<CO_ID> and  M_Id=26 and M_Link=<SSN>

 Insert Moein (M_Coid, M_Sanad, M_Date, M_Bed, M_Bes, M_Ted, Article, M_Tx, M_Ko, M_Mo, M_Ta1, M_Ta2, M_Id, M_Link, M_User, M_Kind, M_Code, M_Time )
 Values ( <CO_ID>, :Sanad, :Date, :Bed, :Bes, 0, :Article, 0, 0, 0, 0, 0, 26, <SSN>, <userId>, 1, :Code, GetDate() )
```

executed once per payee line and then a final time for the bank credit — note the `M_Ko/M_Mo/M_Ta1/
M_Ta2` are hard-coded `0` and repaired afterwards:

```sql
  Update Moein Set Moein.M_Ko=sarfasl.S_Ko, Moein.M_Mo=sarfasl.S_Mo, Moein.M_Ta1=sarfasl.S_Ta1, Moein.M_Ta2=sarfasl.S_Ta2
     from sarfasl Where S_SSN=Moein.M_Code
       and Moein.M_Coid=<CO_ID> and Moein.M_Sanad=<CM_Sanad>
```

**This last statement is scoped to the voucher, not the document** — it rewrites the hierarchy
columns of every treasury line sharing that day's voucher (§8.4). `M_Time` (a real timestamp via
`GetDate()`) appears **only** on these batch postings and on the petty-cash equivalent; the
cheque-lifecycle postings have no timestamp at all.

`TankhahEdit.pas:391-489` is the identical sequence with `Tankhah*` tables, `M_Id = 41`, and the
`CD_BankNo`/`CD_Jari` columns absent.


---

[← 10. SQL and stored procedures (part a)](06-10-a-sql-and-stored-procedures.md) | [index](00-index.md) | [10. SQL and stored procedures (part c) →](06-10-c-sql-and-stored-procedures.md)
