_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 3. Document types

Two disjoint document-type enumerations exist, one per subsystem (§5.0). They overlap
semantically, share no values, and are maintained in completely different ways: subsystem A's is
**hard-coded in Pascal**, subsystem B's is a **table** (`Anbar.Dbo.FactorKind`).

---

### 3.1 Subsystem A — `Anbar_Factor.AF_Type`, four values, hard-coded

The full enumeration is at §5.1.1 and is not repeated. Summary for reference:

| `AF_Type` | Direction | List-screen label | Invoice-screen label | Proposed |
|---|---|---|---|---|
| 1 | inbound `+` | `رسید انبار` goods receipt | `رسيد انبار` | `receipt` |
| 2 | outbound `−` | `حواله انبار` goods issue | `فاکتور فروش کالاو خدمات` goods & services sales invoice | `issue` |
| 3 | outbound `−` | `برگشت از خرید` return to supplier | `برگشت رسيد انبار` reversal of receipt note | `purchase_return` |
| 4 | inbound `+` | `برگشت از فروش` return from customer | `برگشت حواله انبار` reversal of issue note | `sales_return` |

Sources: `AnbarListU.pas:181-184` (grid display), `AnbarListU.pas:540-541` (the same mapping as a
SQL `CASE`), `AnbarFactorU.pas:116-120` (a third, divergent copy with a self-overwriting bug).

**There is no adjustment, no write-off, no stock count, no transfer and no production document in
subsystem A.** An adjustment can only be expressed as a type-1 or type-2 invoice against some
counterparty account, which forces the operator to choose an account for what is not a
transaction. Flag for §15.

#### 3.1.1 Header — `Anbar_Factor`

Complete column list, from the persistent field definitions at `AnbarListU.pas:46-80` (the list
query is `Select *`, so this is the whole table) plus the write path
(`AnbarFactorU.pas:606-614, 654-661`).

| Column | Type | Written where | Meaning | Proposed |
|---|---|---|---|---|
| `AF_SSN` | identity | server | surrogate key. **Used only by `Moadian` and by the settlement screen** (§9) | `id` |
| `AF_COID` | int | `:606` | fiscal year | `fiscal_year_id` |
| `AF_Type` | int | `:607` | 1–4, above | `document_type` |
| `AF_Factor` | int | `:608` | invoice number — the *de facto* business key, mutable (§4.2.6) | `invoice_number` |
| `AF_Sanad` | int | `:609` | voucher number; reassigned on every save (§4.2.2 step 4) | `voucher_id` |
| `AF_Date` | varchar(10) | `:610` | Jalali date string `YYYY/MM/DD` | `date_jalali` + derived `date` |
| `AF_Customer` | int | `:611` | counterparty `Sarfasl.S_SSN`. **Can be `0`** — §4.2.2 step 2 | `party_account_id` |
| `AF_CustomerN` | varchar | `:612` | denormalised counterparty name from the global `Taraf` object | drop; join |
| `AF_Desc` | varchar | `:613` | free narration | `description` |
| `AF_Mab` | bigint | `:660` | **gross** total = `Sum(AFD_Kol)` | `gross_amount` |
| `AF_Kasr` | bigint | `:656` | total discount = `Sum(AFD_Kasr)` | `discount_amount` |
| `AF_Maliat` | bigint | `:658` | total VAT = `Sum(AFD_Maliat)` | `tax_amount` |
| `AF_Total` | bigint | `:654` | **net** total = `Sum(AFD_Total)` | `total_amount` |
| `AF_Sel2` … `AF_Sel5` | int | **never** | — | drop |
| `AF_Mab1` … `AF_Mab5` | bigint | **never** | — | drop |
| `AF_Desc1` … `AF_Desc5` | varchar | **never** | — | drop |
| `AF_Date1` … `AF_Date5` | nvarchar | **never** | — | drop |

> **Twenty dead columns.** `AF_Sel2-5`, `AF_Mab1-5`, `AF_Desc1-5`, `AF_Date1-5` are declared as
> persistent `TField`s on the list query (`AnbarListU.pas:59-77`) — which is how we know they
> exist in the table — and are **read and written by nothing in the repository**. Grep across all
> `.pas` and `.dfm` returns only those declarations. The shape (five parallel slots of amount +
> description + date + a selector flag) strongly suggests an abandoned inline-settlement design
> that was later replaced by `DFish`/`DCheck` (§9), but that is inference. Do not migrate them
> without checking production data for non-null values — **open question §14**.
>
> Note the naming trap already flagged in §4.2.2: at header level `AF_Mab` is *gross* and
> `AF_Total` is *net*; at line level `AFD_Kol` is *gross* and `AFD_Total` is *net*. So `Mab` and
> `Kol` mean the same thing at different levels. §16.

