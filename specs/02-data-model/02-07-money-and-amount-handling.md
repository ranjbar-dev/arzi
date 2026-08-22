_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 7. Money and amount handling

### 7.1 Summary

| Question | Answer | Evidence |
|---|---|---|
| Storage type | **`bigint`** (SQL Server), surfaced as `TLargeintField` / Delphi `Int64` | 125 `TLargeintField` declarations across the `.dfm` files; `Dmu.dfm:951` (`DCheckS_Mab`), `Dmu.dfm:1091` (`Anbar_Tasfieh.Mab`) |
| Scale | **0 — whole units, no minor unit, no decimals** | every money field is an integer field; there is not a single `TCurrencyField` or scaled `TBCDField` on a money column |
| Unit | **Iranian rial (ریال)**, never toman | every printed total is suffixed with the literal `ریال`; repo-wide grep for `تومان` (toman) returns **zero** hits |
| Multi-currency | **None. Vestigial.** | see §7.6 |
| Rounding | **Truncation toward zero** on the invoice path, **banker's rounding** on the pistachio path — two different rules | §7.4 |

### 7.2 The money columns

All of these are `bigint`. Grouped by domain (full per-table detail in §2):

| Legacy column | Table | Meaning | Proposed name |
|---|---|---|---|
| `M_Bed` / `M_Bes` | `Moein` (voucher lines) | debit / credit amount, **both stored positive**, one of the two is 0 | `debit_amount` / `credit_amount` |
| `DM_TBed` / `DM_TBes` | `DMoein` (voucher headers) | denormalised voucher debit / credit totals | `total_debit` / `total_credit` |
| `S_Mab` | `DCheck`, `DFish`, `TCheck` | cheque / deposit-slip amount | `amount` |
| `TM_Mab` | petty-cash header | petty-cash document amount | `amount` |
| `CD_Mab` | cheque detail | cheque line amount | `amount` |
| `AF_Mab`, `AF_Mab1`…`AF_Mab5`, `AF_Kasr`, `AF_Maliat`, `AF_Total` | `Anbar_Factor` (invoice header) | gross, five extra charge/adjustment buckets, deduction, VAT, grand total | `gross_amount`, `extra_amount_1..5`, `deduction_amount`, `tax_amount`, `total_amount` |
| `FD_Mab`, `FD_Kasr`, `FD_Maliat`, `FD_Total` | `Anbar_FactorD` / external `FactorDetail` | line gross, deduction, VAT, total | same, per line |
| `FM_Mab`, `FM_Kasr`, `FM_Maliat`, `FM_Total` | external `Anbar.dbo.FactorMaster` | header-level equivalents in the *external* pistachio system | (not part of the core schema — §1.5) |
| `AJ_Phi`, `Phi`, `PhiIn1/2`, `PhiOut1/2` | `Anbar_Jens`, reports | unit price (`فی`) | `unit_price` |
| `NR_Kol` | `Rppc_Solution.dbo.NewRamz` | purchase-receipt total | external |

`Bed` / `Bes` / `TBed` / `TBes` / `BedR` / `BesR` / `RBed` / `RBes` / `GBed` / `GBes` /
`RemBed` / `RemBes` / `Rem1` / `Rem2` / `Total` / `Kol` / `Maliat` / `Kasr` / `payF` / `payC`
are **query-result aliases**, not base columns; they are all `TLargeintField` too, i.e. every
aggregate stays in `bigint` and no `SUM()` is ever cast to a float.

**Corollary: there is no floating point anywhere on the money path.** This is the single best
property of the legacy design and the rebuild must preserve it (§7.7).

### 7.3 Quantities and weights are *not* integers

Distinguish sharply — these are the only decimal columns in the system:

| Legacy column | Delphi field | SQL Server type | Where |
|---|---|---|---|
| `M_Ted` (voucher-line quantity, `تعداد`) | `TBCDField` `Precision = 18, Size = 3` | `decimal(18,3)` | `DKolU.dfm:567-571`, `DMoein.dfm:778-782`, `DaftarT_U.dfm:668-672` |
| `R1`, `R2`, `TedIn1`, `TedIn2`, `TedOut1`, `TedOut2` (stock balances / in-out quantities) | `TBCDField` `Precision = 14, Size = 3` | `decimal(14,3)` | `Anbar_MandehU.dfm:1753-1814` |
| `NR_P2V`, `NR_P2VV`, `NR_Vazn2` (pistachio weights) | `TBCDField` `Precision = 18, Size = 2` | `decimal(18,2)` | `FactorPesteh_U.dfm:423-431, 482-484` |
| `FD_Num`, `FD_Vaznp` (external invoice qty / weight) | `TFMTBCDField` `Precision = 38, Size = 3` | `decimal(38,3)` (or `numeric`) | `AnbarReportU.dfm:469-473, 495-499` |

