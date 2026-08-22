_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 1. Entity model

Read §5.0 first: there are three physical databases and two independent inventory subsystems.
This section inventories the entities of each. Proposed English names follow
`docs/01-glossary.md` §7 and reuse `docs/02-data-model.md`; where that document already names a
table, the name here is taken from it, not invented.

### 1.0 Entity map

```
  ── subsystem A (main `arzi` catalog, owned by this application) ────────────
     Anbar_Config  1 ──< Anbar_Jens  1 ──< Anbar_FactorD >── 1  Anbar_Factor
     (warehouse)         (item)             (line)                (header)
                            │                                        │
                     AJ_VahedC │                              AF_Customer │
                            ▼                                        ▼
                      Anbar_Vahed                                Sarfasl
                      (unit of measure)                    (chart of accounts —
                                                            the counterparty IS
                                                            a leaf account node)

     Kinds  (pistachio grades — related to nothing by any FK; §8.1)

  ── subsystem B (separate `Anbar` catalog, owned by another application) ────
     Anbar  1 ──< Cala                    FactorKind 1 ──< FactorMaster 1 ──< FactorDetail
     (warehouse) (item, multi-warehouse   (document      (header)              (line)
                  via a CSV column)        type)

  ── weighbridge (separate `Rppc_Solution` catalog) ─────────────────────────
     NewRamz  (one row per delivery: ticket, lab result, price, net weight)
```

**Structural facts that shape everything below:**

- **The item master is global, not per fiscal year.** `Anbar_Jens` has no `*_COID` column
  (`AnbarCalaAddU.pas:173-175`, the insert, lists every column and there is no year). Same for
  `Anbar_Config`, `Anbar_Vahed` and `Kinds`. Only the transactional tables carry `COID`. This
  matches `Sarfasl` (`docs/01-glossary.md` §6b).
- **The warehouse dimension does not reach the movement lines in subsystem A.** `Anbar_Jens.AJ_ID`
  names the item's *home* warehouse, but `Anbar_FactorD` has no warehouse column and no stock query
  ever filters by one (§5.1.2 last bullet). Subsystem A behaves as a single stock pool per item
  code. Subsystem B has a real warehouse dimension (`FM_Anbar`, `FD_Anbar`).
- **There is no item category / group / family table anywhere.** `AJ_ID` (home warehouse) is the
  only grouping attribute, and the item-master screen presents it as a warehouse selector
  (`AnbarCalaU.pas:63-78`), not a category.
- **There is no lot, batch, serial or expiry tracking of any kind.** No column on `Anbar_Jens`,
  `Anbar_FactorD`, `Cala` or `FactorDetail` carries one. `docs/01-glossary.md` §2 lists
  "Serial | سریال | Serial / lot number" as a term — it appears only as `NewRamz.NR_Serial`, which
  is the weighbridge table's **identity primary key**, and `NR_Ramz`, which is a **blind lab code**
  (§8.3.2). Neither is a lot number. **Traceability does not exist.**
- **There is no counterparty table.** `Anbar_Factor.AF_Customer` is a `Sarfasl.S_SSN`, i.e. a leaf
  account node (`docs/01-glossary.md` §6b). `AF_CustomerN` denormalises its name.

---

### 1.1 `Anbar_Config` — warehouse

Maintained by `AnbarTanzimU` (§13). Global; no fiscal-year column.

| Legacy column | Type | Meaning | Persian label (source) | Proposed |
|---|---|---|---|---|
| `AC_ID` | int | warehouse id, allocated as `Max(AC_ID)+1` (`AnbarTanzimU.pas:216-219`) | — | `id` |
| `AC_Name` | varchar(50) | warehouse name | `نام انبار` "warehouse name" (`AnbarTanzimU.pas:213`) | `name` |
| `AC_DMaliat` | numeric | **VAT rate, in percent** — the only tax rate in the system | `AnbarTanzimU.dfm` `CA_DMaliat` | `vat_rate_pct` |
| `AC_Kharid` | int → `Sarfasl.S_SSN` | purchase account | `کد خرید` | `purchase_account_id` |
| `AC_BKharid` | int → `Sarfasl.S_SSN` | purchase-return account | `کد برگشت از خرید` | `purchase_return_account_id` |
| `AC_Foroosh` | int → `Sarfasl.S_SSN` | sales account | `کد فروش` | `sales_account_id` |
| `AC_BForoosh` | int → `Sarfasl.S_SSN` | sales-return account | `کد برگشت از فروش` | `sales_return_account_id` |
| `AC_Kasr` | int → `Sarfasl.S_SSN` | discount account | `کد تخفیف` | `discount_account_id` |
| `AC_Maliat` | int → `Sarfasl.S_SSN` | VAT account | `کد مالیات` | `vat_account_id` |

