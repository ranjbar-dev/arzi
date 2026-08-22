_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 3.2 Subsystem B — `FactorMaster.FM_ID`, a data-driven type table

`Anbar.Dbo.FactorKind` (`SodoorSanadU.dfm:533`, joined as `FK`):

| Column | Type | Meaning |
|---|---|---|
| `FK_ID` | int | the document type id, matched to `FactorMaster.FM_ID` |
| `FK_InOut` | int | direction, `1` inbound / `2` outbound |
| `FK_Name` | varchar(50) | Persian label |
| `FK_UserList` | varchar(200) | comma-delimited allow-list of user ids |
| `FK_AnbarList` | varchar(200) | comma-delimited allow-list of warehouse ids |
| `FK_Enable` | smallint | active flag |

**The table's rows are not in this repository** (it lives in the external `Anbar` database).
`arzi` never reads `FK_UserList`, `FK_AnbarList` or `FK_Enable` for any decision — it selects them
into persistent fields (`SodoorSanadU.dfm:621-641`) and displays only `FK_Name`
(`:390, 1224, 1247`). **The type table's own permission model is inert as far as this application
is concerned.**

The id values are recoverable from the code that branches on them:

| `FM_ID` | Direction | Persian (source) | English | Evidence |
|---|---|---|---|---|
| **11** | in | `اول دوره` (radio `FK261611`, `SodoorSanadU.dfm:122`); `صدور سند موجودی اول دوره` (`MakeSanadU.pas:186`) | Opening stock | posted by `init11` → `M_Id = 31` |
| **12** | in | `رسید انبار` (radio `FK1214`, `:130`); `صدور سند خرید مواد و کالا` (`MakeSanadU.pas:321`) | Goods receipt / purchase | posted by `init12` → `M_Id = 32` |
| **13** | in | `برگشت` (radio `FK1322`, `:139`); `صدور سند برگشت از فروش` (`MakeSanadU.pas:456`) | Sales return | posted by `init13` → `M_Id = 35` |
| **14** | in | `خرید پسته` (radio `FK1214`, `:130`) | Pistachio purchase receipt | written by `FactorPesteh_U.pas:200` → `M_Id = 34`. **No branch in `B_SodoorClick`** |
| **15** | in | `تولید` (radio `FK1525`, `:158`) | Production receipt (finished goods in) | `Print_Anbar15.pas:175,191` |
| **16** | in | `جابجایی` (radio `FK261611`, `:122`) | Inter-warehouse transfer, receiving side | `SodoorSanadU.pas:282`, `Print_Anbar16.pas:159` |
| **21** | out | — | *(inferred: opening-stock reversal / period-out)* | referenced only as "no branch" in §5.3.4 |
| **22** | out | `حواله انبار` (radio `FK1322`, `:139`); `صدور سند فروش` (`MakeSanadU.pas:590`) | Goods issue / sales invoice | posted by `init22` → `M_Id = 33` |
| **25** | out | `تولید` (radio `FK1525`, `:158`) | Production issue (raw materials consumed) | `SodoorSanadU.pas:278`, `Print_Anbar15.pas:175` |
| **26** | out | `جابجایی` (radio `FK261611`, `:122`) | Inter-warehouse transfer, sending side | `SodoorSanadU.pas:282`, `Print_Anbar16.pas:181-190` |

**The numbering scheme is systematic**: the tens digit is the direction (`1x` inbound, `2x`
outbound) and the units digit is the reason (`1` period, `2` trade, `3` return, `4` pistachio,
`5` production, `6` transfer). `FM_InOut` duplicates the tens digit on every row
(`FactorPesteh_U.pas:200` writes `FM_InOut = 1` alongside `FM_ID = 14`). **Reconstruct this as
`(direction, reason)` in the rebuild rather than porting the composite integer.**

`AnbarReportU.dfm:385` restricts its type picker to `FK_ID in (11,12,13,14,16,22,26)` — i.e. the
activity report **cannot see production (15, 25) or type 21 at all**.

#### 3.2.1 Header — `FactorMaster`

Complete column list from `SodoorSanadU.dfm:546-655` and `Print_Anbar15.pas:21-43`
(both are `Select *`):

