_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### continued (§3.1 "Procedures declared on the main connection in `Dmu.dfm`", cont'd)

#### `Sarfasl_Deep` — delete/inspect an account subtree
`ListSarfaslu.dfm:249-…`, component `Sp_Del`

| Parameter | ADO type | Default |
|---|---|---|
| `@k` | `ftInteger` | `Null` |
| `@m` | `ftInteger` | `Null` |
| `@t1` | `ftInteger` | `Null` |
| `@t2` | `ftInteger` | `Null` |
| `@Co` | `ftInteger` | `1` |

Component named `Sp_Del` — so it either deletes the node or reports whether it can be deleted
(children? postings?). `Deep` (`عمق`) matches `TDM.is_Sarfasl_Last_Deep` (`Dmu.pas:920-…`), which
tests "is this the deepest node". **Encodes business rules** (referential-integrity checks the
schema does not enforce). **High extraction priority.**

---

#### `Select_Kol` / `Select_moein` / `Select_Taf1` / `Select_Taf2` — cascading account pickers
`ArticleMoeinu.dfm:459, 474, 501, 535`; also `KolStateU.dfm:85`, `KolSatateU.dfm:79`,
`NewSarfaslu.dfm:242`

| Procedure | Parameters |
|---|---|
| `Select_Kol` | *(none)* |
| `Select_moein` | `@kol ftInteger` |
| `Select_Taf1` | `@kol`, `@moein` |
| `Select_Taf2` | `@kol`, `@moein`, `@taf1` |

Four lookup lists driving the cascading combo boxes. **Pure reporting queries.** In the rebuild
these collapse into one endpoint `GET /api/v1/accounts?parent=<code>`.

---

#### `Active_Set` — recompute account activity flags
Called as raw SQL only: `SNewu.pas:303`, `SNewu.pas:346`, `S_KolU.pas:363` — `exec Active_Set`.
**No parameters.** Invoked after creating or modifying accounts, so it is a **maintenance
procedure** that rewrites a derived column across `Sarfasl` (very likely `S_Active` — the name
appears in the field inventory — and/or `S_Child`). Compare `TDM.Update_Sarfasl_Child`
(`Dmu.pas:300-318`), which does the `S_Child` half in client-side SQL. **Encodes business rules.**

---

#### `Sahamdar_Seek` — party lookup by card number
`Dmu.dfm:150-172`

| Parameter | ADO type | Default |
|---|---|---|
| `@S_card` | `ftInteger` | `0` |

Looks a party up by `S_Card`. **Reporting query.**

---

#### `Sahamdar_Edit` — upsert a party
`Dmu.dfm:173-307` — the largest parameter list in the system, and effectively **the `Sahamdar`
column list**:

| Parameter | ADO type | Size | Meaning | Persian |
|---|---|---|---|---|
| `@Card` | `ftInteger` | — | card / member number — the business key | — |
| `@kind` | `ftWord` | — | `1` = natural person, `2` = legal entity (`docs/01-glossary.md` §6b) | — |
| `@Name` | `ftString` | 50 | first name | نام |
| `@Famil` | `ftString` | 50 | surname | فامیل |
| `@Father` | `ftString` | 50 | father's name | نام پدر |
| `@BDate` | `ftString` | **8** | date of birth (Jalali, short form) | تاریخ تولد |
| `@BPlace` | `ftString` | 20 | place of birth | محل تولد |
| `@SDate` | `ftString` | **8** | ID-card issue date | تاریخ صدور |
| `@SPlace` | `ftString` | 20 | ID-card issue place | محل صدور |
| `@IDNO` | `ftInteger` | — | ID-card number (`شماره شناسنامه`) | — |
| `@Address` | `ftString` | 100 | address | آدرس |
| `@CodeMelli` | `ftString` | 10 | national ID | کد ملی |
| `@CodePosti` | `ftString` | 10 | postal code | کد پستی |
| `@Mobile` | `ftString` | 12 | mobile number | موبایل |
| `@Phone` | `ftString` | 12 | landline | تلفن |
| `@Siba` | `ftString` | 13 | SIBA bank account number (default `' '`) | سیبا |

