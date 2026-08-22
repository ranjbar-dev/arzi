_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 6.4 Implementation C — `Anbar_Mandeh` (the stored procedure) `AVin`

Distinct from the report of §6.3 despite the shared name. `Anbar_Mandeh;1`
(`Dmu.dfm:721-743`) is a stored procedure taking a **single** parameter `@Coid`. Its body is not
in the repository. Its result set is used in exactly one place —
`TAnbarFactor.OP1Click` (`AnbarFactorU.pas:204-239`), the menu item captioned
`لیست مانده با متوسط قیمت تمام شده` ("list of balances at average cost price",
`AnbarFactorU.dfm:557`) — and there it exposes:

| Field | Read at | Inferred meaning |
|---|---|---|
| `AFD_Code` | `:221` | item code |
| `AFD_Name` | `:222` | item name |
| `AFD_Vahed` | `:224` | unit of measure |
| `Remi` | `:216` | remaining quantity |
| `AVin` | `:217` | **average inbound price** — the cost |

`AFD_Prop` is fetched too but the assignment is **commented out** (`:223`), so the generated lines
carry no specification text.

The handler builds an entire invoice from stock on hand:

```pascal
for i:=1 to dm.Anbar_Mandeh.recordcount do
begin
  rem  :=  Dm.Anbar_Mandeh.fieldByName('Remi').asinteger ;
  AVin :=  Dm.Anbar_Mandeh.fieldByName('AVin').asinteger ;
  if rem > 0 then
  begin
     CDS1.Append;
     …
     CDS1.FieldByName('Num').Asinteger := Rem;
     CDS1.FieldByName('phi').AsInteger := Avin;
     CDS1.FieldByName('kol').asstring := inttostr(Rem * AVin);
     CDS1.FieldByName('kasr').AsInteger := 0;
     CDS1.FieldByName('maliat').AsInteger := 0;
     CDS1.FieldByName('total').Asstring := inttostr(Rem * Avin);
     CDS1.Post;
  end;
  Dm.anbar_mandeh.next;
end;
```

This is the **opening-stock document generator**. It is how a fiscal year is seeded: run it inside
a new invoice, get one line per item at quantity-on-hand valued at average inbound cost, with zero
discount and zero tax, then save.

Defects in it:

- **`Rem` and `AVin` are `int64` but read with `.AsInteger`** (`:206,216,217`), truncating the
  `Numeric(14,3)` quantity to whole units. An item with 12.75 kg on hand carries forward as 12 kg.
  **0.75 kg is destroyed in the carry-forward.** This is the same `.AsInteger` fault as
  §5.2.2 point 3 but with permanent effect: the truncation is written to the database.
- **`if rem > 0`** silently drops every item with zero or *negative* on-hand. Since negative stock
  is permitted (§5.2, §6.6), a year with negative balances carries forward as if they were zero —
  the negative simply disappears at the year boundary.
- **`Dm.Anbar_Mandeh.first` is never called** before the loop (`:214`). It works only because
  `.Open` positions on the first record; any prior navigation on the shared `DM` dataset would
  make the loop start mid-set and run off the end. `for i := 1 to RecordCount` with a manual
  `.next` is the pattern used throughout this codebase and is fragile everywhere.
- **The result carries no tax even for taxable items** — `kasr` and `maliat` are hard-zeroed
  (`:228-229`), which is right for an opening balance and wrong if an operator uses the menu item
  to build an ordinary sales invoice, which nothing prevents.
- **`AVin` is truncated to whole rial** on both sides (server `int` and client `.AsInteger`), so
  `kol = Rem × AVin` will not reconcile to the prior year's stock value.

**Rebuild note:** period-opening is a first-class operation, not a menu item on the invoice screen
that pre-fills a grid. It needs its own document type (§3) and it must preserve fractional
quantities.

---

### 6.5 There is no cost carry-forward, and that is a structural gap

Every costing query filters on `AFD_Coid` (`Dmu.dfm:635-720` three times; `Anbar_MandehU.dfm`
throughout). On the first day of a new fiscal year:

- `Σ AFD_Num[type 1]` = 0, so `phiin` = 0 (`if @Noin > 0` fails).
- `Anbar_Mandeh`'s opening balance `R1` is `Σ … Where AFD_Date < @FDate and AFD_Coid = @Coid` —
  and *within* the new `@Coid` there are no earlier-dated rows, so `R1` = 0 too.

So both stock **and** cost start each year at zero, and the only bridge is a manually created
opening-stock invoice built by §6.4. If an operator forgets to run it, the year simply opens
empty; if they run it twice, stock doubles. Nothing detects either case. The generic
carry-forward machinery (`EnteghalU`, `docs/03-accounting-core.md`) covers the ledger, not the
warehouse.

This is the single largest functional gap in the inventory domain and belongs in §14 and §15.

---

### 6.6 Purchase overheads, discounts and taxes: none of them affect cost

The question is answered exhaustively by the two live formulas:

| Element | Where it is stored | Enters `Anbar_Jens_Phi1.phiin`? | Enters `Anbar_Mandeh.Phiin1`? |
|---|---|---|---|
| Line discount `AFD_Kasr` | on the line | **No** — numerator is `AFD_Num × AFD_Phi` | **No** — numerator is `AFD_Kol`, which is pre-discount |
| VAT `AFD_Maliat` | on the line | **No** | **No** |
| Freight / carriage | **not modelled anywhere** | n/a | n/a |
| Customs, duty, insurance | **not modelled anywhere** | n/a | n/a |
| Purchase-side rounding | truncation, §6.1 | inherited | inherited |
| Header-level discount | **does not exist** — `AF_Kasr` is only the *sum* of the line discounts (`AnbarFactorU.pas:656-657`) | n/a | n/a |

There is no landed-cost concept, no cost-adjustment document, no revaluation and no
write-down/write-up. A purchase invoice's freight would have to be entered as a separate expense
voucher in the ledger, where it never meets the item.

Consequence for the rebuild: **cost = gross unit price of purchase lines** is the ported
behaviour. Any change (netting discounts, capitalising freight) alters reported margins and is a
§15 proposal, not a port.

---

### 6.7 Subsystem B has no costing at all

`Anbar.Dbo.FactorDetail` carries `FD_Phi`, `FD_Mab`, `FD_Kasr`, `FD_Maliat`, `FD_Total`,
`FD_VaznP` (§8.3.4) — transaction prices, not costs. `AnbarReportU.pas:203-222` aggregates
`Sum(FD_Num)`, `Sum(FD_Total)`, `Sum(FD_VaznP)` per document for an activity report. Nothing
derives an average, a balance or a cost. The external warehouse application owns whatever costing
exists there, and its source is unavailable (§5.1.5, §14).

For pistachio specifically (§8), the purchase "cost" is whatever `NR_Phi` the weighbridge
application wrote, applied to `NR_Vazn` — accepted without recomputation and posted straight to
`700-3-<grade>` (§8.4). The deduction arithmetic of §8.2 is, in effect, the pistachio costing
model: it converts a gross weighbridge weight into a billable net weight, and the unit price is
negotiated per lot.

---

### 6.8 Summary of the costing rules to port

```
unit_price(line)     := operator input; defaults to items.sale_price on EVERY document type
gross_amount(line)   := trunc(quantity × unit_price)
discount(line)       := absolute amount, or trunc(gross × pct / 100)
tax(line)            := trunc((gross − discount) × warehouse.vat_rate / 100)   -- §7
line_total(line)     := gross + tax − discount

advisory_cost(item)  := trunc(  Σ(quantity × unit_price) over purchases, current fiscal year,
                                                          excluding the invoice being edited
                              ÷ Σ quantity over the same set )
                        -- 0 when there are no purchases
                        -- never persisted; recomputed on every item lookup
                        -- discounts and tax excluded by construction
                        -- no warehouse dimension, no date cut-off, no carry-forward
cogs                 := NOT COMPUTED ANYWHERE
inventory_valuation  := NOT COMPUTED ANYWHERE except as a report column (§6.3)
```


---

[← 6. Costing and valuation (part a)](05-06-a-costing-and-valuation.md) | [index](00-index.md) | [7. Pricing (part a) →](05-07-a-pricing.md)