In `TBCDField`/`TFMTBCDField` the `Size` property **is the scale**, and `Precision` the total
digits. So: money = `bigint` (scale 0); quantity/weight = `decimal(_,2)` or `decimal(_,3)`.

### 7.4 Rounding rules — two different ones, both undocumented

The whole of the amount arithmetic performed on the client is four lines in
`AnbarFactorAddU.pas` plus one in `PestehD_U.pas`.

**Invoice line — truncation toward zero** (`AnbarFactorAddU.pas:107, 168-170`):

```pascal
kasr.intvalue   := Round(int(kol.IntValue * kasrd.FloatValue / 100 ));      // :107
kol.IntValue    := Round(int( Num.floatValue * Phi.IntValue ));            // :168
Maliat.IntValue := Round(int((kol.intvalue - Kasr.intvalue) * DMaliat / 100 )); // :169
Total.IntValue  := kol.IntValue + Maliat.IntValue - kasr.IntValue;         // :170
```

`Int(x)` in Delphi discards the fractional part **toward zero** (`Int(-2.7) = -2.0`); the
surrounding `Round` then operates on an already-integral value and is a no-op. So:

- `gross = trunc(quantity × unit_price)`
- `deduction = trunc(gross × deduction_pct / 100)`
- `tax = trunc((gross − deduction) × tax_pct / 100)`
- `total = gross + tax − deduction` — computed from the *already truncated* components, so the
  truncation error compounds across the three terms and is never redistributed.

`DMaliat` (the VAT percentage, `درصد مالیات`) comes from `AnbarConfig.AC_DMaliat` read `AsFloat`
(`AnbarFactorAddU.pas:145`) and is forced to `0` when the item is flagged non-taxable
(`AnbarJens.AJ_Maliat = 0`, `AnbarFactorAddU.pas:146`). `AJ_Maliat` is a 0/1 flag
(`AnbarCalaAddU.pas:147-149`).

**Pistachio settlement — banker's rounding** (`PestehD_U.pas:129`):

```pascal
Kol.IntValue := Round(NabV.FloatValue * Phi.IntValue );
```

No `Int()`. Delphi's `Round` is **round-half-to-even** (IEEE 754 "banker's rounding"), so
`Round(2.5) = 2` and `Round(3.5) = 4`. This is a *different rule* from the invoice path for
structurally identical arithmetic (quantity × unit price).

**This inconsistency is a genuine business-logic discrepancy, not a style difference.** For the
same weight and price, the pistachio screen and the invoice screen can produce amounts differing
by 1 rial. The rebuild must ask the business owner which one is correct rather than silently
unifying them (§12, §13).

There is **no rounding anywhere else**: repo-wide grep for `Round(`/`Trunc(`/`Floor(` in `.pas`
returns only `AnbarFactorAddU.pas:107,168,169`, `PestehD_U.pas:129`, and `Utility.pas:417` (a date
function, §6.3.1). No SQL statement in the codebase contains `ROUND(`.

### 7.5 Presentation of amounts

**Thousands separators** — `TDM.inttoStr3` (`Dmu.pas:859-867`):

```pascal
function TDM.inttoStr3(N: int64; DefaluZero:String='0' ): String;
begin
   S:= inttostr( N );
   if Length( S ) > 3 Then S := Copy( S , 1 , Length(S)-3)  + ',' + Copy( S , Length(S)-2,  3 );
   if Length( S ) > 7 Then S := Copy( S , 1 , Length(S)-7)  + ',' + Copy( S , Length(S)-6,  7 );
   if Length( S ) >11 Then S := Copy( S , 1 , Length(S)-11) + ',' + Copy( S , Length(S)-10, 11 );
   if N=0 then Result:= DefaluZero Else Result:= S;
end;
```