**Encodes business rules**: whether it inserts or updates, and whether it creates the matching
`Sarfasl` node (the callers `CompanyEditU.pas:294-297` and `SahamdarEditU.pas:338` call
`Sarfasl_Add` *separately*, which suggests it does not — verify). `@IDNO` is an **integer**, so
Iranian ID numbers with leading zeros are corrupted. `@CodeMelli` is correctly a 10-char string.

---

#### `Sahamdar_Show` — party detail
`Dmu.dfm:568-…`

| Parameter | ADO type | Default |
|---|---|---|
| `@Id` | `ftInteger` | `Null` |

**Reporting query.** Note the parameter is `@Id`, not `@Card` — a different key from
`Sahamdar_Seek`. Which one is the primary key must be settled (§12).

---

#### `Anbar_AjnasView` — item master listing
`Dmu.dfm:396-417`

| Parameter | ADO type | Default |
|---|---|---|
| `@ID` | `ftInteger` | `93` |

`اجناس` = goods. Lists `Anbar_Jens` for one warehouse (`AJ_ID` = warehouse id — the default `93` is
a real warehouse number). Consumer: `AnbarCalaAddU.pas:95` reads `AJ_Maliat` from it.
**Reporting query.**

---

#### `Anbar_CardJensi` — stock card (item movement)
`Dmu.dfm:453-496`

| Parameter | ADO type | Size | Default |
|---|---|---|---|
| `@Coid` | `ftInteger` | — | `1397` |
| `@Code` | `ftInteger` | — | `Null` |
| `@D1` | `ftString` | 10 | `Null` |
| `@D2` | `ftString` | 10 | `Null` |

`کارت جنسی` = item ledger card: every inbound/outbound movement of item `@Code` in the period, with
a running quantity. **Encodes the stock-valuation/running-balance rule.** Caller:
`AnbarCardJensiU.pas`.

---

#### `Anbar_Mandeh` — stock on hand
`Dmu.dfm:721-750`

| Parameter | ADO type | Default |
|---|---|---|
| `@Coid` | `ftInteger` | `1397` |

`مانده انبار` = stock balance. Result columns (`Anbar_MandehU.dfm:1753-1814`) are
`R1`, `R2`, `TedIn1`, `TedIn2`, `TedOut1`, `TedOut2`, all `decimal(14,3)`, plus
`Mabin1/2`, `MabOut1/2`, `Phiin1/2`, `PhiOut1/2` as `bigint` — i.e. **two parallel
quantity/value pairs**, presumably two units of measure (`AJ_Vahed`, `AJ_Vahed2`) or two warehouses.
**Encodes the inventory-valuation rule. High extraction priority.**

---

#### `Anbar_AddToFactor` — add a line to an inventory invoice
`AnbarFactorU.dfm:433-…`, component `SP_AnbarAddToFactor`

| Parameter | ADO type | Size | Meaning |
|---|---|---|---|
| `@COID` | `ftInteger` | — | fiscal year |
| `@Type` | `ftInteger` | — | invoice type (`AF_Type`) |
| `@Factor` | `ftInteger` | — | invoice number |
| `@Date` | `ftString` | 10 | Jalali date |
| `@Customer` | `ftInteger` | — | counterparty (`Sarfasl.S_SSN`) |
| `@Code` | `ftInteger` | — | item code (`AJ_Code`) |
| `@Name` | `ftString` | 50 | denormalised item name |
| `@prop` | `ftString` | 50 | item property/spec (`AFD_Prop`) |
| `@Vahed` | `ftString` | 50 | unit of measure |
| `@Num` | **`ftFloat`** | — | quantity |
| `@Phi` | `ftLargeint` | — | unit price (rial) |
| `@Kol` | `ftLargeint` | — | line gross (rial) |
| `@kasr` | `ftLargeint` | — | deduction (rial) |
| `@Maliat` | `ftLargeint` | — | VAT (rial) |
| `@user` | `ftInteger` | — | user id |

