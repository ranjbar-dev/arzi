_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 6. Costing and valuation

### 6.0 The headline: there is no costing engine

Before the detail, four facts that between them define the whole subject:

1. **No cost is ever stored.** There is no `cost`, `avg_cost`, `standard_cost`, `last_cost` or
   `landed_cost` column on `Anbar_Jens`, on `Anbar_FactorD`, or on any snapshot table. The item
   master's only money column is `AJ_Phi`, and `AnbarCalaAddU.dfm:88` labels it
   `بهای فروش` — **"sale price"**, not cost.
2. **No cost-of-goods-sold is ever computed.** No query in the repository values an outbound
   movement at anything other than the price typed on the line. Grep for any expression pairing an
   issue line (`AFD_Type in (2,3)`) with an inbound average returns nothing.
3. **The one cost figure that exists is a display suggestion**, recomputed on demand, shown next to
   the price box on the line editor, and copied into the price only if the operator clicks it
   (`AnbarFactorAddU.pas:161-164`).
4. **The method, where it is named, is weighted average of purchases.** The application's own words
   are the caption of the menu item that materialises it:
   `AnbarFactorU.dfm:557` — `لیست مانده با متوسط قیمت تمام شده`,
   "list of balances at average cost price".

So the answer to "weighted average, FIFO, last purchase price or manual?" is: **manual**, with a
**weighted-average-of-purchases** advisory number offered at the point of entry. Two independent
implementations of that advisory number exist and they do not agree (§6.2, §6.3). A third, FIFO-
adjacent "last purchase price" implementation exists but is dead (§5.1.4).

---

### 6.1 What a line actually stores

The line editor `AnbarFactorAddU` is the only screen that creates a priced movement in subsystem A.
Its arithmetic is `TAnbarFactorAdd.PhiChange` (`AnbarFactorAddU.pas:166-171`) plus
`TAnbarFactorAdd.KasrDChange` (`:105-108`), quoted verbatim:

```pascal
procedure TAnbarFactorAdd.KasrDChange(Sender: TObject);
begin
    kasr.intvalue := Round(int(kol.IntValue * kasrd.FloatValue / 100 ));
end;

procedure TAnbarFactorAdd.PhiChange(Sender: TObject);
begin
    kol.IntValue := Round ( int ( Num.floatValue * Phi.IntValue ) );
    Maliat.IntValue := Round ( int ( (kol.intvalue - Kasr.intvalue) * DMaliat / 100 ));
    Total.IntValue := kol.IntValue + Maliat.IntValue -kasr.IntValue ;
end;
```

```
gross_amount (AFD_Kol)     = trunc( quantity × unit_price )
discount     (AFD_Kasr)    = trunc( gross_amount × discount_pct / 100 )      -- if entered as %
tax          (AFD_Maliat)  = trunc( (gross_amount − discount) × vat_rate / 100 )
line_total   (AFD_Total)   = gross_amount + tax − discount
```

`Round(int(x))` is `trunc` for non-negative `x` — Delphi's `Int()` truncates toward zero and the
result is already integral, so `Round` is a no-op. Every money figure on a line is therefore
**truncated, not rounded** — a systematic downward bias of up to 1 rial per line, three times per
line.

`AFD_Total` is **not** a parameter of the write path. `SP_AnbarAddToFactor`
(`Anbar_AddToFactor;1`, `AnbarFactorU.dfm:433-546`) takes `@COID, @Type, @Factor, @Date,
@Customer, @Code, @Name, @prop, @Vahed, @Num, @Phi, @Kol, @kasr, @Maliat, @user` — fifteen
parameters, no `@Total`. So the stored procedure recomputes `AFD_Total` server-side. Its body is
not in the repository; the formula above is the only definition available, and **the two could
have drifted** (open question §14).

Column meanings, taken from the grid titles (`AnbarFactorU.dfm:345-422`):

