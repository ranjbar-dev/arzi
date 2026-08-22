_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 4.3 Accounting-core SQL in `Dmu.pas`

#### 4.3.1 `TDM.DMoein_Make` — upsert the voucher header

```pascal
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
   QS.ExecSQL;
```
`Dmu.pas:820-837`

**Intent.** After lines have been written to `Moein`, create or refresh the matching `DMoein`
header: recompute the totals and the line count from the lines, then upsert.

**Findings — this one statement is the origin of several claims made elsewhere in this document.**

- **It is the evidence for §2.8's C/M swap** (§12.10 item 5): the `UPDATE` branch writes
  `DM_CUser`/`DM_CDate`, and the `INSERT` branch writes `DM_MUser`/`DM_MDate` (setting `DM_CUser`
  to a literal `0`). The prefixes are inverted relative to create/modify convention.
- **`DM_Tx` is hard-coded to `0`** on insert — a new header is always a draft (§9.7).
- **Classic check-then-act race** (§5.6): `IF EXISTS … UPDATE … ELSE INSERT` with no transaction,
  no `HOLDLOCK`, and no unique constraint on `(DM_Coid, DM_Sanad)` to catch the loser. Two
  concurrent saves of the same voucher number produce **two header rows**.
- Totals are recomputed from `Moein` at header level, so they are correct **at the instant of this
  batch** and drift thereafter (§7.7 check 4).
- `@Desc varchar(200)` while `DMoein.DM_Desc` is `varchar(500)` (§2.8) — **descriptions longer than
  200 characters are silently truncated** by the local variable before they ever reach the column.
  Not previously recorded; add to the migration checks.

**Rebuild.** `INSERT … ON CONFLICT (fiscal_year_id, voucher_number) DO UPDATE`, inside the same
transaction as the lines, against the real unique constraint of §11.4.

#### 4.3.2 `TDM.Dmoein_UpdateMab` — refresh totals, and delete an empty header

```pascal
   QS.SQL.Add(' Declare @Sanad int Set @Sanad='+ inttostr(_Sanad) );
   QS.SQL.Add(' Declare @Coid  int Set @Coid='+ inttostr(CO_ID) );
   QS.SQL.Add(' Declare @Count int, @TBed Bigint, @TBes Bigint ' );

   QS.SQL.Add(' Select @TBed=isnull(Sum(M_Bed),0), @TBes=isnull(Sum(M_bes),0), @Count=isnull(Count(*),0) ');
   QS.SQL.Add('    From moein where M_Sanad=@sanad and M_Coid=@Coid ');
   QS.SQL.Add(' Update DMoein Set DM_TBed=@TBed, DM_TBes=@TBes, DM_Count=@Count ');
   QS.SQL.Add('    where DM_Sanad=@sanad and DM_Coid=@Coid ');

   QS.SQL.Add('    Delete DMoein Where DM_Sanad=@sanad and DM_Coid=@Coid and DM_Count=0 ');
   QS.ExecSQL;
```
`Dmu.pas:846-856`

**Intent.** Recompute the header's denormalised totals after lines change, and **garbage-collect the
header when its last line is deleted**.

**Findings.**

- The `DELETE` is the rule §2.8 states as "a header with `DM_Count = 0` is deleted". It fires
  **unconditionally at the end of every totals refresh**, so deleting the last line of a voucher
  silently destroys the header — including its description and its `DM_Tx` state — with no
  confirmation and no audit record.
- Runs **outside any transaction** — §9.4's torn-document failure mode is precisely this batch
  interleaved with `MoeinAdd`.

**Rebuild.** Totals maintained in-transaction (or derived); deleting the last line is an explicit,
audited operation, not a side effect of a totals refresh.

#### 4.3.3 `TDM.Delete_Moein_Sanad` — guarded voucher deletion

```pascal
    FmtStr( S , 'Select  Max(M_ID) As ID, Count(*) As N, Max(M_Tx) As TX From Moein Where M_Sanad=%d and M_COID=%d', [Sanad,Co_id] );
```
`Dmu.pas:1289` — then, only if `N > 0`, `TX = 0` and `ID = 0`:
```pascal
    FmtStr( S , 'Delete Moein Where M_Sanad=%d and M_COID=%d', [Sanad,Co_id] );
    FmtStr( S , 'Delete DMoein Where DM_Sanad=%d and DM_COID=%d', [Sanad,Co_id] );
    QX.ExecSQL;
```
`Dmu.pas:1318-1322`

**Intent.** Delete a whole voucher, but only when it (a) exists, (b) is still a draft
(`MAX(M_Tx) = 0`) and (c) was **not generated by another module** (`MAX(M_ID) = 0`).

**Findings.**

- `MAX(M_ID) > 0` is the guard that stops a user deleting a system-generated voucher (a cheque
  posting, an invoice posting) from the accounting screen. It is the **only referential protection
  the `(M_ID, M_Link)` polymorphic pointer receives anywhere** (§13.8).