All six account links are written at `AnbarTanzimU.pas:179-184`, read back at `:84-100` through
the `Taraf` account-picker.

> **`Anbar_Config` is the posting-rule table.** Six accounts × N warehouses is how the inventory
> module decides what to debit and credit (§10). Because subsystem A's lines carry no warehouse,
> the *item's home warehouse* (`AJ_ID`) is what selects the account set — an indirection that only
> works while every item stays in one warehouse.

> **`AC_DMaliat` is a per-warehouse rate, not a per-item or per-date rate.** Changing it changes
> the VAT on every future line for every item in that warehouse and has no effective-date. Historic
> lines keep their stored `AFD_Maliat`, so the rate is effectively snapshotted per line — which is
> correct behaviour by accident.

**There is no `is_active` flag and no delete.** `AnbarTanzimU`'s popup declares three items `N1`,
`N2`, `N3` (`AnbarTanzimU.pas:21-23`) but implements only `N1Click` (add) and `N3Click` (rename).
**`N2` has no handler** — presumably "delete warehouse", never written. A warehouse, once created,
is permanent.

---

### 1.2 `Anbar_Jens` — item master

Maintained by `AnbarCalaAddU`; listed through the stored procedure `Anbar_AjnasView(@ID)`.
Global; no fiscal-year column.

| Legacy column | Type | Written at | Persian label | English | Proposed |
|---|---|---|---|---|---|
| `AJ_Code` | int | `AnbarCalaAddU.pas:153,173` | `کد کالا` | **Item code — the primary key.** Operator-assigned, no auto-numbering (§2) | `code` (+ surrogate `id`) |
| `AJ_ID` | int → `Anbar_Config.AC_ID` | `:154,173` | `انبار محل استقرار` | Home warehouse | `warehouse_id` |
| `AJ_Name` | varchar(80) | `:158,169` | `نام کالا` | Item name | `name` |
| `AJ_Prop` | varchar(50) | `:159,169` | `مشخصه فنی` | Technical specification | `specification` |
| `AJ_Vahed` | varchar(50) | `:160,170` | `واحد شمارش` | Unit of measure — **the label text, denormalised** | drop; join |
| `AJ_VahedC` | int → `Anbar_Vahed.AV_Code` | `:161,169` | (same control) | Unit of measure — the key | `unit_of_measure_id` |
| `AJ_Phi` | int | `:155,168` | `بهای فروش` | **Sale price** (not cost — §6.0) | `sale_price` |
| `AJ_Alarm` | int | `:156,168` | `حد اقل موجودی` | **Minimum stock level** | `min_stock` |
| `AJ_Maliat` | int 0/1 | `:147-149,168` | `مشمول مالیات` | Subject to VAT | `is_taxable` |
| `AJ_Manfi` | int 0/1 | `:150-152,168` | `منفی شدن موجودی` | **Negative stock permitted** — default `1` (§5.2.2, corrected) | `allow_negative_stock` |
| `AJ_UserID` | int | `:157,169` | — | Last modifying user | `updated_by` |
| `AJ_Net` | varchar(50) | `:162,170` | — | **Workstation name** of the last writer, `Util.GetComputerName` | `updated_from_host` |
| `AJ_DateTime` | datetime | `:170,175` | — | `GetDate()` at last write | `updated_at` |
| `SSTID` | varchar(13) | `:163,170` | `شناسه مالیاتی` | **Tax authority item identifier** (the 13-character Iranian "شناسه کالا/خدمت") | `tax_item_code` |