| Column | Type | Meaning |
|---|---|---|
| `FM_SSN` | identity | surrogate key — **the link used by `Moein.M_Link` for `M_Id ∈ {31,32,33,35}`** |
| `FM_COID` | int | fiscal year |
| `FM_Anbar` | int | warehouse id — **subsystem B has a real warehouse dimension; subsystem A does not** |
| `FM_Factor` | int | document number, per `(COID, Anbar)` — `isnull(Max(FM_Factor)+1, 1700001)` (`FactorPesteh_U.pas:194-195`) |
| `FM_Date` | varchar(10) | Jalali document date |
| `FM_InOut` | int | 1 in / 2 out (redundant with `FM_ID`) |
| `FM_ID` | int | document type, above |
| `FM_TCode` | varchar | counterparty account **code string** (e.g. `'301-1-2145'`) |
| `FM_TSSN` | int | counterparty `Sarfasl.S_SSN` |
| `FM_TName` | varchar(100) | denormalised counterparty name |
| `FM_Desc` | varchar(200) | narration |
| `FM_SanadNo` | int | voucher number, `0` when unposted |
| `FM_SanadDate` | varchar(10) | voucher date, `''` when unposted. **Written wrong by the pistachio path** (§8.3.4) |
| `FM_Lock` | smallint | `0` unconfirmed → `1` confirmed → `2` posted (§4.5) |
| `FM_Mab` | bigint | gross |
| `FM_Kasr` | bigint | discount |
| `FM_Maliat` | bigint | VAT |
| `FM_Total` | bigint | net |
| `FM_Count` | int | line count, denormalised |
| `FM_Link` | int | **the paired document's `FM_Factor`** — see §3.2.3 |
| `FM_UserID` | int | creating user |
| `FM_LUserID` | int | last-modifying user |
| `FM_LDate` | varchar(10) | last-modified date |

Note `FM_LUserID` / `FM_LDate` are the closest thing to an audit trail anywhere in the system, and
`arzi` never writes them — the pistachio insert (`FactorPesteh_U.pas:197-202`) omits both.

#### 3.2.2 Line — `FactorDetail`

From `Print_Anbar15.pas:46-60` and the pistachio insert (`FactorPesteh_U.pas:206-215`):

| Column | Type | Meaning |
|---|---|---|
| `FD_SSN` | identity | surrogate key |
| `FD_InOut` | int | direction, copied from the header |
| `FD_Anbar` | int | warehouse, copied from the header |
| `FD_FMSSN` | int | **link to the header by `FM_SSN`** — a real surrogate FK, unlike subsystem A |
| `FD_Code` | int | item code in `Anbar.Dbo.Cala` |
| `FD_CodeN` | varchar | denormalised `Cala.C_Name` |
| `FD_CodeP` | varchar | denormalised `Cala.C_Prop` |
| `FD_CodeV` | varchar | denormalised `Cala.C_Vahed` |
| `FD_Num` | numeric | quantity |
| `FD_Phi` | int | unit price |
| `FD_Mab` | bigint | gross |
| `FD_Kasr` | bigint | discount |
| `FD_Maliat` | bigint | VAT |
| `FD_Total` | bigint | net |
| `FD_VaznP` | numeric | **weight** — a second quantity dimension that subsystem A has no equivalent for |

`FD_VaznP` is what makes subsystem B usable for a weight-traded commodity: quantity and weight are
separate columns. On the pistachio path both are set to the same net kilogram figure
(§8.3.4). `AnbarReportU.pas:203-222` reports `Sum(FD_VaznP)` as a first-class total alongside
`Sum(FD_Num)`.

#### 3.2.3 Paired documents — transfer and production

Two of the ten types are **not single documents**. Both use `FM_Link` to point at the counterpart,
and — consistently with everything else in this codebase — the pointer is a **document number,
not a key**.

**Inter-warehouse transfer (`جابجایی`, 16 ↔ 26).** `Print_Anbar16.print_Jabejaei`
(`Print_Anbar16.pas:150-204`):

1. Load `FactorMaster` by `FM_SSN`. Branch on `FM_ID = 16` (receiving) vs anything else (sending)
   and stash `FM_Factor` / `FM_Anbar` / `FM_SSN` into `_F1/_A1/_SSN1` or `_F2/_A2/_SSN2`.
2. `_SSN := Q1.FieldValues['FM_Link']` — despite the variable name, this is used as a **factor
   number**: `Where FM_Factor = <_SSN> and FM_Coid = <_Coid>` (`:173`).
3. Load the counterpart, stash it into the other slot.
4. Show `FactorDetail Where FD_FMSSN = _SSN2` — **only the sending side's lines**.

So a transfer is two `FactorMaster` rows in two different warehouses, cross-referenced by number,
each with its own line set, and the print shows the out-side lines with both warehouse ids.

> **Fragility.** Step 2's lookup is scoped by `(FM_Factor, FM_Coid)` but **not by `FM_Anbar`**, and
> `FM_Factor` is only unique per `(COID, Anbar)` (§3.2.1 numbering). If two warehouses in the same
> year both have a document numbered *n*, the query returns two rows and the code silently takes
> the first. `Print_Anbar16.pas:175-179` checks only `RecordCount = 0`, never `> 1`.

**Production (`تولید`, 15 ↔ 25).** `Print_Anbar15.print_Tolid` (`Print_Anbar15.pas:170-195`):

1. Load by `FM_SSN` to read `FM_Coid` and `FM_Date`.
2. Re-query **all** `FM_ID = 15` documents for that `(COID, FM_Date)`, ordered by `FM_SSN`.
   The screen is a day's production run, not one document.
3. `DS1DataChange` (`:123-146`) then, for the selected row, resolves `FM_Link` as a factor number
   (`Where FM_Factor = <FM_Link>`, `:139`) to find the paired 25, and shows
   `FactorDetail Where FD_FMSSN in (<15's SSN>, <25's SSN>)` — **both sides' lines in one grid**.
   If the counterpart is not found it falls back to `SSN2 := SSN1` (`:141`), silently showing the
   input side twice.