- Both `DELETE`s are in one batch, again untransacted: a failure between them leaves an **orphan
  `DMoein` header with no lines**.
- The guard reads `MAX(M_Tx)`, not "every line" — a voucher with one draft and one posted line
  would be blocked, correctly, but a voucher with mixed `M_ID` values where the maximum is `0`
  cannot occur, so the check is sound *given* that `M_ID` is non-negative.

**Rebuild.** One transaction; the guard becomes a service-layer precondition, and `ON DELETE
CASCADE` on `voucher_lines.voucher_id` (§11.4) makes the two-statement dance unnecessary.

#### 4.3.4 `TDM.Delete_Moein_ssn` — the single-line variant

Same shape, keyed on `M_SSN` instead of the voucher number:

```pascal
    FmtStr( S , 'Select  Max(M_ID) As ID, Count(*) As N, Max(M_Tx) As TX From Moein Where M_SSN=%d ', [SSN] );
    ...
    FmtStr( S , 'Delete Moein Where M_SSN=%d ', [SSN] );
```
`Dmu.pas:1335`, `Dmu.pas:1363`

**Finding — a real defect.** This deletes a line and **never calls `Dmoein_UpdateMab`**, so the
header's `DM_TBed`, `DM_TBes` and `DM_Count` are left stale, and the empty-header cleanup of §4.3.2
never runs. It is a direct cause of the denormalisation drift that §7.7 check 4 tells the migration
to probe for. Note also that `M_SSN` is not year-qualified — correct, since it is the surrogate key,
but it means the fiscal-year gate (§9.8) is not applied on this path.

#### 4.3.5 Voucher-navigation and state queries

| Statement | Location | Intent |
|---|---|---|
| `Select isnull(max(M_TX),0) As TX From Moein Where M_Sanad=… And M_COID=…` | `Dmu.pas:1171-1174` | the voucher's effective posting state = the **maximum** of its lines' states (§9.7) |
| `Select isnull(Max(M_Tx),0) as TX From Moein Where M_Coid=… and M_sanad=…` | `Dmu.pas:1544-1547` | the same query a second time, in a second method |
| `Declare @N int Set @N=0` … `Set @N=(Select min(M_sanad) from moein where m_tx=<tx> and M_sanad <\|> <n>)` … `Select @N As N` | `Dmu.pas:872-877` | previous / next voucher in a given state — the ⏴ ⏵ navigation buttons |
| `Select isnull(Max(M_Date),'') as F_Date From Moein Where M_Coid=… and M_sanad=…` | `Dmu.pas:1486-1489` | `TDM.Get_SanadDate` — the voucher's date is `MAX` of its lines' dates (§2.7) |
| `Select * From Moein Where M_Coid=… and M_Sanad=… and M_ID in (<IDList>)` | `Dmu.pas:1515-1518` | does this voucher contain lines from a given module family? |
| `Select * From Moein Where M_Coid=… and M_sanad=…` | `Dmu.pas:1529-1532` | all lines of a voucher |
| `Select * From DMoein Where DM_sanad=… and DM_coid=…` | `Dmu.pas:990-991` | `Is_Admin_Or_Valid_Sanad` — read `DM_Lock` (§9.6) |

**Findings.**

- **`Dmu.pas:872-877` omits `M_COID` entirely.** Voucher-to-voucher navigation therefore ranges
  over **every fiscal year at once** — the same cross-year leak §12.12 flags for `Asnad_View`, but
  here it is visible in plain source rather than hidden in a procedure body. ⚠ Confirmed defect.
- `Get_SanadDate` reading `MAX(M_Date)` only makes sense because every line of a voucher is
  *supposed* to share the header's date (§2.7). Nothing enforces it.
- The `M_ID in (<IDList>)` splice at `Dmu.pas:1517` is the same raw-string interpolation as §4.4.3.

#### 4.3.6 Document-number allocation

Both allocators are quoted and analysed in **§5.3**; cross-referenced here for completeness:

- `Select isnull(Max(M_sanad),0)+1 as NewSanad From moein Where M_COid=…` — `Dmu.pas:1247`
  (`TDM.New_Sanad`, §5.3.1)
- `Select isnull(Max(AF_Factor),0)+1 as NewFactor From Anbar_factor Where AF_COid=…` —
  `Dmu.pas:1258` (`TDM.New_AnbarFactor`, §5.3.3)

Note both scope by `*_COid` — so numbering is per-year, as §11.0 models it — and both are the
`MAX+1` race of §5.6 R8.

---

### 4.4 Chart-of-accounts and party lookups in `Dmu.pas`

#### 4.4.1 The four-segment account lookups

The same query shape appears **six times** in `Dmu.pas`:

```pascal
   Q1.SQL.Add(' Select * From sarfasl Where S_Ko='+ inttostr(_Ko) +
              ' and S_Mo=' + inttostr(_Mo) + ' and S_Ta1=' + inttostr(_Ta1) + ' and S_Ta2=' + inttostr(_Ta2) );
   Q1.Open;
```
`Dmu.pas:929-931`, `:939-941`, `:949-951`, `:959-961` — the four `Is_Admin_Or_Valid_Daftar` /
`is_Sarfasl_Last_Deep` overloads (§9.6).

