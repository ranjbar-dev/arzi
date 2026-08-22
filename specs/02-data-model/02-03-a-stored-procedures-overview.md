_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 3. Stored procedures

### 3.0 What is and is not knowable from this repository

**No stored-procedure body exists in this repository.** There is no `.sql` file, no `CREATE
PROCEDURE`, no schema script anywhere in the tree. Every procedure below is documented from:

- its **design-time declaration** in a `.dfm` (`TADOStoredProc.ProcedureName` plus the complete
  `Parameters` collection, which Delphi captures by querying the live server at design time — so
  the parameter names, ADO types, sizes, directions and *server defaults* are trustworthy);
- its **call sites** in `.pas` (which arguments are supplied, in what units);
- the **result-set columns** the callers read by name, and any persistent field definitions
  (which give the result columns' types and sizes).

The behaviour column below is therefore **inferred**. §3.5 lists exactly what must be dumped from a
live database before any of this can be ported.

The `;1` suffix on every `ProcedureName` (e.g. `'XNew;1'`) is the SQL Server *procedure group
number*, not part of the name.

### 3.1 Procedures declared on the main connection in `Dmu.dfm`

All fifteen use `Connection = Ado` except `B_SelectSerial`. Every one has an implicit
`@RETURN_VALUE ftInteger pdReturnValue`, omitted below.

---

#### `Taraz4Setooni` — 4-column trial balance
`Dmu.dfm:34-84`, component `SP_Taraz4Setooni`

| Parameter | ADO type | Dir | Size | Design default |
|---|---|---|---|---|
| `@ToDate` | `ftString` | in | 8 | `Null` |
| `@St` | `ftInteger` | in | — | `Null` |
| `@Ki` | `ftInteger` | in | — | `Null` |
| `@Sabt` | `ftInteger` | in | — | `Null` |
| `@Co` | `ftInteger` | in | — | `1` |

Computes the **4-column trial balance** (`تراز ۴ ستونی`) as at `@ToDate` — debit turnover, credit
turnover, debit balance, credit balance per account. `@Co` is the fiscal year (`CO_ID`). `@St` is
the account **level** (`سطح`) or a state filter, `@Ki` the `M_Kind` filter, `@Sabt` the `M_Tx`
posting-state filter (cf. `Taraz_6Sotooni` below, where `@Sabt` and `@Level` are separate and
explicit). **`@ToDate` is declared `ftString` size 8** — the short `YY/MM/DD` Jalali form (§6.1),
while the caller passes `D1.Farsi_Date`; a 10-character value is silently truncated by ADO.
Caller: `Taraz4Setooni_U.pas`. Result columns include `K`, `M` (`Taraz4Setooni_U.dfm`).

**Reporting query. No business rules written.** But it encodes the *definition* of a trial balance
(which `M_Kind`/`M_Tx` rows count) and that definition must be recovered.

---

#### `Taraz_6Sotooni` — 6-column trial balance
`Dmu.dfm:751-820`, component `SP_Taraz6Setooni`

| Parameter | ADO type | Dir | Size | Design default |
|---|---|---|---|---|
| `@D1` | `ftString` | in | 10 | `'1397/09/01'` |
| `@D2` | `ftString` | in | 10 | `'1397/09/31'` |
| `@kind` | `ftInteger` | in | — | `1` |
| `@Coid` | `ftInteger` | in | — | `1397` |
| `@Level` | `ftInteger` | in | — | `3` |
| `@Sabt` | `ftInteger` | in | — | `3` |

The **6-column trial balance** (`تراز ۶ ستونی`) for the period `@D1…@D2`: opening debit/credit,
period debit/credit, closing debit/credit. `@Level` = which of the four account levels to aggregate
to (default 3 = Tafsil1). `@kind` = `M_Kind` (1 = ordinary ledger). `@Sabt` = posting-state filter
(default `3`, which is **not** a valid `M_Tx` value — `M_Tx` ∈ {0,1,2}, §9.7 — so `3` is probably a
sentinel meaning "all"). Dates are the correct 10-character form here.
Caller: `Taraz6SetooniU.pas`.

**Reporting query**, but it embeds the opening-balance rule (everything before `@D1`) — a business
definition.

---

#### `Moein_View_Daftar` — subsidiary ledger (`دفتر معین`)
`Dmu.dfm:85-149`, component `Sp_Moein_View_Daftar`

| Parameter | ADO type | Dir | Size | Design default |
|---|---|---|---|---|
| `@ko` | `ftInteger` | in | — | `Null` |
| `@mo` | `ftInteger` | in | — | `Null` |
| `@Ta1` | `ftInteger` | in | — | `Null` |
| `@ta2` | `ftInteger` | in | — | `Null` |
| `@D1` | `ftString` | in | **8** | `Null` |
| `@D2` | `ftString` | in | **8** | `Null` |
| `@Co` | `ftInteger` | in | — | `1` |

Returns the ledger movements of one account (identified by the full 4-segment code) between two
dates, presumably with a running balance. Again **size-8 date parameters** — the same truncation
hazard as `Taraz4Setooni`.

**Reporting query** with an embedded running-balance rule.

---

#### `Moein_All` — all ledger lines for a year
`BastanHesab.dfm:43-58`, component `SP1`

| Parameter | ADO type | Dir | Design default |
|---|---|---|---|
| `@Coid` | `ftInteger` | in | `1398` |

Result columns read by the caller: `M_Bed`, `M_Bes`, `M_Ko`, `M_Mo`, `M_Ta1`, `M_Ta2`
(`BastanHesab.pas:57-73`). Almost certainly the per-account net balance for the year — it is the
input to the closing-entry export (§10.6), so **it is the definition of "the closing balance of
every account"**. Compare with the inline `#R` query in `EnteghalU.dfm:330-349`, which computes
apparently the same thing a different way: **two independent implementations of the year-end
balance**, and they must be diffed (§12).

**Encodes a business rule.** High priority to extract.

---

#### `MoeinAdd` — insert a voucher line
`ArticleMoeinu.dfm:337-458` (`@kind` default `1`), `ArticleRooznamehU.dfm:209-…` (`@kind` default `2`)

| Parameter | ADO type | Dir | Size | Design default |
|---|---|---|---|---|
| `@Ko` | `ftInteger` | in | — | `Null` |
| `@Mo` | `ftInteger` | in | — | `Null` |
| `@ta1` | `ftInteger` | in | — | `Null` |
| `@ta2` | `ftInteger` | in | — | `Null` |
| `@bed` | `ftString` | in | **20** | `Null` |
| `@bes` | `ftString` | in | **20** | `Null` |
| `@ted` | `ftString` | in | **15** | `Null` |
| `@Sanad` | `ftInteger` | in | — | `Null` |
| `@date` | `ftString` | in | **8** | `Null` |
| `@user` | `ftInteger` | in | — | `Null` |
| `@des` | `ftString` | in | 200 | `Null` |
| `@State` | `ftInteger` | in | — | `Null` |
| `@kind` | `ftInteger` | in | — | `1` / `2` |

**The only write procedure on the accounting core.** Note that the **amounts are passed as
strings** (`@bed`, `@bes` `varchar(20)`) and so is the quantity (`@ted` `varchar(15)`) — the
procedure must parse them server-side. That is where the money type is decided (§7) and it is a
real risk: a locale-dependent or lenient conversion inside T-SQL. `@kind` distinguishes the ledger
(`1`) from the journal/`Rooznameh` view (`2`).

It almost certainly also resolves `M_Code` from `Sarfasl` and maintains `DMoein` totals — the
callers do not do it. **Encodes business rules. Highest extraction priority.**

---

#### `MoeinViewSanad` / `MoeinTotalSanad` — voucher detail and totals
`SanadMoeinu.dfm:335-…` and `SanadMoeinu.dfm:448-…`

| Procedure | Parameters |
|---|---|
| `MoeinViewSanad` | `@Sanad ftInteger`, `@Co ftInteger` |
| `MoeinTotalSanad` | `@sanad ftInteger`, `@co ftInteger` (default `1396`) |

Return the lines of one voucher and its debit/credit totals. **Reporting queries.**
Note `MoeinTotalSanad` is declared **without** the `;1` group suffix — the only one.

---

#### `Moein_ChapSanad` — voucher print
`RoozViewU.dfm:173-…`, `SanadViewU.dfm:599-…`, component `ChapSanad` (`چاپ سند` = "print voucher")

| Parameter | ADO type | Design default |
|---|---|---|
| `@Sanad` | `ftInteger` | `1` / `Null` |
| `@Co` | `ftInteger` | `1` (only in `SanadViewU.dfm`) |

Note the `RoozViewU.dfm` declaration has **only `@Sanad`** — no fiscal-year parameter. Either the
procedure was changed and one form was not refreshed, or voucher numbers were assumed globally
unique. Either way it is a **cross-year data-leak hazard** (§12).

**Reporting query**, feeding FastReport.

---

#### `Asnad_View` — voucher list
`RoozViewU.dfm:207-…`, component `Sp1`

| Parameter | ADO type | Design default |
|---|---|---|
| `@State` | `ftInteger` | `0` |
| `@kind` | `ftInteger` | `1` |

Lists vouchers (`اسناد`) filtered by posting state (`M_Tx`/`DM_Tx`) and kind. **No fiscal-year
parameter** — same hazard as above. **Reporting query.**

---

#### `KolState` — general-ledger account state
`KolStateU.dfm:100-…`, component `Sp1`

| Parameter | ADO type | Design default |
|---|---|---|
| `@kol` | `ftInteger` | `413` |
| `@CoID` | `ftInteger` | `1397` |

Per-Kol summary for one fiscal year. **Reporting query.**

---

#### `Sarfasl_view` — chart-of-accounts listing
`ListSarfaslu.dfm:221-…`, component `Sp1`

| Parameter | ADO type | Design default |
|---|---|---|
| `@Co` | `ftInteger` | `1` |

Note it takes a fiscal year even though `Sarfasl` has **no year column** (§1.4). Either the
parameter is ignored, or the procedure joins `Base` for the code widths (`No_Ko`…`No_Ta2`, §8.3) to
format the codes. **Reporting query** — but resolve which.

---

#### `Sarfasl_Seek_SSN` / `Sarfasl_Seek_Name` — account lookup
`Dmu.dfm:308-331` (component `Sarfasl_seekSSN`) and `Dmu.dfm:329-…` (component `Sarfasl_SeekName1`)

| Procedure | Parameter | Type | Size | Default |
|---|---|---|---|---|
| `Sarfasl_Seek_SSN` | `@SSN` | `ftInteger` | — | `0` |
| `Sarfasl_Seek_Name` | `@Name` | `ftString` | 50 | `''` |

Point lookups by surrogate key and by (partial?) name. **Reporting queries.**
Result columns read include `M_L`, `S_Name`, `S_SSN` (`ArticleMoeinu.pas:181-183`, commented out).

---

#### `Sarfasl_ADD` — create an account
`SNewu.dfm:908-…`, `S_KolU.dfm:658-…` (component `SP_Add`); also called as raw SQL from
`CompanyEditU.pas:309-311` and `SahamdarEditU.pas:338-…`

| Parameter | ADO type | Size | Design default |
|---|---|---|---|
| `@Ko` | `ftInteger` | — | `514` |
| `@Mo` | `ftInteger` | — | `0` |
| `@Ta1` | `ftInteger` | — | `0` |
| `@Ta2` | `ftInteger` | — | `0` |
| `@Name` | `ftString` | 100 | `'test'` |

Inserts a `Sarfasl` node at the given 4-segment code. Called positionally from Delphi:

```pascal
Q1.SQL.Add('Exec Sarfasl_Add  '+ inttostr(_Ko)+ ', '+ inttostr(_Mo)+ ', '+
            inttostr(_Ta1)+ ', '+ inttostr(_Ta2)+ ', '+ QuotedStr(_Name) );   // CompanyEditU.pas:309
```

Note the case difference (`Sarfasl_Add` in the raw calls vs `Sarfasl_ADD` in the `.dfm`) — harmless
under SQL Server's default collation, but a real difference under a case-sensitive one.

**Encodes business rules**: it must maintain `S_Child` (the child counter, cf. `Dmu.pas:300-318`),
`FullName`, and possibly `M_L`/`M_R` nested-set columns. **High extraction priority.**

---

---

[← 02-02-b-table-inventory-parties-vouchers.md](02-02-b-table-inventory-parties-vouchers.md) | [02-03-b-stored-procedures-continued.md →](02-03-b-stored-procedures-continued.md)