Derived / view-only columns returned by `Anbar_AjnasView` and shown in the grid
(`AnbarCalaU.dfm:145-260`):

| Column | Meaning |
|---|---|
| `AJ_PhiS` | `AJ_Phi` pre-formatted as a display string, grid title `قیمت` ("price") |

**What the item master does *not* have:**

| Absent concept | Consequence |
|---|---|
| `is_active` / discontinued flag | Items can only be hard-deleted (§2), and only if they have never been used |
| maximum stock / reorder quantity | Only `AJ_Alarm` (minimum) exists — and see below |
| category / group / family | No classification at all |
| barcode | none |
| purchase price / standard cost | none — §6.0 |
| second unit of measure, conversion factor | none. Subsystem B's `FD_VaznP` (weight) has no counterpart here |
| supplier / preferred vendor | none |
| lot / serial / expiry | none |
| `AJ_Alarm` enforcement | **The minimum-stock level is displayed, never checked.** `AnbarFactorAddU.pas:133` puts it into a read-only box `Rem2` beside the on-hand box `Rem1`, and that is the only use. No warning, no colour change, no report filters on it. It is a note to the operator. |

> **`AJ_ID` is nearly decorative.** It selects which `Anbar_Config` row the line editor reads for
> the warehouse name and VAT rate (`AnbarFactorAddU.pas:138-146`), and it orders the item-search
> results (`Dmu.dfm:604-628`, `Order by AJ_ID, AJ_Code`). It does **not** partition stock: every
> balance query in §5.1 sums `Anbar_FactorD` by `AFD_Code` alone. Moving an item between
> warehouses by editing `AJ_ID` retroactively re-attributes its entire movement history and
> changes its VAT rate and posting accounts for all future lines, with no migration document.

---

### 1.3 `Anbar_Vahed` — unit of measure

`select * from Anbar_Vahed Order By AV_Name` (`AnbarCalaAddU.dfm:267-268`), bound to a
`TDBLookupComboBox` with `KeyField = 'AV_Code'`, `ListField = 'AV_Name'` (`:248-249`).

| Column | Meaning | Proposed |
|---|---|---|
| `AV_Code` | key, `int` (the code writes it as `inttostr(AJ_Vahed.KeyValue)`, `AnbarCalaAddU.pas:161`) | `id` |
| `AV_Name` | Persian label, e.g. `کیلوگرم` (kilogram) | `name` |

**There is no maintenance screen for it.** No unit in the repository inserts, updates or deletes
`Anbar_Vahed`; the table is read-only from this application's point of view and must be populated
by direct SQL. Its row set is therefore not recoverable from source (**open question §14**); the
only value observed in code is `کیلوگرم` (§5.2.2 point 3).

There is no conversion factor, no base unit and no decimal-precision attribute — which is why the
`.AsInteger` truncations of §5.2.2 and §6.4 are damaging: the system cannot tell a kilogram from a
"each".

---

### 1.4 `Kinds` — pistachio product grades

`TableName = 'Kinds'` (`Dmu.dfm:301-307`), columns `K_id` / `K_name`. Fully documented in §8.1,
including the seven-value enumeration and the fact that the same integer doubles as an item code
and as an account-code segment. Not related to any other table by a foreign key. **The only
consumer is a dead form** (`PestehD_U.pas:94-96`).

| Column | Meaning | Proposed |
|---|---|---|
| `K_id` | grade id, `1..7` | `id` |
| `K_name` | Persian grade name | `name` |

Proposed table name: `pistachio_grades` (§16). Do **not** call it `kinds` or `account_types`.

---

### 1.5 `Anbar_Factor` / `Anbar_FactorD` — invoice header and line

Full column inventories in §3.1.1 and §3.1.2; not repeated here. Key points for the entity model:

- The header→line relationship is by **business number** (`AFD_Factor = AF_Factor`, scoped by
  `AFD_Coid`), not by the surrogate key `AF_SSN`. `AF_SSN` is used only by `Moadian` and by the
  settlement screen (§9).