And the progressive-narrowing variant, used to resolve a partial code:

```pascal
   Q1.SQL.Add(' Select * From sarfasl Where s_Ko='+ inttostr(ko) );
   if mo>0  then Q1.SQL.Add( ' And s_mo='+  inttostr(mo)  );
   if ta1>0 then Q1.SQL.Add( ' And s_ta1='+ inttostr(ta1) );
   if ta2>0 then Q1.SQL.Add( ' And s_ta2='+ inttostr(ta2) );
   Q1.Open;
```
`Dmu.pas:1026-1030` and, identically, `Dmu.pas:1056-1060`.

Plus two surrogate-key point lookups:
`Select * From sarfasl Where s_SSN=…` (`Dmu.pas:1044-1045`),
`Select * From Sarfasl Where S_SSN=…` (`Dmu.pas:1393-1394`, `Dmu.pas:1450-1451`).

**Intent.** Resolve a typed account code to a row, and decide whether it is a postable leaf.

**Findings.**

- **No `*_COID` filter** — correct, and the primary source evidence that `Sarfasl` is global
  (§1.4, §2.5).
- The caller asserts `RecordCount = 1` for a full code (§2.5), which is exactly the uniqueness
  §13.1 proposes to enforce. **The `if mo>0` narrowing means a partial code legitimately returns
  many rows**, so the assertion only holds for the four-segment form.
- `is_Sarfasl_Last_Deep` **fails open** when the code matches nothing (§9.6): zero rows is not
  treated as an error. The rebuild's `accounts::is_leaf()` must fail closed.
- Same-shaped query duplicated six times across four methods — a single `accounts::find_by_code()`
  in the rebuild.

#### 4.4.2 Party and party-configuration lookups

| Statement | Location | Intent |
|---|---|---|
| `Select * From Sahamdar Where S_Card=…` | `Dmu.pas:975-976` | `Is_Admin_Or_Valid_Jari` — read the party's `S_Lock` (§9.6). **Fails open for an unknown card.** |
| `Select * From Sahamdar Where S_Card=…` | `Dmu.pas:1423-1424` | the same query again, in a different method |
| `Select * From SahamdarConfig Where SC_K=<k> and SC_M=<m> …` | `Dmu.pas:1412-1414` | the Kol/Moein pairs for which a party gets an account (§2.6, `SahamdarEditU.pas:288-300`) |
| `Select * From DFish where S_SSN=…` | `Dmu.pas:349-350` | point lookup of a deposit slip |
| `Select * From Pass_Config where P_User=<u> and P_ID=<k>` | `Dmu.pas:1557-1559` | **`TDM.IsEnabel`** — one round trip per permission per check (`08-platform-and-security.md` §4.2) |

**Finding.** `SahamdarConfig` is a **table** — not a query, despite being declared as a `TADOQuery`
in `Dmu.dfm:8607`. §2.2 lists it under "not tables"; this statement shows the *dataset* is a query
but the *object it selects from* is a real table with columns `SC_K`, `SC_M`, `SC_T`, `SC_Kind`,
`SC_Tik`, `SC_Rem`. **It is missing from §2's master list and from §11's DDL.** Add it to the
artefacts required in §12.5.

#### 4.4.3 `TDM.Get_NewSanad_DateID` — the injection surface

```pascal
    Dm.Q1.SQL.Add(' Select isnull( Max(M_Sanad) , 0 ) as S  ');
    Dm.Q1.SQL.Add('   From Moein ');
    Dm.Q1.SQL.Add('   Where M_Tx=0 and M_Coid='+ inttostr(Dm.CO_ID) );
    Dm.Q1.SQL.Add('   and M_ID in( ' + IDList+ ')' );
    Dm.Q1.SQL.Add('   and M_Date='+ QuotedStr(F_Date) );
    Dm.Q1.Open;
```
`Dmu.pas:1467-1472`

**Intent.** "Is there already a draft voucher for today, created by one of these modules? If so,
reuse its number rather than allocating a new one." Fully analysed in **§5.3.2**; quoted here
because it is the one place in the data layer where a **string parameter is spliced into SQL
without quoting**.

**Findings.**

- `IDList` arrives as a caller-supplied string such as `'21,22,23,24'`. Every call site passes a
  literal, so it is not *currently* exploitable — but it is an injection by construction, and §5.7
  replaces it with a typed `smallint[]` bind parameter.
- `M_Date` is compared with `=` against a **Jalali string** (§6.5). Correct only because both sides
  use the identical 10-character format — which §12.1 cannot prove.

---


---

[← 02-04-a-adhoc-sql-schema-and-startup.md](02-04-a-adhoc-sql-schema-and-startup.md) | [02-04-c-adhoc-sql-design-time-datasets.md →](02-04-c-adhoc-sql-design-time-datasets.md)
