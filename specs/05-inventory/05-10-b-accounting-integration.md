_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 10.2 Engine B — `MakeSanadU`, the readable one

`SodoorSanadU.B_SodoorClick` (`SodoorSanadU.pas:167-208`) routes by `FM_ID` (§3.2.4) into one of
four builders. Each builder:

1. reads the `FactorMaster` header by `FM_SSN` (`Select * … Where FM_SSN=<n>`);
2. reads the **warehouse's** account configuration from `Anbar.DBo.Anbar Where A_Code = FM_Anbar`;
3. appends 2–4 rows into an in-memory `VSanad` table, which the operator sees in a grid and can
   review before confirming;
4. `B_OkClick` (`:75-132`) then writes them to `Moein`.

**Subsystem B has its own account-configuration table**, `Anbar.Dbo.Anbar`, with columns
`A_Code`, `A_Aval`, `A_Kharid`, `A_Foroosh`, `A_BForoosh`, `A_Kasr`, `A_Maliat`
(`MakeSanadU.pas:219, 354, 669, 488, 266/401/559/646, 291/426/512/693`). It is
**parallel to and independent of `Anbar_Config`** (§1.1) — the same six-ish roles, a different
table, in a different database, edited by a different application. `arzi` cannot maintain it.

#### 10.2.1 `init11` — opening stock (`FM_ID = 11` → `M_Id = 31`)

Caption `صدور سند موجودی اول دوره` ("issue opening-stock voucher", `MakeSanadU.pas:190`).
Default narration `' عملیات انبار مورخ ' + FM_Date` ("warehouse operations dated …", `:203`).

| # | Line | Account source | Debit | Credit | Narration | `.pas` |
|---|---|---|---|---|---|---|
| 1 | Opening stock | `Anbar.A_Aval` | `FM_Mab` | — | `بابت رسید شماره <FM_Factor> <FM_Desc>` | `:219-238` |
| 2 | Counterparty | `FM_TSSN` | — | `FM_Total` | `رسید شماره <FM_Factor> <FM_Desc>` | `:242-261` |
| 3 | Discount, only if `FM_Kasr > 0` | `Anbar.A_Kasr` | `FM_Kasr` | — | `تخفیف رسید شماره …` | `:264-286` |
| 4 | VAT | `Anbar.A_Maliat` | — | `FM_Maliat` | `مالیات رسید شماره …` | `:289-311` |