| Column | Persian title | English | Formula |
|---|---|---|---|
| `AFD_Code` | `کد کالا` | Item code | keyed / picked |
| `AFD_Name` | `نام کالا` | Item name | denormalised from `Anbar_Jens.AJ_Name` |
| `AFD_Prop` | `مشخصه` | Specification | denormalised from `AJ_Prop` |
| `AFD_Vahed` | `واحد` | Unit of measure | denormalised from `AJ_Vahed` |
| `AFD_Num` | `تعداد` | Quantity | keyed; `Numeric(14,3)`; always positive (§5.1.1) |
| `AFD_Phi` | `فی` | Unit price | keyed, or copied from `AJ_Phi`, or copied from the average |
| `AFD_Kol` | `مبلغ` | Gross amount | `trunc(Num × Phi)` |
| `AFD_Kasr` | `تخفیف` | Discount | keyed as an absolute amount **or** derived from a percentage |
| `AFD_Maliat` | `مالیات` | VAT | derived, see §7 |
| `AFD_Total` | `مبلغ کل` | Line total | `Kol + Maliat − Kasr`, recomputed in the SP |

**The unit price defaults to the item's sale price on every document type.**
`AnbarFactorAddU.pas:125` — `Phi.IntValue := DM.Anbar_Jens_Phi1.FieldByName('AJ_Phi').AsInteger`
— runs unconditionally in `CodeExit`, with no branch on `AF_Type`. On a **goods receipt**
(`AF_Type = 1`, a purchase) the price box is pre-filled with the *selling* price. The operator is
expected to overtype it, or to click the average-cost box. Nothing warns.

---

### 6.2 Implementation A — `Anbar_Jens_Phi1.phiin` (the line-entry advisory)

`Dmu.dfm:635-720`; already quoted in full at §5.1.3. The costing part is:

```sql
Declare @Noin  Real   Set @Noin  = (Select sum(AFD_num) from Anbar_FactorD
                                    where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F )
Declare @Mabin bigint Set @Mabin = (Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD
                                    Where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @Phiin int
Set @Phiin = 0
if @Noin >0  Set @phiin = Cast( @Mabin / @Noin as int )
```

```
average_cost(item) = trunc(  Σ (AFD_Num × AFD_Phi)  over AFD_Type = 1, AFD_Coid = year,
                                                       AFD_Factor ≠ current invoice
                           ÷ Σ  AFD_Num             over the same set  )
```

Behavioural properties, each of which is a decision the rebuild must make consciously:

| Property | Consequence |
|---|---|
| **Purchases only** (`AFD_Type = 1`) | Sales returns (`type 4`) put units back into stock (§5.1.1) but contribute **nothing** to the average. Purchase returns (`type 3`) remove units and are likewise ignored. The denominator of the average is therefore *larger* than the quantity on hand whenever returns exist, and the divisor and the stock figure describe different populations. |
| **Whole fiscal year, no date cut-off** | This is not a moving average. Buying at a higher price in Esfand retroactively raises the "cost" of an issue made in Farvardin — because the number is recomputed from scratch every time it is displayed. There is no point-in-time cost. |
| **Scoped by `AFD_Coid`** | The average resets to zero at the start of each fiscal year. There is no opening cost, no carry-forward, no `Anbar_Jens` cost column to seed it. §6.5. |
| **Excludes the current invoice** | Correct for editing; degenerate but harmless on a brand-new invoice (`AF_Factor.Text` is empty → `StrToIntDef(…,0)` → `0` → matches nothing, `AnbarFactorAddU.pas:117`). |
| **Uses `AFD_Num × AFD_Phi`, recomputed** | Not the stored `AFD_Kol`. Because `AFD_Kol` was truncated at write time (§6.1), the two differ whenever `Num` is fractional. |
| **`Cast(… as int)` truncates** | The average cost is a whole number of rial, rounded down. |
| **No warehouse dimension** | `AJ_ID` never appears. One global average per item code across all warehouses. |
| **NULL propagation** | If the item has no type-1 lines, `@Noin` and `@Mabin` are `NULL`; `if @Noin > 0` evaluates to UNKNOWN and `@phiin` stays `0`. Safe. But the same NULLs propagate into the derived column `Remi = (@Noin - @NoOut - @NoBin + @NoBOut)` at `Dmu.dfm:714`, which is `NULL` unless the item has movements of **all four** types. `Remi` happens to be **dead** — no unit reads it (the client recomputes on-hand from the four component columns at `AnbarFactorAddU.pas:129-130`, where `.AsInteger` maps `NULL` → `0`) — so this is latent, not live. Do not carry `Remi` forward. |

