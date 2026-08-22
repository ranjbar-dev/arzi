_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 9. Settlement (Tasfieh)

### 9.0 What "settlement" is here

`تسویه فاکتور` — "invoice settlement". It **attaches treasury instruments to an invoice**:
deposit slips (`DFish`, `فیش واریزی`) and cheques (`DCheck`, `چک`). That is all it does.

**There is no settlement algorithm.** No allocation of a payment across invoices, no FIFO matching,
no running balance, no partial/full status, no over-payment guard, no ageing, no write-off. The
screen is a filtered list of treasury rows plus create / edit / delete buttons. The "settlement"
is the existence of the link column, nothing more.

Cross-reference `docs/06-treasury.md` for `DFish` / `DCheck` themselves; this section covers only
the inventory side.

---

### 9.1 The screen — `TasfiehFactor`

Form caption `تسویه حساب فاکتور` ("invoice account settlement", `TasfiehFactor.dfm:5`).
Two public entry points, one per subsystem.

| Entry point | Caller | Argument | `_Prg` | Source table |
|---|---|---|---|---|
| `Init_Factor` (`TasfiehFactor.pas:120-156`) | `AnbarListU.pas:482`, button `تسویه فاکتور` | `AF_SSN` | `1` | `Anbar_Factor` |
| `Init_Pesteh` (`:158-193`) | `SodoorSanadU.pas:154` | `FM_SSN` | `2` | `Anbar.Dbo.FactorMaster` |

Both load the document header into the same read-only panel:

| Control | Persian label | English | `Init_Factor` source | `Init_Pesteh` source |
|---|---|---|---|---|
| `F_Code` | `کد بدهکار` | Debtor account code | `Taraf.Get_FullCode(AF_Customer)` | `FM_TCode` |
| `F_Name` | — | Counterparty name | `AF_CustomerN` | `FM_TName` |
| `F_Date` | `تاریخ فاکتور` | Invoice date | `AF_Date` | `FM_Date` |
| `F_No` | `شماره فاکتور` | Invoice number | `AF_Factor` | `FM_Factor` |
| `F_Sanad` | `شماره سند` | Voucher number | `AF_Sanad` | `FM_SanadNo` |
| `F_Desc` | `توضیح فاکتور` | Invoice description | `AF_Desc` | `FM_Desc` |
| `F_Mab` | `مبلغ فاکتور` | Invoice amount | **`AF_Total`** (net) | **`FM_Total`** (net) |

Both then open the same parameterised query with `@SSN := F_No.IntValue`.

> **Definitive proof that the treasury link is by document *number*.** `Init_Factor` receives the
> surrogate key `AF_SSN` (`AnbarListU.pas:481` passes `Q1.FieldByName('AF_SSN')`), uses it **only**
> to fetch the header (`TasfiehFactor.pas:126`), and then binds the query parameter from
> `F_No.IntValue` — which was set from `AF_FACTOR` at `:135`:
> ```pascal
> F_No.Text := QS.FieldByName('AF_FACTOR').AsString;      // :135
> …
> Q1.Parameters.ParamByName('SSN').Value := F_No.IntValue; // :150
> ```
> The parameter is called `SSN`, the local variable is called `SSN`, and the value is the
> **invoice number**. Identically in `Init_Pesteh` (`:173` then `:187`, `FM_FACTOR`).
> This closes the question raised by the data-layer agent: **`DCheck.S_LinkSSN` and
> `DFish.S_LinkSSN` hold the business document number, not `AF_SSN`/`FM_SSN`.** Corroborated
> independently at `AnbarListU.pas:356, 368, 450-454, 538-539` and `SodoorSanadU.dfm:526-528`.

---

### 9.2 The query

`TasfiehFactor.dfm:427-446`, verbatim:

```sql
Declare @SSN int Set @SSN=:SSN
Declare @Prg int Set @PRg=:PRG
Declare @Coid int Set @Coid = :CoiD

Select  1 AS Type, S_SSN, S_State, S_StateName, S_FishNo, S_CheckNo= Space(50),
        S_Date, S_Sanad, S_Mab, S_BesCR, S_Desc, '' AS S_DateS
   From DFish
   Where S_LinkPrg=@PRG and S_LinkSSN=@SSN and S_Coid=@COID

   union

Select 2 As Type, S_SSN, S_State, S_StateName, S_FishNo=Space(50), S_CheckNo,
       S_Date, S_Sanad, S_Mab, S_BesCR, S_Desc, S_DateS
From DCheck
   Where S_LinkPrg=@PRG and S_LinkSSN=@SSN and S_Coid=@COID
```

**The composite link key is `(S_LinkPRG, S_LinkSSN, S_COID)`:**

| `S_LinkPRG` | Meaning | Set by |
|---|---|---|
| `1` | subsystem A invoice — `S_LinkSSN = AF_Factor` | `TasfiehFactor.pas:143` |
| `2` | subsystem B / pistachio document — `S_LinkSSN = FM_Factor` | `TasfiehFactor.pas:180` |

See `docs/06-treasury.md` for the full `S_LinkPRG` enumeration.

Notes on the query itself:

- **`UNION`, not `UNION ALL`.** Duplicate rows across the two branches are impossible (`Type`
  differs), so this only costs a needless sort — but it also means the result is **ordered by the
  union's implicit distinct sort**, i.e. by `Type` then `S_SSN`, not by date. There is no
  `ORDER BY`. Deposit slips always sort before cheques.
- `S_CheckNo = Space(50)` / `S_FishNo = Space(50)` pad the non-applicable column with 50 spaces
  rather than `NULL`, so the grid shows blank-looking but non-empty values.
- The design-time `ConnectionString` again names a developer machine (`MOHSEN-RANJBAR\SQLEXPRESS`,
  `Initial Catalog=Arzi89`) and is overwritten at runtime (`:124, 147`).
- `Prepared = True` on a query whose SQL never changes — harmless.

---

### 9.3 Grid and totals

`TasfiehFactor.dfm:300-393`:

| Column | Persian title | English |
|---|---|---|
| `S_StateName` | `نوع دریافت` | Receipt type / status (denormalised label) |
| `S_FishNo` | `شماره فیش` | Deposit-slip number |
| `S_CheckNo` | `شماره چک` | Cheque number |
| `S_Date` | `تاریخ` | Date |
| `S_DateS` | `سررسید` | Due date (cheques only) |
| `S_Sanad` | `شماره سند` | Voucher number |
| `S_Mab` | `مبلغ` | Amount |
| `S_BesCR` | `کد بستانکار` | Credit account code |
| `S_Desc` | `توضیحات` | Description |

The only aggregate in the whole feature is the grid footer
(`FooterRow.FieldFooterDefs.Strings = ('S_Mab=%Sum')`, `TasfiehFactor.dfm:305-306`) — a
**client-side sum computed by the grid component**, refreshed by
`G1.RecalculateSummaryResults(True)` after every operation (`:153, 190, 206, 215, 251, 288`).

> **Nothing compares that sum with `F_Mab`.** The invoice total and the settled total sit in two
> boxes on the same form and are never subtracted. There is no "outstanding" figure, no colour
> change, no warning on over-settlement. An invoice for 10 000 000 can carry 50 000 000 of
> cheques and the screen reports it without comment.

The list screen exposes the same figures as two computed columns —
`payF = Sum(DFish.S_Mab)` and `payC = Sum(DCheck.S_Mab)` (`AnbarListU.pas:538-539`) — again with
no comparison against `AF_Total`. **Deriving "is this invoice paid?" is left entirely to the
reader.**

---


---

[← 8. The Pesteh (pistachio) specialisation (part c)](05-08-c-pesteh-pistachio-specialisation.md) | [index](00-index.md) | [9. Settlement (Tasfieh) (part b) →](05-09-b-settlement-tasfieh.md)