Validation messages, in order: `کد اول دوره برای انبار تعریف نشده است` ("the opening-stock code is
not defined for the warehouse", `:222`), `کد طرف حساب برای انبار تعریف نشده است`
("the counterparty code is not defined for the warehouse", `:246`),
`کد تخفیفات برای انبار تعریف نشده است` ("the discount code is not defined for the warehouse",
`:269`), `کد مالیات برای انبار تعریف نشده است` ("the VAT code is not defined…", `:294`).

#### 10.2.2 `init12` — purchase (`FM_ID = 12` → `M_Id = 32`)

Caption `صدور سند خرید مواد و کالا` ("issue materials-and-goods purchase voucher", `:325`).
**Structurally identical to `init11`**, with `A_Aval` replaced by `A_Kharid` and the message
`کد خرید مواد و کالا برای انبار معرفی نشده است` ("the materials-and-goods purchase code is not
registered for the warehouse", `:357`).

| # | Line | Account | Debit | Credit | `.pas` |
|---|---|---|---|---|---|
| 1 | Purchases | `Anbar.A_Kharid` | `FM_Mab` | — | `:354-373` |
| 2 | Supplier | `FM_TSSN` | — | `FM_Total` | `:377-396` |
| 3 | Discount, if `> 0` | `Anbar.A_Kasr` | `FM_Kasr` | — | `:399-421` |
| 4 | VAT | `Anbar.A_Maliat` | — | `FM_Maliat` | `:424-446` |

#### 10.2.3 `init22` — sale (`FM_ID = 22` → `M_Id = 33`)

Caption `صدور سند فروش` ("issue sales voucher", `:594`).

| # | Line | Account | Debit | Credit | `.pas` |
|---|---|---|---|---|---|
| 1 | Customer | `FM_TSSN` | `FM_Total` | — | `:622-641` |
| 2 | Discount, if `> 0` | `Anbar.A_Kasr` | `FM_Kasr` | — | `:644-666` |
| 3 | Sales | `Anbar.A_Foroosh` | — | `FM_Mab` | `:669-688` |
| 4 | VAT, if `> 0` | `Anbar.A_Maliat` | — | `FM_Maliat` | `:691-713` |

#### 10.2.4 `init13` — sales return (`FM_ID = 13` → `M_Id = 35`)

Caption `صدور سند برگشت از فروش` ("issue sales-return voucher", `:460`). The exact mirror of
`init22`:

| # | Line | Account | Debit | Credit | `.pas` |
|---|---|---|---|---|---|
| 1 | Sales returns | `Anbar.A_BForoosh` | `FM_Mab` | — | `:488-507` |
| 2 | VAT, if `> 0` | `Anbar.A_Maliat` | `FM_Maliat` | — | `:510-532` |
| 3 | Customer | `FM_TSSN` | — | `FM_Total` | `:535-554` |
| 4 | Discount, if `> 0` | `Anbar.A_Kasr` | — | `FM_Kasr` | `:557-579` |

**There is no `init` for a purchase return.** `A_BKharid` has no equivalent on this path, and
`B_SodoorClick` has no branch for a `2x` purchase-return type. Subsystem B can receive purchase
returns as documents but can never post them.

#### 10.2.5 Defect — purchase and opening-stock vouchers do not balance

Let `Mab` = gross, `Kasr` = discount, `Maliat` = VAT, and (from §6.1)
`Total = Mab + Maliat − Kasr`.

| Builder | Σ debits | Σ credits | Difference |
|---|---|---|---|
| `init22` sale | `Total + Kasr` = `Mab + Maliat` | `Mab + Maliat` | **0 — correct** |
| `init13` sales return | `Mab + Maliat` | `Total + Kasr` = `Mab + Maliat` | **0 — correct** |
| `init12` purchase | `Mab + Kasr` | `Total` = `Mab + Maliat − Kasr` | **`2·Kasr − Maliat`** |
| `init11` opening stock | `Mab + Kasr` | `Total` | **`2·Kasr − Maliat`** |

Two separate faults produce this:

1. **The VAT line is switched off.** `MakeSanadU.pas:289` and `:424` read, verbatim:
   ```pascal
   // maliat
       if false then //QS.FieldValues['FM_Maliat'] >0 then
   ```
   The condition has been replaced by the literal `false` and the real test commented out **in
   both inbound builders**. `init13` and `init22` still have the live test (`:510`, `:691`). So an
   inbound document's VAT is inside `FM_Total` (and therefore inside the credit to the supplier)
   but has **no corresponding debit to the VAT account**. Input VAT is never recognised.
2. **The discount is debited on the inbound side.** `init12:411` and `init11:276` set
   `M_Bed := FM_Kasr`. On a purchase, a discount reduces the cost and belongs on the **credit**
   side — which is exactly what `init13:570` does for the mirror case. The inbound builders have
   the sign inverted.

**Worked arithmetic.** A purchase of 10 000 000 rial with a 500 000 discount and 9 % VAT on the
net:

```
FM_Mab    = 10 000 000
FM_Kasr   =    500 000
FM_Maliat = trunc((10 000 000 − 500 000) × 9 / 100) =   855 000
FM_Total  = 10 000 000 + 855 000 − 500 000          = 10 355 000

posted by init12:
   Dr  Purchases (A_Kharid)   10 000 000
   Dr  Discounts (A_Kasr)        500 000
   Cr  Supplier (FM_TSSN)                  10 355 000
   -------------------------------------------------
   Σ Dr = 10 500 000     Σ Cr = 10 355 000     out by 145 000

check: 2·Kasr − Maliat = 1 000 000 − 855 000 = 145 000  ✓

the correct entry would be:
   Dr  Purchases              10 000 000
   Dr  Input VAT (A_Maliat)      855 000
   Cr  Supplier                            10 355 000
   Cr  Discounts (A_Kasr)                     500 000
```

**Severity: critical if inbound documents ever carry a discount or VAT; invisible if they never
do.** Since production and transfer are unposted anyway, and the pistachio path hard-zeroes both
(§8.3.4), it is entirely possible no such document exists in production — which would explain why
it was never noticed. **Query to settle it (§14):**
`Select count(*) From Anbar.Dbo.FactorMaster Where FM_ID in (11,12) and (FM_Kasr <> 0 or FM_Maliat <> 0)`.

`MakeSanadU` has **no balance check of its own**. The grid shows debit and credit totals
(`G1.RecalculateSummaryResults`, `:145,316,451,585,718`) and `B_OkClick` (`:75-132`) begins with
the comment `// Control data` followed by **nothing** — the validation was never written.

#### 10.2.6 Other findings in `MakeSanadU`

| Finding | Evidence |
|---|---|
| **`SN` is used before it is assigned.** In `init11` the counterparty short name `SN` is concatenated into line 1's narration at `:231-232` but only assigned at `:243`. Same in `init12` (`:367` vs `:378`) and `init13` (`:501` vs `:536`). Delphi zero-initialises local strings, so the effect is a trailing space instead of the party name — a cosmetic but permanent defect in the stored narration. | `:231, 243, 367, 378, 501, 536` |
| **`_ID` is read from the document and immediately discarded.** `_ID := QS.FieldByName('FM_ID').AsInteger;` then `_ID := 31;` (`:204-206`, and identically at `:339-341`, `:474-476`, `:608-610`). The document's own type is never used. | as cited |
| **`VSanad.Append` before validation.** In `init22` the sales line (`:670-675`) and the VAT line (`:694-699`) call `Append` *before* checking `Taraf.Get_SSn <= 0`, so a failed validation exits with the dataset in insert mode. | `:670, 694` |
| **`B_OkClick` is not transactional.** Three separate `ExecSQL` calls — delete old lines, insert new lines (one round trip per line), back-fill `M_Ko/M_Mo/M_Ta1/M_Ta2` — plus the `FactorMaster` update and `DMoein_Make`. No `Begin Transaction`. | `:81-127` |
| **`Taraf.Set_SSN` inside the insert loop is a no-op.** `:103` resolves the account but its result is never read; the parameters were already bound at `:97-102`. Dead call, one extra query per line. | `:103` |
| **The `M_Ko/M_Mo/M_Ta1/M_Ta2` back-fill is scoped by voucher, not by `M_Id` or `M_Link`.** `:112-114` updates every line of `M_Sanad` — including lines merged in from another document by `Get_NewSanad_DateID`. Harmless (it rewrites them to the same values) but wasteful and fragile. | `:110-115` |
| **The idempotency delete covers `31..39` and is not scoped by `M_Sanad`.** This is what rescues the un-post asymmetry of §5.3. | `:84-86` |

---

### 10.3 Engine C — the pistachio receipt

Fully documented at §8.4. Summary: two lines, `M_Id = 34`, `M_Link = FM_Factor`,
`Dr 700-3-<grade>` / `Cr 301-1-<supplier>`, both for `NR_Kol`, account codes hard-coded as string
prefixes, no discount, no VAT, no reversal.

Its `M_Link` convention (document **number**) differs from Engine B's (`FM_SSN`), which means
Engine B's un-post could never find Engine C's lines even if `34` were in its `IN` list — the
compounding cause of the "Critical" row in §5.3.3.

---

### 10.4 How the link back to the source document is stored

Five distinct mechanisms coexist. This is the answer to "how is the link stored", and the answer
is "inconsistently".

| Direction | Mechanism | Scoping | Evidence |
|---|---|---|---|
| voucher line → subsystem A invoice | `Moein.M_Link = AF_Factor`, `M_ID = 1` | `M_Coid` | `AnbarFactorU.pas:621`; `AnbarListU.pas:384, 446` |
| subsystem A invoice → voucher | `Anbar_Factor.AF_Sanad` | `AF_COID` | `AnbarFactorU.pas:609` |
| voucher line → subsystem B document | `Moein.M_Link = FM_SSN`, `M_Id ∈ {31,32,33,35}` | `M_Coid` | `MakeSanadU.pas:85, 93`; `SodoorSanadU.pas:257` |
| subsystem B document → voucher | `FactorMaster.FM_SanadNo` + `FM_SanadDate` + `FM_Lock = 2` | `FM_Coid` | `MakeSanadU.pas:121-123` |
| voucher line → pistachio receipt | `Moein.M_Link = FM_Factor`, `M_Id = 34` | `M_Coid` | `FactorPesteh_U.pas:224, 226` |
| treasury → subsystem A invoice | `DFish.S_LinkSSN` / `DCheck.S_LinkSSN` = `AF_Factor`, with `S_LinkPRG = 1` | `S_COID` | `AnbarListU.pas:356, 368, 450-454, 538-539` — §4.2.6 |
| tax e-invoicing → subsystem A invoice | `Moadian.M_Link = AF_SSN`, `M_Id = 1` | none | `AnbarListU.pas:537` |

> **`Moadian` is read-only from here.** The `Send` column on the invoice list counts rows in
> `Moadian` for the invoice; **nothing in this repository ever inserts into `Moadian`.** The
> Iranian tax-authority e-invoicing submission is done by some other tool (or not at all) and
> `arzi` only reports whether a row exists. `Anbar_Jens.SSTID` (the 13-character tax item code,
> §1.2) exists for the same integration and is likewise never read by anything. Flag both for
> §14.

> **Missing scoping.** `SodoorSanadU.dfm:526-528` correlates treasury to a subsystem-B document
> with `Where s_Coid=@Coid and S_linkSSN=FM.FM_factor` — **with no `S_LinkPRG` filter at all**,
> unlike `AnbarListU.pas:538-539` which does filter `S_Linkprg=1`. Since `S_LinkPRG = 1` means
> "subsystem A invoice", the `SodoorSanadU` list will attribute an inventory invoice's deposit
> slip to any `FactorMaster` document that happens to share its number. `FM_Factor` for warehouse
> 17 starts at 1 700 001, but other warehouses number from low values, so collision with
> `AF_Factor` is plausible. **Defect: cross-module payment totals on the subsystem-B list can be
> wrong.**

---

### 10.5 Events that generate no accounting entry

| Event | Why | Consequence |
|---|---|---|
| **Production input (`FM_ID = 25`) and output (`FM_ID = 15`)** | `B_SodoorClick` has no branch — `' Not implemented yet. '` (`SodoorSanadU.pas:203`) | Raw materials are consumed and finished goods appear with **no cost transfer, no WIP, no variance**. §3.2.4 |
| **Inter-warehouse transfer (`FM_ID = 16/26`)** | same | No value moves between warehouse accounts |
| **`FM_ID = 21`** | same | unknown document type, never posted |
| **Purchase returns in subsystem B** | no builder exists; `A_BKharid` is unused | Cannot be posted at all |
| **Input VAT on subsystem B receipts** | `if false then` (§10.2.5) | Input VAT never recognised |
| **Cost of goods sold** | no engine computes it (§6.0) | Gross margin is not derivable from the ledger |
| **Inventory valuation / stock account** | there is no stock or inventory asset account in `Anbar_Config` or `Anbar.Anbar` — the roles are `purchase`, `sale`, `returns`, `discount`, `VAT`, `opening` | **The system uses a periodic, not perpetual, inventory model in the ledger**, while computing perpetual quantities in the warehouse (§5.1). The two never reconcile. |

That last row is the structural summary: **the ledger is periodic and the warehouse is perpetual,
and nothing joins them.** Stock quantity lives in `Anbar_FactorD`; stock value lives nowhere.

---

### 10.6 Requirements for the rebuild

1. **One posting engine**, in the service layer, driven by a declarative rule table keyed on
   `(document_type, line_role)` → `(debit_account, credit_account, amount_source)`. The three
   engines above are the same eight rules written three times.
2. **One link convention**: `voucher_lines.source_document_id` and
   `voucher_lines.source_document_line_id`, both surrogate FKs with declared referential action.
   Never a document number.
3. **A balance assertion in the engine.** `Σ debits = Σ credits` per voucher, enforced before
   commit. §10.2.5 could not have shipped with one.
4. **Post and un-post as one symmetric, transactional operation** over a single declared set of
   document types (§5.3, closing paragraph).
5. **Decide on periodic vs perpetual.** If perpetual, add a stock/inventory asset account per
   warehouse and post COGS on issue at the costing method chosen in §6/§15. This is a behaviour
   change and belongs in §15, but note that porting the current behaviour means porting a ledger
   from which inventory value is absent.
6. **Post production and transfer**, or explicitly decide not to. Today's silence is not a
   decision, it is `' Not implemented yet. '`.


---

[← 10. Accounting integration (part a)](05-10-a-accounting-integration.md) | [index](00-index.md) | [11. Stock card and stock balance →](05-11-stock-card-and-balance.md)