**Numbering scheme.** One sequence per fiscal year across all four types:
`Select isnull(Max(AF_Factor),0)+1 … Where AF_COid = <year>` (`Dmu.pas:1253-1262`). No lock, no
unique constraint, no prefix, no per-type series. Concurrency hazard per §5.4; the only duplicate
check anywhere is on the renumber screen (`AnbarListU.pas:423-429`).

#### 3.1.2 Line — `Anbar_FactorD`

The write path is the stored procedure `Anbar_AddToFactor` (§6.1); its body is not in the
repository, so the column list is assembled from the parameter list
(`AnbarFactorU.dfm:433-546`), the read path (`AnbarFactorU.pas:719-735`) and the aggregate queries
(§5.1.2, §5.1.3, `AnbarFactorU.pas:654-661`).

| Column | Source | Meaning | Proposed |
|---|---|---|---|
| `AFD_SSN` | identity | surrogate key. **Not stable across edits** (§5.4) | `id` |
| `AFD_Coid` | `@COID` | fiscal year | `fiscal_year_id` |
| `AFD_Type` | `@Type` | copied from `AF_Type` — the direction (§5.1.1) | denormalised; derive |
| `AFD_Factor` | `@Factor` | copied from `AF_Factor` — **the link to the header is by number, not by `AF_SSN`** | `invoice_id` |
| `AFD_Date` | `@Date` | copied from `AF_Date` (§5.1.2) | denormalised; derive |
| `AFD_Customer` | `@Customer` | copied from `AF_Customer` — inferred, name not otherwise observed | denormalised; derive |
| `AFD_Code` | `@Code` | `Anbar_Jens.AJ_Code` | `item_id` |
| `AFD_Name` | `@Name` varchar(50) | denormalised `AJ_Name` | drop; join |
| `AFD_Prop` | `@prop` varchar(50) | denormalised `AJ_Prop` | drop; join |
| `AFD_Vahed` | `@Vahed` varchar(50) | denormalised `AJ_Vahed` | drop; join |
| `AFD_Num` | `@Num` float(15) | quantity, `Numeric(14,3)`, always positive | `quantity` |
| `AFD_Phi` | `@Phi` bigint | unit price | `unit_price` |
| `AFD_Kol` | `@Kol` bigint | gross = `trunc(Num × Phi)` | `gross_amount` |
| `AFD_Kasr` | `@kasr` bigint | discount | `discount_amount` |
| `AFD_Maliat` | `@Maliat` bigint | VAT | `tax_amount` |
| `AFD_Total` | **computed in the SP** | net = `Kol + Maliat − Kasr` (per the client formula) | `total_amount` |
| `AFD_UserID` | `@user` | creating user — inferred | `created_by` |

**The header→line link is `AFD_Factor = AF_Factor` scoped by `AFD_Coid`** — a mutable business key
again, which is exactly why the renumber screen has to rewrite `Anbar_FactorD`
(`AnbarListU.pas:443-444`).

`AFD_Type` and `AFD_Date` are denormalised copies of header values, and every stock query reads
them rather than joining (§5.1.2, §5.1.3). They are consistent only because the save path deletes
and re-inserts all lines (§4.2.2 step 7/8); nothing enforces it.

#### 3.1.3 Business rules per type

| Rule | 1 receipt | 2 issue | 3 purchase return | 4 sales return |
|---|---|---|---|---|
| Stock effect | `+` | `−` | `−` | `+` |
| Negative-stock check applied | no | **yes**, unless `AJ_Manfi = 1` (§5.2) | **no — gap** | no |
| Enters the average cost | **yes** | no | no | no (§6.2) |
| Settlement (`AR_Variz`) allowed | no | **yes only** (`AnbarListU.pas:476-480`) | no | no |
| Permission code | 1404 | 1405 | 1407 | 1406 |
| Default unit price | `AJ_Phi`, the **sale** price, on all four (`AnbarFactorAddU.pas:125`) | | | |
| VAT applied | on all four, from `Anbar_Config.AC_DMaliat` if `AJ_Maliat = 1` (§7) | | | |
| Voucher `M_ID` | `1` (deleted on re-save); allocator reserves `1..9` | | | |
| Voucher narration | `'فروش کالا '` ("goods sale") on all four (`AnbarFactorU.pas:647`) | | | |

Everything else — validation, save path, print, delete, renumber — is identical across the four
types. **The type is a sign and a label, nothing more.** There is no per-type field set, no
per-type numbering, no per-type workflow.

---


---

[← 2. Item master CRUD rules (part b)](05-02-b-item-master-crud-rules.md) | [index](00-index.md) | [3. Document types (part b) →](05-03-b-document-types.md)
