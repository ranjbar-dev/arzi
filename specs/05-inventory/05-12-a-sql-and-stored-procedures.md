_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 12. SQL and stored procedures

**Stored-procedure bodies and table DDL do not exist in this repository.** What follows is the
complete inventory of every SQL artefact the inventory domain touches: call signatures, the SQL
that *is* readable (quoted verbatim with `file:line`), inferred behaviour, and — for the three
procedures that matter — exactly what to run against production to recover them.

### 12.0 Stored-procedure inventory

| # | Procedure | Declared at | Parameters | Called from | Body recoverable? |
|---|---|---|---|---|---|
| 1 | `Anbar_AddToFactor;1` | `AnbarFactorU.dfm:433-546` | `@COID` int, `@Type` int, `@Factor` int, `@Date` varchar(10), `@Customer` int, `@Code` int, `@Name` varchar(50), `@prop` varchar(50), `@Vahed` varchar(50), `@Num` float, `@Phi` bigint, `@Kol` bigint, `@kasr` bigint, `@Maliat` bigint, `@user` int | `AnbarFactorU.pas:628-643`, once per line | **no — CRITICAL** |
| 2 | `Anbar_AjnasView;1` | `Dmu.dfm:396-416` | `@ID` int (warehouse) | `AnbarCalaU.pas:124-126` | no |
| 3 | `Anbar_CardJensi;1` | `Dmu.dfm:453-496` | `@Coid` int, `@Code` int, `@D1` varchar(10), `@D2` varchar(10) | `AnbarCardJensiU.pas:107-113` | no |
| 4 | `Anbar_Mandeh;1` | `Dmu.dfm:721-743` | `@Coid` int | `AnbarFactorU.pas:211-213` (`OP1`) | no |
| 5 | `Anbar_ReportKharidForoosh;1` | `Dmu.dfm:497-531` | `@D1` varchar(12), `@D2` varchar(12), `@Type` int | `AnbarReportKharidU.pas:64-68` — **unreachable screen (§13.12)** | no; and dead |
| 6 | `Anbar_PrintFactor;1` | `Dmu.dfm:532-560` | `@COID` int, `@Factor` int | `FactorPrintU.pas:96-98` — **unreachable unit (§13.15)** | no; and dead |
| 7 | `B_SelectSerial;1` | `Dmu.dfm:1143-1160` | `@GhabzNo` int | `Get_Serial.pas:56-58` — **unreachable (§8.6)** | no; external DB |
| 8 | `SP_SetRamz` | *(not declared in this project)* | `@GhabzNo`, `@Ramz`, `@User` → `Error`, `Message` | `Lab.pas:88-92` — **commented out** | no; external DB |
| 9 | `Sp_NRSelectGhabz` | *(not declared in this project)* | `@GhabzNo` | `Lab.pas:116-118`, `Ghabz.pas:87-90` — **all commented out** | no; external DB |

Supporting procedures from other domains that the inventory module calls (documented in
`docs/03-accounting-core.md`): `Sarfasl_Seek_SSN;1` (`@SSN`).

---

### 12.1 `Anbar_AddToFactor` — the one that matters

The single most important unknown in this specification. §10.1.1 lays out the evidence that it
writes both `Anbar_FactorD` **and** the `Moein` voucher lines.

**Call site**, verbatim (`AnbarFactorU.pas:626-645`):

```pascal
CDS1.First;
For i:=1 to CDS1.RecordCount Do Begin
   SP_AnbarAddToFactor.Parameters.ParamByName('@COID').Value := DM.CO_ID;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Type').Value := AF_Type.Tag;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Factor').Value := AF_Factor.Text;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Date').Value := AF_Date.Text;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Customer').Value := S_Bed.tag;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Code').Value := CDS1.FieldByName('Code').AsInteger;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Name').Value := CDS1.FieldByName('Name').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Prop').Value := CDS1.FieldByName('Prop').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Vahed').Value := CDS1.FieldByName('Vahed').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Num').Value := CDS1.FieldByName('Num').AsFloat;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Phi').Value := CDS1.FieldByName('Phi').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Kol').Value := CDS1.FieldByName('Kol').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Kasr').Value := CDS1.FieldByName('Kasr').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@Maliat').Value := CDS1.FieldByName('Maliat').AsString;
   SP_AnbarAddToFactor.Parameters.ParamByName('@user').Value := Dm.userId;
   SP_AnbarAddToFactor.ExecProc;
   CDS1.Next;
End;
```

**Inferred contract:**

```
Anbar_AddToFactor(@COID, @Type, @Factor, @Date, @Customer,
                  @Code, @Name, @prop, @Vahed, @Num, @Phi, @Kol, @kasr, @Maliat, @user)
  1. INSERT Anbar_FactorD (AFD_Coid, AFD_Type, AFD_Factor, AFD_Date, AFD_Customer,
                           AFD_Code, AFD_Name, AFD_Prop, AFD_Vahed,
                           AFD_Num, AFD_Phi, AFD_Kol, AFD_Kasr, AFD_Maliat,
                           AFD_Total,   -- computed: @Kol + @Maliat - @kasr
                           AFD_UserID)
  2. read AF_Sanad from Anbar_Factor where (AF_Coid=@COID and AF_Factor=@Factor)
     -- must exist: the caller posts the header at AnbarFactorU.pas:614, before the loop
  3. resolve Anbar_Jens.AJ_Code=@Code -> AJ_ID -> Anbar_Config
     -- the only route to the six posting accounts, none of which is a parameter
  4. INSERT Moein lines with M_ID=1, M_Link=@Factor, M_Sanad=<AF_Sanad>, M_Coid=@COID
     per the debit/credit table inferred at §10.1.2
```

