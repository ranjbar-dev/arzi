_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 8. Accounting integration

### 8.1 Party/person events that create vouchers: **none**

Exhaustively:

| Event | Voucher created? | Evidence |
|---|---|---|
| Create person / legal entity | **No** | `SahamdarEditU.pas:283-330`, `CompanyEditU.pas:255-301` — only `Sahamdar` INSERT/UPDATE + `Sarfasl_Add` |
| Update person / legal entity | **No** | same |
| Lock / unlock a party | **No** | `SahamdarU.pas:78-93` — single-column update |
| Move person ⇄ company | **No** | `SahamdarU.pas:136-176` — single-column update |
| Create the linked detail account | **No** | `Sarfasl_Add` inserts a chart node only |
| Edit party extended attributes | **No** | `Sarfasl_TakmilU.pas:65-84` |
| Add / pick a party bank account | **No** | `SahamdarInfoU.pas` is read-only |

Party master data is therefore **non-financial**. Vouchers touching a party's accounts always
originate in the treasury, sales, purchasing or inventory modules — other agents' domains.

### 8.2 The one posting rule this domain does own: year-end carry-forward

Fully specified in §1.6. Restated as debit/credit rules, per party account with balance `B` (where
`B_dr = Σ M_Bed`, `B_cr = Σ M_Bes` for the closing year):

**Closing voucher, fiscal year *N*, dated `Base(N).ToDate`:**

```
Dr  party account            B_cr
    Cr  closing control acct          B_cr
Dr  closing control acct     B_dr
    Cr  party account                B_dr
```
(implemented as two rows, each carrying both a debit and a credit amount — the legacy `Moein` row
holds `M_Bed` *and* `M_Bes` simultaneously, `EnteghalU.pas:~251-259`.)

**Opening voucher, fiscal year *N+1*, dated `Base(N+1).FromDate`:**

```
Dr  party account            B_dr
    Cr  opening control acct         B_dr
Dr  opening control acct     B_cr
    Cr  party account                B_cr
```

Net effect: the party's net position is reproduced in year *N+1* with the same sign, and the
closing/opening control accounts absorb the contra side.

### 8.3 The voucher-header helper (invoked by the rollover)

```pascal
// Dmu.pas:815-839
procedure TDM.DMoein_Make(_Sanad: integer; _Date, _Desc: String; _Kind:integer=1 );
begin
   QS.Close;
   QS.SQL.Clear;
   QS.ConnectionString := Ado.ConnectionString;
   QS.SQL.Add(' Declare @Sanad int Set @Sanad='+ inttostr(_Sanad) );
   QS.SQL.Add(' Declare @Coid  int Set @Coid='+ inttostr(CO_ID) );
   QS.SQL.Add(' Declare @User  int Set @User='+ inttostr(userId) );
   QS.SQL.Add(' Declare @Date varchar(10) Set @Date='+ QuotedStr(_Date) );
   QS.SQL.Add(' Declare @Desc varchar(200) Set @Desc='+ QuotedStr(_Desc ));

   QS.SQL.Add(' Declare @TC int, @TBed Bigint, @TBes Bigint ' );
   QS.SQL.Add(' Select @TBed=isnull(Sum(M_Bed),0), @TBes=isnull(Sum(M_bes),0), @TC=isnull(Count(*),0) ');
   QS.SQL.Add('    From moein where M_Sanad=@sanad and M_Coid=@Coid ');

   QS.SQL.Add(' if Exists( Select * From DMoein Where DM_Sanad=@Sanad and DM_Coid=@Coid)');
   QS.SQL.Add('    Update DMoein Set DM_CUser=@User, DM_CDate=GetDate(), DM_Tbed=@Tbed, DM_Tbes=@TBes, DM_Count=@TC, DM_Desc=@Desc ');
   QS.SQL.Add('    Where DM_Sanad=@Sanad and DM_Coid=@Coid ');
   QS.SQL.Add(' else ');
   QS.SQL.Add('   Insert DMoein (Dm_Sanad, DM_Date, DM_Desc, DM_Coid, DM_Tx, DM_TBed, DM_Tbes, DM_Count, DM_Muser, DM_MDate, DM_CUser, DM_Kind)  ');
   QS.SQL.Add('   Values( @Sanad, @Date, @Desc, @Coid, 0, @TBed, @TBes, @Tc, @User, GetDate(), 0,'+inttostr(_Kind) +' ) ');
   QS.SQL.Add(' ');
   QS.ExecSQL;
end;
```

An upsert that recomputes the header totals from its lines. The rebuild should make this a derived
view or a trigger-maintained aggregate rather than an explicit call.

### 8.4 Closing-balance export (`BastanHesab.pas`)

`بستن حساب` = "close accounts". Calls a stored procedure taking `@COID` and writes two INI-style
files, `D:\Bed.GGS` (debits) and `D:\Bes.GGS` (credits), each line carrying
`Kol / Moein / Taf1 / Taf2 / Mab`:

```pascal
// BastanHesab.pas:41-43
     SP1.Close;
     SP1.Parameters.ParamByName('@COID').Value := DM.CO_ID;
     SP1.Open;
```
```pascal
// BastanHesab.pas:55-76 (abridged)
        if Sp1.FieldByName('M_Bed').AsString >'0'  then
         Begin
            J:=J+1;
            S:= 'Line'+inttostr(J);
            F1.WriteString(S, 'Kol', Sp1.FieldByName('M_Ko').AsString );
            ...
            F1.WriteString(S, 'Mab', Sp1.FieldByName('M_Bed').AsString );
         End Else Begin
            ... F2 ... 'Mab', Sp1.FieldByName('M_Bes').AsString );
         End;
```
Message `فایل خروجی در مسیر d:\ ساخته شد` = "The output file has been created in d:\"
(`BastanHesab.pas:81`).

> The branch test is a **string** comparison (`AsString > '0'`), not numeric. `'10000000' > '0'` is
> true, so it happens to work for non-negative integers, but it is fragile. Also, hard-coded `D:\`
> paths. §12-Q18.

---


---

[← Previous](07-07-sahamdarconfig-party-account-configuration.md) · [Index](00-index.md) · [Next →](07-09-sql-and-stored-procedures.md)