Three hard-coded comma insertions, so grouping stops after 12 significant digits; a value above
`999,999,999,999` (10¹²) renders with the leading digits unseparated. Zero renders as the
`DefaluZero` parameter (default `'0'`, callers frequently pass `''` to blank it). A **negative**
value is not special-cased: the minus sign counts toward `Length(S)`, shifting every comma one
place — `-1234567` renders as `-1,234,567`? No: `Length('-1234567') = 8 > 7`, so a comma is
inserted at both positions computed from the sign-inclusive length, yielding a misplaced group.
Verify against live output before replicating; the rebuild should simply use a correct formatter.

**Amounts in words (Persian)** — `TDM.N23` + `TDM.Str2String` (`Dmu.pas:568-635`), duplicated
verbatim as `TUtil.N23` / `TUtil.Str2String` / `TUtil.No2String` (`Utility.pas:446-522`).
Chunks the decimal string into groups of three and appends the scale words:

| Group | Persian | Value |
|---|---|---|
| 1 | (none) | 10⁰ |
| 2 | هزار (`hezar`) | 10³ |
| 3 | ميليون (`milyun`) | 10⁶ |
| 4 | ميليارد (`milyard`) | 10⁹ |
| 5 | تريليارد (`trilyard`) | 10¹² |

Only **five** groups are handled (`Dmu.pas:606-633`), so any amount ≥ 10¹⁵ silently loses its
leading digits — well inside `bigint` range (max ≈ 9.2 × 10¹⁸). The scale word for 10¹² is
تريليارد, which conventionally denotes 10¹⁵, not 10¹² — a mislabelling carried into every printed
document.

Call sites always append the currency literally:

```
CheckEditU.pas:335,345      'جمع : '+Dm.Str2String(CM_Mab.Inttext)+' ریال'      "Total: <words> rial"
TankhahEdit.pas:311,323     'جمع : '+Dm.Str2String(TM_Mab.Inttext)+' ریال'
PrintNu.pas:86,134          'جمع: '+Dm.Str2String( S )+ ' ریال'
PrintMU.pas:93              ' جمع : '+ S + ' ریال '
RooznamehViewU.pas:181      'جمع : '+ S+ ' ریال'
Factorprint2U.pas:97        Util.No2String( Q4.FieldByName('AF_Total').AsInteger) + ' ریال '
```

Note `Factorprint2U.pas:97` reads a `bigint` invoice total through `.AsInteger` (32-bit) — it
**overflows or silently truncates above 2,147,483,647 rial (≈ 2.1 billion rial, ~7 000 USD)**.
That is an easily reachable invoice total. Record it as a legacy defect to fix, not to port.

**Minimum amount.** The only amount floor found: `Asnad_Daryaft_NewU.pas:56` rejects a cheque
below 1 rial with `'مبلغ چک حداقل 1 ریال مجاز است'` — *"the minimum permitted cheque amount is
1 rial"*. Confirms the unit is rial and the minor unit is 1, i.e. amounts are integral rial.

**Dead string-decimal arithmetic.** `TDM.Add_String` / `TDM.Adj_Cent` (`Dmu.pas:637-698`) implement
signed 30-digit decimal addition on strings with **two implied decimal places** (`Adj_Cent` pads
to `.00`). This is the fossil of an earlier design in which amounts had cents. It has **zero call
sites** (repo-wide grep: only the declarations at `Dmu.pas:116,119` and the bodies). Do not port
it, and do not take it as evidence that amounts have decimals — the live columns do not.

### 7.6 Multi-currency: the name is vestigial

The application's main-window caption is `حسابداري ارزي` — literally **"foreign-currency
accounting"** (`Mainu.dfm:6`, escape sequence `#1581#1587#1575#1576#1583#1575#1585#1610' '#1575#1585#1586#1610`).
That is where the product name *arzi* comes from.

**No foreign-currency machinery exists anywhere in the codebase.** Exhaustively:

- No column named or prefixed `Arz`, `Nerkh` (rate), `Currency`, `Dollar`, `Rate`, `Exchange`.
  Repo-wide grep across all `.pas` returns one hit, and it is a commented-out list of Delphi
  `TFieldType` enum members (`Backup_U.pas:75`).
- No currency-code column on any table, no rate table, no rate field on any document.
- No second amount column paired with a rate on `Moein`, `DCheck`, `Anbar_Factor` or anywhere else.
  The `AF_Mab1..AF_Mab5` buckets on the invoice header are *charge* buckets, not currency columns —
  they are all `bigint` with no accompanying rate or code.