**Facts, not inference:**

- No `@Total` parameter → `AFD_Total` is computed inside.
- No `@Sanad` parameter → the voucher number is read from `Anbar_Factor`, which makes the
  caller's ordering (post header at `:614`, loop at `:627`) load-bearing.
- No warehouse parameter → the accounts must be resolved via the item.
- `ExecProc`, and the result is discarded (`:643`) → the procedure returns no status the client
  acts on.
- Called once per line, outside any transaction → §5.4, §10.1.3.
- `@Phi`, `@Kol`, `@Kasr`, `@Maliat` are declared `ftLargeint` but bound from `AsString`
  (`:638-641`) — ADO coerces; a non-numeric string would fail at bind time.

**Recover it with:**

```sql
EXEC sp_helptext 'Anbar_AddToFactor';
-- or
SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_AddToFactor'));
```

**The three questions its body answers**, all of which change the rebuild:

1. **Which `M_ID` values does it write?** The delete on re-save covers only `M_ID = 1`
   (`AnbarFactorU.pas:621`) while the allocator reserves `1..9` (`:593`) and the delete-invoice
   path clears `1..9` (`AnbarListU.pas:384`). If the answer is anything but "always 1", inventory
   voucher lines **double on every re-save** (§10.1.3 defect 2).
2. **What are the exact debit/credit rules per `@Type`?** §10.1.2 is an educated guess.
3. **What does it do with `@Customer = 0`?** Reachable because of the broken guard at
   `AnbarFactorU.pas:579-583` (§4.2.2).

---

### 12.2 `Anbar_CardJensi` — the stock card

`Anbar_CardJensi;1(@Coid int, @Code int, @D1 varchar(10), @D2 varchar(10))`.

**Result columns**, recovered from the report bindings (`AnbarCardJensiU.dfm`):
`AFD_Date` (`:489`), `AFD_Factor` (`:520`), `Sanad` (`:552`), `AFD_TypeN` (`:584`),
`SunstomerN` (`:615`, a typo for `CustomerN`), `AFD_IN` (`:648`), `AFD_OUT` (`:701`),
`AFD_Phi` (`:796`), `ssn` (`:711`), `Sumr` (`:742`).

**Inferred behaviour:** filter `Anbar_FactorD` on `AFD_Coid = @Coid`, `AFD_Code = @Code`,
`AFD_Date BETWEEN @D1 AND @D2`; pivot `AFD_Num` into `AFD_IN` (types 1, 4) and `AFD_OUT`
(types 2, 3) per §5.1.1; join `Anbar_Factor` for `AF_Sanad` and `Sarfasl` for the counterparty
name; produce a type label.

**Two questions its body answers** (§11.1.3):

1. **Does it emit an opening-balance row?** The report's running total starts at `R := 0`, so
   without one the balance column is meaningless for any window that is not the whole year.
   `Sumr` may be it.
2. **What is the `ORDER BY`?** The running balance is a sequential accumulation over the result
   order, so the ordering *is* the algorithm. If it is `AFD_SSN`, editing an old invoice reorders
   the whole card (§11.1.3).

---

### 12.3 `Anbar_Mandeh` — the opening-stock generator

`Anbar_Mandeh;1(@Coid int)`. One parameter, no date, no code range.

**Result columns**, from `AnbarFactorU.pas:216-224`: `AFD_Code`, `AFD_Name`, `AFD_Vahed`,
`AFD_Prop` (fetched, assignment commented out at `:223`), **`Remi`** (remaining quantity) and
**`AVin`** (average inbound price).

**Inferred behaviour:** for every item, `Remi = Σ AFD_Num[1,4] − Σ AFD_Num[2,3]` over the whole
`@Coid`, and `AVin = Σ(AFD_Num × AFD_Phi)[type 1] / Σ AFD_Num[type 1]` — i.e. the same figures as
`Anbar_Jens_Phi1` (§6.2) but for all items at once and without the current-invoice exclusion.

**Not to be confused with** the `Q1` query inside `Anbar_MandehU.dfm` (§5.1.2), which shares the
name, takes five parameters and is fully readable. Two different artefacts.

**Question its body answers:** whether `Remi` is `Numeric(14,3)` or already integer. The client
reads it with `.AsInteger` (`AnbarFactorU.pas:216`), destroying fractions in the year-opening
document (§6.4).

---

### 12.4 `Anbar_AjnasView` — the item list view

`Anbar_AjnasView;1(@ID int)`, where `@ID` is `Anbar_Config.AC_ID`.

**Result columns** from the two consumers (`AnbarCalaU.dfm:145-260`, `AnbarCalaAddU.pas:90-98`):
`AJ_Code`, `AJ_Name`, `AJ_Prop`, `AJ_Vahed`, **`AJ_VahedC`**, `AJ_Phi`, **`AJ_PhiS`**
(a pre-formatted price string), `AJ_Maliat`, `AJ_Manfi`, `AJ_Alarm`, `SSTID`.

**Inferred behaviour:** `Select … From Anbar_Jens Where AJ_ID = @ID`, plus a formatted price
column. Whether it joins `Anbar_Vahed` is unknown; `AJ_Vahed` is denormalised on the table already,
so probably not.

Its default parameter value in the `.dfm` is `93` (`Dmu.dfm:414`) — a design-time leftover.

---


---

[← 11. Stock card and stock balance](05-11-stock-card-and-balance.md) | [index](00-index.md) | [12. SQL and stored procedures (part b) →](05-12-b-sql-and-stored-procedures.md)