- `AFD_Type`, `AFD_Date` and `AFD_Customer` are denormalised copies of header columns, and the
  stock and costing queries read the copies, never the header.
- Twenty columns on the header (`AF_Sel2-5`, `AF_Mab1-5`, `AF_Desc1-5`, `AF_Date1-5`) are dead.

---

### 1.6 Subsystem B entities (external `Anbar` catalog — read-mostly)

`arzi` reads these and writes exactly one row set into them (the pistachio receipt, §8.3.4).
No DDL is available; columns are recovered from `Select *` field lists.

| Table | Role | Columns documented at |
|---|---|---|
| `Anbar` | warehouse master | not read by name in this repo; warehouse ids appear as literals (`17`) and inside `Cala.C_Anbar` |
| `Cala` | item master | `C_Code`, `C_Name`, `C_Prop`, `C_Vahed`, `C_Anbar` — §8.3.3 |
| `FactorKind` | document-type table | `FK_ID`, `FK_InOut`, `FK_Name`, `FK_UserList`, `FK_AnbarList`, `FK_Enable` — §3.2 |
| `FactorMaster` | document header | §3.2.1 |
| `FactorDetail` | document line | §3.2.2 |

> **`Cala.C_Anbar` is a multi-warehouse membership list stored as a delimited string.** The only
> query against it is
> `Where C_code=<n> and ( C_Anbar like '%,17,%')` (`FactorPesteh_U.pas:137`). So one item row can
> belong to many warehouses, encoded as `,1,4,17,` — the exact opposite of subsystem A, where
> `Anbar_Jens.AJ_ID` is a single scalar. **The two item masters are structurally incompatible**
> and merging them (§15) requires a real `item_warehouses` junction table.

> **`Cala` and `Anbar_Jens` are separate item masters with separate code spaces.** Nothing keeps
> them in sync. The pistachio path resolves grade `5` against `Cala`; the invoice path resolves
> `AJ_Code` against `Anbar_Jens`. An item can exist in one and not the other.

---

### 1.7 Weighbridge entity — `Rppc_Solution.Dbo.NewRamz`

One row per pistachio delivery, carrying the weighbridge ticket, the lab assessment, the agreed
price and the resulting net weight. Complete column inventory at §8.3.1, state machine at §8.3.2.
Written by the weighbridge application; `arzi` reads it and updates three columns
(`NR_State`, `NR_Resid`, `NR_Sanad`) when it issues a receipt.

Proposed English name: `pistachio_deliveries` (§16).

---

### 1.8 Full legacy → proposed name map for §1 entities

Consistent with `docs/02-data-model.md`; the complete identifier map including columns not
covered here is §16.

| Legacy | Subsystem | Proposed table | Notes |
|---|---|---|---|
| `Anbar_Config` | A | `warehouses` | + six FK columns to `accounts` |
| `Anbar_Jens` | A | `items` | add surrogate `id`; `code` becomes a unique attribute |
| `Anbar_Vahed` | A | `units_of_measure` | needs a maintenance screen (§15) |
| `Kinds` | A | `pistachio_grades` | **not** account types |
| `Anbar_Factor` | A | `inventory_documents` | merged with `FactorMaster` (§15) |
| `Anbar_FactorD` | A | `inventory_document_lines` | merged with `FactorDetail` (§15) |
| `Anbar.Dbo.Anbar` | B | `warehouses` | same table after merge |
| `Anbar.Dbo.Cala` | B | `items` | same table after merge; `C_Anbar` becomes `item_warehouses` |
| `Anbar.Dbo.FactorKind` | B | `document_types` | or a Rust enum, §3.4 |
| `Anbar.Dbo.FactorMaster` | B | `inventory_documents` | |
| `Anbar.Dbo.FactorDetail` | B | `inventory_document_lines` | + `weight` column |
| `Rppc_Solution.Dbo.NewRamz` | weighbridge | `pistachio_deliveries` | |
| `Sarfasl` (as counterparty) | A | `accounts` | see `docs/03-accounting-core.md`; a party **is** a leaf account |


---

_start_ | [index](00-index.md) | [2. Item master CRUD rules (part a) →](05-02-a-item-master-crud-rules.md)