**Presentation.** `AnbarFactorAddU.pas:127-128` puts the figure in `Phi1`, a read-only box labelled
`AnbarFactorAddU.dfm` `sLabel9`; clicking it (`Phi1Click`, `:161-164`) does exactly
`phi.intvalue := phi1.intvalue`. There is no keyboard route to the same action and no visual
affordance that the box is clickable — an undiscoverable but load-bearing interaction that the
React rebuild must replace with a real button.

#### 6.2.1 Worked arithmetic

Item `1001`, unit `کیلوگرم` (kg), fiscal year `1403`. Purchases (type 1) recorded so far:

| Invoice | `AFD_Num` | `AFD_Phi` | `AFD_Kol` stored | `AFD_Kasr` | `AFD_Maliat` | `AFD_Total` |
|---|---|---|---|---|---|---|
| 5 | 100.000 | 50 000 | 5 000 000 | 0 | 0 | 5 000 000 |
| 9 | 60.000 | 65 000 | 3 900 000 | 0 | 0 | 3 900 000 |
| 14 | 40.000 | 72 500 | 2 900 000 | 145 000 (5 %) | 247 950 (9 %) | 3 002 950 |

```
Σ (AFD_Num × AFD_Phi) = 100×50 000 + 60×65 000 + 40×72 500
                      = 5 000 000 + 3 900 000 + 2 900 000 = 11 800 000
Σ  AFD_Num            = 100 + 60 + 40                     =        200
phiin                 = trunc( 11 800 000 / 200 )         =     59 000 rial/kg
```

**Note what is absent.** Invoice 14 carried a 145 000 discount and 247 950 of VAT; its true landed
cost per unit is `(2 900 000 − 145 000) / 40 = 68 875` before tax, or `3 002 950 / 40 = 75 073.75`
including it. Neither figure enters the average. The average uses the **gross list extension only**:
discounts do not reduce cost, and tax does not increase it.

**Truncation divergence.** Take instead a fractional line: `Num = 0.5`, `Phi = 999`.
`AFD_Kol` stored = `trunc(0.5 × 999)` = `499`. `Anbar_Jens_Phi1` recomputes `0.5 × 999 = 499.5`.
The two implementations of the average therefore differ on the same data — see §6.3.

---

### 6.3 Implementation B — `Anbar_Mandeh` report `Phiin1` / `Phiin2` / `PhiOut1` / `PhiOut2`

`Anbar_MandehU.dfm:1613-1727`, quoted at §5.1.2. The valuation lines are:

```sql
Update #R Set Mabin1 = isnull((Select Sum(AFD_Kol) From Anbar_FactorD
        Where AFD_Code=#R.AJ_Code and AFD_Coid=@Coid
          and AFD_Date>=@FDate and AFD_Date<=@TDate and Afd_Type=1),0)
Update #R Set Phiin1 = Cast( Mabin1 / Tedin1 as int ) Where Tedin1 > 0
```

and three structurally identical blocks producing `Phiin2` (type 4, sales returns),
`PhiOut1` (type 2, sales) and `PhiOut2` (type 3, purchase returns).

```
average_price(item, type, d1..d2) = trunc( Σ AFD_Kol  ÷ Σ AFD_Num )   over that type, in window
```

Differences from Implementation A — **all four are real behavioural divergences**:

| | `Anbar_Jens_Phi1.phiin` | `Anbar_Mandeh.Phiin1` |
|---|---|---|
| Amount source | `AFD_Num × AFD_Phi`, recomputed live | `AFD_Kol`, the stored (truncated) column |
| Date window | none — whole fiscal year | `@FDate … @TDate`, a lexicographic string range |
| Current invoice | excluded | included |
| Types produced | type 1 only | all four, separately |
| Zero guard | `if @Noin > 0` (T-SQL `IF`) | `Where Tedin1 > 0` (set-based) |

The last row matters: `Phiin1` is only *assigned* where `Tedin1 > 0`, so it keeps its
initialised `0` otherwise — same outcome, different mechanism, and the temp column is
`cast(0 as bigint)` (`Anbar_MandehU.dfm:116`) so the divide never happens on an empty set.

`PhiOut1` — the average **selling** price of the period — is the closest thing to a margin figure
the system produces. It is not compared with `Phiin1` anywhere; the report prints both columns and
leaves the arithmetic to the reader.

---


---

[← 5. Stock quantity mathematics (part b)](05-05-b-stock-quantity-mathematics.md) | [index](00-index.md) | [6. Costing and valuation (part b) →](05-06-b-costing-and-valuation.md)