**The second write procedure.** Note the client has *already* computed `@Kol`, `@kasr` and
`@Maliat` (§7.4, `AnbarFactorAddU.pas:107,168-170`) and passes them in — so the money arithmetic
lives in the **client**, and the procedure must not recompute it. It does maintain the invoice
header totals (`AF_Mab`, `AF_Kasr`, `AF_Maliat`, `AF_Total`) and the line count. `@Num` is
**`ftFloat`** — the one place a floating-point quantity crosses the wire (§7.3 says the column is
`decimal(_,3)`), so rounding at that boundary must be checked.

**Encodes business rules. High extraction priority.**

---

#### `Anbar_PrintFactor` — invoice print
`Dmu.dfm:532-567`, component `SP_AnbarPrintFactor`

| Parameter | ADO type | Default |
|---|---|---|
| `@COID` | `ftInteger` | `1396` |
| `@Factor` | `ftInteger` | `1` |

**Reporting query**, feeding FastReport.

---

#### `Anbar_ReportKharidForoosh` — purchase/sales report
`Dmu.dfm:497-531` and `AnbarReportKharidU.dfm:548-…` (component `SP1`)

| Parameter | ADO type | Size | Default (`Dmu.dfm`) | Default (`AnbarReportKharidU.dfm`) |
|---|---|---|---|---|
| `@D1` | `ftString` | 12 / **10** | `'1396/08/01'` | `'1396/01/01'` |
| `@D2` | `ftString` | 12 / **10** | `'1396/12/01'` | `'1396/12/29'` |
| `@Type` | `ftInteger` | — | `1` | `1` |

`خرید فروش` = purchase/sales. **The two declarations disagree on the parameter size** (12 vs 10) —
one of them was captured against an older version of the procedure. **No fiscal-year parameter**,
so the date range alone scopes it. **Reporting query.**

---

#### `MakeSanad_CheckDaryafti` — generate the voucher for a received cheque
Called as raw SQL only: `CheckDaryaftU.pas:356`

```pascal
Dm.Q1.SQL.Add(' Exec MakeSanad_CheckDaryafti '+ inttostr( tag ) ) ;
```

One positional parameter: the `DCheck.S_SSN` of the cheque. Generates the debit/credit voucher
lines for a received cheque (`چک دریافتی`). **Encodes business rules — this is posting logic.
Highest extraction priority.** Note the commented-out tail of the same line
(`// + ' and M_Sanad = ' + S_Sanad.Text`) shows the call was once a query.

---

#### `XNew` — server-side "today"
`Dmu.dfm:418-437`. `@COID ftInteger` (default `1`). Returns a result set with at least a
`CurrentDate` column — the current **Jalali** date as a string. Sole consumer
`TDM.Current_Date` (`Dmu.pas:1232-1239`, §6.2).

**This procedure contains the live Gregorian→Jalali conversion** and is therefore the single most
important body to extract (§6.3). The name and the `@COID` parameter suggest it once did more
(allocate a new document number?), and the extra result columns — if any — must be inspected.

---

#### `B_SelectSerial` — external purchase-receipt lookup
`Dmu.dfm:1143-1164`. **`Connection = ADO_RPPCSOLUTION`** — the only procedure on the external
connection. `@GhabzNo ftInteger` (default `0`). Returns exactly one row for a valid receipt,
including a `SerialNoPsnBts` column. Full behaviour in §5.4.

**Belongs to the external system.** Do not port; integrate.

---


---

[← 02-03-a-stored-procedures-overview.md](02-03-a-stored-procedures-overview.md) | [02-03-c-stored-procedures-functions-and-summary.md →](02-03-c-stored-procedures-functions-and-summary.md)