- Every printed total is hard-coded to `ریال`.

**Conclusion: `arzi` is a brand/legacy name only.** The system is single-currency IRR. Do not build
a currency dimension into the rebuild. If the business genuinely needs FX, that is a new feature
and belongs in §13, not in a port.

### 7.7 Proposed PostgreSQL model

| Concern | Proposal | Rationale |
|---|---|---|
| Money column type | **`bigint`**, unit = **whole rial**, `NOT NULL DEFAULT 0` | exact match for the legacy type and semantics; zero migration risk; no float, ever |
| Alternative considered | `numeric(20,0)` | rejected — `bigint` is what the source is, is faster, and the minor unit is genuinely 1 rial (`Asnad_Daryaft_NewU.pas:56`) |
| Explicitly rejected | `money`, `double precision`, `real`, `float8` | PostgreSQL `money` is locale-dependent and lossy; floats are unusable for ledgers |
| Rust representation | `i64` (`Decimal` **not** needed for money) | preserves exactness end-to-end; serialise as a JSON **number** only if the frontend is guaranteed to stay below 2⁵³, otherwise as a **string** — safest is string |
| Quantity / weight | `numeric(18,3)` (voucher `M_Ted`, stock), `numeric(18,2)` (pistachio weights) — Rust `rust_decimal::Decimal` | mirrors the `TBCDField` precision/scale found in §7.3 |
| Unit price | `bigint` (rial per unit) — matches `AJ_Phi` | but see §12: confirm no site relies on a fractional unit price |
| Debit/credit | keep **two non-negative columns** `debit_amount bigint NOT NULL DEFAULT 0` and `credit_amount bigint NOT NULL DEFAULT 0` with `CHECK (debit_amount >= 0 AND credit_amount >= 0 AND (debit_amount = 0 OR credit_amount = 0))` | preserves the legacy convention exactly; a signed single-column model would change every report |
| Rounding | implement **both** legacy rules as named, tested functions — `trunc_toward_zero` for the invoice path and `round_half_even` for the pistachio path — and pick per call site to match `AnbarFactorAddU.pas` / `PestehD_U.pas` | port-as-is; unifying them is a **behaviour change** and must be approved (§13) |
| Rounding implementation | do the multiplication in `Decimal` (exact), then apply the rule, then store `i64` | avoids the legacy `Extended`-float intermediate in `AnbarFactorAddU.pas:168` |
| Percentages | `numeric(5,2)` for `AC_DMaliat` (VAT %) and the per-line deduction %, **not** float | `AnbarFactorAddU.pas:145` reads it `AsFloat` today; storing it exact removes a class of off-by-one-rial bugs |
| Currency | **no currency column.** Single currency, IRR, whole rial. Document the assumption in one place so it is cheap to revisit. | §7.6 |
| Formatting | done in the frontend from the raw integer via `Intl.NumberFormat('fa-IR')`; the backend never returns pre-formatted strings | replaces `inttoStr3` (`Dmu.pas:859`) and its comma bugs |
| Amount-in-words | a Rust/TS helper with **full** scale coverage up to `i64::MAX` and the correct Persian scale words | replaces `Str2String` (`Dmu.pas:604`) and fixes both the 10¹⁵ cut-off and the تريليارد mislabelling |

Migration checks to run against live data before cutover:

1. `SELECT ... WHERE debit <> 0 AND credit <> 0` — any voucher line with both sides populated
   violates the assumed convention and needs a business decision.
2. `SELECT MAX(AF_Total) FROM Anbar_Factor` — if any value exceeds 2 147 483 647, confirm whether
   the printed word-form was ever wrong (the `.AsInteger` bug at `Factorprint2U.pas:97`).
3. Recompute `AF_Total` from `AF_Mab + AF_Maliat − AF_Kasr` for every invoice and list mismatches;
   the truncation compounding in `AnbarFactorAddU.pas:107,168-170` means stored totals may not
   reproduce from stored components.
4. Recompute `DM_TBed` / `DM_TBes` from `SUM(M_Bed)` / `SUM(M_Bes)` per voucher and list mismatches
   — these are denormalised and maintained by the application, not by a constraint.


---

[← 02-06-b-date-handling-arithmetic-and-model.md](02-06-b-date-handling-arithmetic-and-model.md) | [02-08-a-configuration-ini-and-tanzim.md →](02-08-a-configuration-ini-and-tanzim.md)
