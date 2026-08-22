_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 10. Accounting integration

Cross-reference `docs/03-accounting-core.md` for `Moein` / `DMoein` / `M_Tx` semantics; this
section covers only what the inventory domain contributes.

### 10.0 Three posting engines, three `M_Id` ranges, three link conventions

| Engine | Source documents | `M_Id` values | `M_Link` holds | Posted when |
|---|---|---|---|---|
| **A** — the stored procedure `Anbar_AddToFactor` | `Anbar_Factor` types 1–4 | `1` (range `1..9` reserved) | `AF_Factor` — **the number** | inline, on every save |
| **B** — `MakeSanadU` | `FactorMaster` `FM_ID ∈ {11,12,13,22}` | `31, 32, 35, 33` | `FM_SSN` — **the key** | manually, from `SodoorSanadU` |
| **C** — `FactorPesteh_U` | `FactorMaster` `FM_ID = 14` | `34` | `FM_Factor` — **the number** | inline, on receipt creation |

Nothing posts `FM_ID ∈ {15, 25, 16, 26, 21}` — production and transfer produce **no accounting
entry at all** (§3.2.4).

Every engine allocates its voucher number through `Dm.Get_NewSanad_DateID(date, idList)`
(`Dmu.pas:1461-1477`), which **merges into an existing draft voucher of the same Jalali date whose
lines are in the same `M_Id` family**, and only issues a new number when there is none. So a day's
inventory activity collapses into one voucher per family per date.

---

### 10.1 Engine A — subsystem A invoices

#### 10.1.1 The posting is inside a stored procedure whose body is unavailable

`B_SaveClick` (`AnbarFactorU.pas:567-667`) never inserts a `Moein` row. It **deletes** them
(`:621`):

```sql
Delete Moein where M_Coid=<year> and M_ID=1 and M_Link=<AF_Factor>
```

then loops `SP_AnbarAddToFactor` once per line (`:627-645`), then calls `DMoein_Make` and
`Dmoein_UpdateMab` on the new voucher number (`:647-649`). The only possible conclusion is that
`Anbar_AddToFactor` writes both the `Anbar_FactorD` row **and** the `Moein` voucher lines. Four
independent pieces of evidence:

1. Nothing else in the unit — or in `AnbarListU`, or in `Dmu` — inserts a `Moein` row with
   `M_ID` in `1..9`.
2. `DMoein_Make(AF_Sanad, …)` creates a voucher *header*; it is meaningless unless lines exist.
3. `Get_NewSanad_DateID` reserves the range `'1,2,3,4,5,6,7,8,9'` (`:593`), and
   `AR_DeleteClick` deletes `Moein … M_id in (1,2,3,4,5,6,7,8,9)` (`AnbarListU.pas:384`).
   Somebody writes those values.
4. `Anbar_Config.AC_Kharid`, `AC_BKharid`, `AC_Foroosh`, `AC_BForoosh`, `AC_Kasr`, `AC_Maliat`
   are **written by `AnbarTanzimU` and read by nothing in the repository**
   (verified by grep: only `AnbarTanzimU.pas:84-100, 179-184`). They exist to be read by the SP.

> **This is the single largest unknown in the inventory domain.** The exact debit/credit rules for
> subsystem A invoices are not derivable from source. **Open question §14** — recover
> `Anbar_AddToFactor` from the production database with
> `sp_helptext 'Anbar_AddToFactor'` or `SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_AddToFactor'))`.

#### 10.1.2 What can be established with certainty

**Parameters passed** (`AnbarFactorU.pas:628-643`), fifteen, once per line:

```
@COID   @Type   @Factor  @Date  @Customer
@Code   @Name   @prop    @Vahed @Num  @Phi  @Kol  @kasr  @Maliat
@user
```

**Not passed: `@Sanad`.** So the procedure must read the voucher number itself. It can:
`Dm.AnbarFactor.Post` at `:614` writes `AF_Sanad` **before** the loop starts at `:627`, so
`Select AF_Sanad From Anbar_Factor Where AF_Coid=@COID and AF_Factor=@Factor` is available to it.
That ordering is load-bearing and undocumented — **if the SP were ever called before the header
post, every line would post to voucher 0.**

**Also not passed: the warehouse.** The SP has only `@Code`, so it must resolve
`Anbar_Jens.AJ_Code → AJ_ID → Anbar_Config` itself to reach the six posting accounts. This is the
mechanism by which the item's *home warehouse* determines the accounts (§1.1).

**Also not passed: `@Total`.** `AFD_Total` is recomputed server-side (§6.1).

**The inferred posting shape**, from the account columns available and from the fact that
subsystem B's engine (which *is* readable) uses exactly the same six roles:

| `AF_Type` | Debit | Credit | Discount | VAT |
|---|---|---|---|---|
| 1 receipt | `AC_Kharid` (purchases) ← `AFD_Kol` | `AF_Customer` ← `AFD_Total` | `AC_Kasr` ← `AFD_Kasr` | `AC_Maliat` ← `AFD_Maliat` |
| 2 issue | `AF_Customer` ← `AFD_Total` | `AC_Foroosh` (sales) ← `AFD_Kol` | `AC_Kasr` | `AC_Maliat` |
| 3 purchase return | `AF_Customer` | `AC_BKharid` | `AC_Kasr` | `AC_Maliat` |
| 4 sales return | `AC_BForoosh` | `AF_Customer` | `AC_Kasr` | `AC_Maliat` |

**Mark this table as inference, not evidence.** It is what the account names and subsystem B's
behaviour imply. Verify against the SP body before porting.

#### 10.1.3 Established defects on this path regardless of the SP body

| # | Defect | Evidence |
|---|---|---|
| 1 | **The counterparty can be `0`.** The `S_Bed.Tag = 0` guard is unreachable (§4.2.2 step 2). Whatever the SP does with `@Customer = 0` — post to account id 0, or fail silently — the invoice saves. | `AnbarFactorU.pas:579-583` |
| 2 | **Only `M_ID = 1` is deleted on re-save, but `1..9` is reserved.** If the SP writes any other value in the range (e.g. `2` for a sales invoice) those lines are never removed and **double on every re-save**. | `AnbarFactorU.pas:621` vs `:593`, `AnbarListU.pas:384` |
| 3 | **Not transactional.** Delete at `:622` commits; a failure in the insert loop leaves the invoice with no lines and no voucher (§5.4). | `AnbarFactorU.pas:617-645` |
| 4 | **One posting round-trip per line.** For a 200-line invoice that is 200 stored-procedure calls, each opening its own implicit transaction. | `AnbarFactorU.pas:627-645` |
| 5 | **`DMoein_Make` runs after the lines, outside any transaction**, and the *old* voucher's total is recomputed only if `_OLD > 0`. | `AnbarFactorU.pas:647-649` |
| 6 | **The voucher narration is `'فروش کالا '` ("goods sale") for all four types**, including receipts and both returns. | `AnbarFactorU.pas:647` |

---


---

[← 9. Settlement (Tasfieh) (part b)](05-09-b-settlement-tasfieh.md) | [index](00-index.md) | [10. Accounting integration (part b) →](05-10-b-accounting-integration.md)