`ID1 := 15; ID2 := 25` (`:175`) are assigned and **never read** — dead private fields.

So: **production is a two-document pair (materials issued 25, output received 15) and transfer is a
two-document pair (out 26, in 16), both matched by `FM_Link` = counterpart document number, and
both printed by grouping rather than by a real relationship.**

#### 3.2.4 Neither production nor transfer generates any accounting entry

`B_SodoorClick` (`SodoorSanadU.pas:190-204`) has branches for `FM_ID ∈ {11, 12, 13, 22}` only.
Everything else falls to:

```pascal
End else begin
  MessageDlg(' Not implemented yet. ', mterror, [mbok], 0 );
end;
```

— an **English** message in an otherwise entirely Persian UI, which is the author telling you this
is unfinished. So:

| `FM_ID` | Posts a voucher? | Via |
|---|---|---|
| 11, 12, 13, 22 | yes | `MakeSanadU.init11/12/13/22` → `M_Id` 31/32/35/33 |
| 14 | yes, but **only at creation time** | `FactorPesteh_U.pas:223-226` → `M_Id 34`; not reachable from `SodoorSanadU` (§8.4) |
| **15, 25** production | **no — ever** | `' Not implemented yet. '` |
| **16, 26** transfer | **no — ever** | `' Not implemented yet. '` |
| **21** | **no — ever** | `' Not implemented yet. '` |

Production and transfer therefore **move stock in the external warehouse system with no accounting
consequence whatsoever**. For a transfer that is arguably defensible (same entity, same total) —
though a proper system still moves value between warehouse accounts. For **production it is not**:
raw materials are consumed and finished goods appear, and the ledger never learns. Work-in-progress,
production variances and finished-goods valuation do not exist.

This is the single largest accounting gap in the domain and is restated in §10 and §14.

---

### 3.3 The pistachio purchase receipt (`FM_ID = 14`) as a document type

Fully specified in §8.3.4. Summary of its shape as a document:

| Aspect | Value |
|---|---|
| Header table | `Anbar.Dbo.FactorMaster`, `FM_ID = 14`, `FM_InOut = 1`, `FM_Anbar = 17` (hard-coded) |
| Numbering | `isnull(Max(FM_Factor)+1, 1700001)` per `(FM_Coid, FM_Anbar = 17)` |
| Lines | exactly one, always (`FM_Count = 1`) |
| Line quantity | `NR_Vazn` — **net kilograms**, also copied into `FD_VaznP` |
| Discount / VAT | always `0` |
| Created from | a `Rppc_Solution.Dbo.NewRamz` row in state `NR_State = 3` |
| Lifecycle | inserted directly at `FM_Lock = 2` — no draft, no confirm |
| Posting | inline, `M_Id = 34`, two lines, `M_Link = FM_Factor` |
| Reversal | **none exists** (§8.4 note 5) |

---

### 3.4 Cross-cutting: what a "document" is worth in this system

| Property | Subsystem A (`Anbar_Factor`) | Subsystem B (`FactorMaster`) |
|---|---|---|
| Types | 4, hard-coded in three places | 10, in a table `arzi` cannot edit |
| Warehouse dimension | **none** — `AJ_ID` on the item is decorative for stock purposes | `FM_Anbar` / `FD_Anbar`, real |
| Weight dimension | none | `FD_VaznP` |
| Explicit status | none — inferred from `Moein.M_Tx` | `FM_Lock` 0/1/2 |
| Header→line link | by number (`AFD_Factor`) | by key (`FD_FMSSN`) |
| Document→voucher link | `AF_Sanad` + `Moein.M_Link = AF_Factor` | `FM_SanadNo` + `Moein.M_Link = FM_SSN` (except `M_Id = 34`, which uses `FM_Factor`) |
| Paired documents | none | transfer 16↔26, production 15↔25, via `FM_Link` = counterpart number |
| Audit columns | `AFD_UserID` only | `FM_UserID`, `FM_LUserID`, `FM_LDate` — the last two never written |
| Created by `arzi` | yes, fully | only `FM_ID = 14` |
| Posted by `arzi` | yes, inline with save | yes, for `FM_ID ∈ {11,12,13,14,22}` only |

**Rebuild target.** One `inventory_documents` table with `document_type` (a real enum or FK),
`direction` derived from the type, `warehouse_id` mandatory, `status` explicit, lines linked by
surrogate FK, and `counterpart_document_id` as a nullable self-FK for paired documents. The ten
subsystem-B types plus subsystem A's four collapse to roughly:
`opening_stock`, `purchase_receipt`, `sales_issue`, `purchase_return`, `sales_return`,
`production_input`, `production_output`, `transfer_out`, `transfer_in`, `adjustment` (new, §15).
Full mapping in §16.


---

[← 3. Document types (part a)](05-03-a-document-types.md) | [index](00-index.md) | [4. The invoice (Factor) lifecycle (part a) →](05-04-a-invoice-factor-lifecycle.md)
