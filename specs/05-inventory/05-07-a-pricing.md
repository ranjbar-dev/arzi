_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 7. Pricing

### 7.0 Summary

There is **one price per item** (`Anbar_Jens.AJ_Phi`, labelled `بهای فروش`, "sale price"), it is
used as the default on **every** document type including purchases, and the operator can overtype
it on any line. There are no price lists, no customer-specific prices, no quantity breaks, no
price validity dates, no currencies, no promotions and no margin control. Discount is per line.
VAT is one rate per warehouse, applied if the item is flagged taxable.

Everything below happens inside one form, `AnbarFactorAddU` — the line editor. Its Persian
caption is `افزودن به فاکتور` ("add to invoice", `AnbarFactorAddU.dfm:5`).

---

### 7.1 How a unit price is chosen

`TAnbarFactorAdd.CodeExit` (`AnbarFactorAddU.pas:110-158`) fires when the operator leaves the item
code box (`AnbarFactorAddU.dfm`, `Code.OnExit = CodeExit`) and is also called directly after the
item-search dialog (`:200`) and when loading a line for edit (`:99`).

```pascal
Phi.IntValue := DM.Anbar_Jens_Phi1.FieldByName('AJ_Phi').AsInteger;   // :125
I := Dm.Anbar_Jens_Phi1.FieldByName('Phiin').Asinteger;               // :127
phi1.Text := inttostr(i);                                             // :128
```

Three prices are on screen at once:

| Control | Persian label (`.dfm`) | English | Source | Editable |
|---|---|---|---|---|
| `Phi` | `قیمت کالا` (`sLabel3`) | Item price | `AJ_Phi`, the item's sale price | **yes** — this is the transaction price |
| `Phi1` | `متوسط قیمت خرید` (`sLabel9`) | **Average purchase price** | `Anbar_Jens_Phi1.phiin`, §6.2 | no, read-only, `TabStop = False` |
| `AJ_Phi` on the item master | `بهای فروش` (`AnbarCalaAddU.dfm:88`) | Sale price | keyed by the item maintainer, mandatory (§2) | via the item screen |

`sLabel9`'s caption is the application's own name for the average of §6.2 and confirms its
intent: **average purchase price**, i.e. cost.

**The only way to price a line at cost** is to click the read-only `Phi1` box
(`AnbarFactorAddU.pas:161-164`):

```pascal
procedure TAnbarFactorAdd.Phi1Click(Sender: TObject);
begin
     phi.intvalue := phi1.intvalue;
end;
```

There is no keyboard equivalent, no button, no visual affordance that the box is clickable, and
no hint text. §6.2 flags this; the React rebuild must expose it as a real control.

> **Defect — purchases default to the selling price.** `:125` runs unconditionally, with no branch
> on `AnbarFactor.AF_Type.Tag`. Creating a **goods receipt** (type 1) pre-fills the price box with
> the item's *sale* price. If the operator tabs past it the purchase is booked at retail, which
> then feeds straight into the average-cost calculation of §6.2 and inflates it permanently.
> Nothing warns. The correct default for types 1 and 3 is the average purchase price (`Phi1`) or
> the last purchase price; for types 2 and 4 it is `AJ_Phi`.

**What does *not* influence the price:**

| Absent mechanism | Note |
|---|---|
| price list / price book | no table exists |
| customer-specific or group price | `AF_Customer` is never consulted when pricing |
| quantity break / tiered pricing | `Num` affects only the extension |
| effective-dated prices | `AJ_Phi` is a single scalar; changing it silently repricess all future lines. Historic lines keep `AFD_Phi`. |
| currency / exchange rate | everything is rial |
| cost-plus / margin rule | no cost is stored (§6.0) |
| minimum-price / below-cost guard | none — a line may be priced at 0, and `B_OKClick` only rejects `Total = 0` |
| price history / audit | none |

---

### 7.2 Discount

Two co-existing input paths, both feeding `AFD_Kasr` (`تخفیف`, `sLabel8`).

**Path 1 — absolute amount.** `Kasr: TEditInt` (`AnbarFactorAddU.dfm`, `IntLength = 14`), keyed
directly, `OnChange = PhiChange`.

**Path 2 — percentage.** `KasrD: TEditDecimal` (`IntLength = 2, DecimalLength = 2` → range
`0.00`–`99.99`), Persian label `درصد` ("percent", `sLabel12`), `OnChange = KasrDChange`:

```pascal
procedure TAnbarFactorAdd.KasrDChange(Sender: TObject);
begin
    kasr.intvalue := Round(int(kol.IntValue * kasrd.FloatValue / 100 ));
end;
```

```
discount = trunc( gross_amount × discount_pct / 100 )
```

The percentage applies to the **gross** amount (`Kol = Num × Phi`), before tax. `KasrD` is not
persisted — only the resulting `Kasr` reaches `Anbar_FactorD`.

> **Defect — the discount percentage does not re-apply when quantity or price changes.**
> `KasrDChange` reads `kol.IntValue` at the moment the percentage is typed. `PhiChange` (which
> fires on `Phi`, `Num` and `Kasr`) recomputes `Kol`, `Maliat` and `Total` but **never re-derives
> `Kasr` from `KasrD`**. Two consequences:
> 1. Typing the discount percentage *before* the quantity yields `Kasr = 0` (because `Kol` is
>    still 0), and it stays 0 when the quantity is entered afterwards. The line ships with no
>    discount and a percentage box showing e.g. `5.00`.
> 2. Correcting the quantity after setting a percentage leaves the old absolute discount, which no
>    longer corresponds to the percentage displayed.
>
> The tab order makes case 1 unlikely but not impossible: `Code` is `TabOrder = 0`,
> **`KasrD` is `TabOrder = 2`**, and `Phi` / `Num` come later. Tabbing straight through the form
> in order reaches the discount percentage *before* the quantity.

> **`KasrD` is reset to `'0'` when a line is re-opened for editing** (`AnbarFactorAddU.pas:97`,
> `KasrD.Text := '0';`) while `Kasr` keeps its stored value (`:98`). Editing an existing line
> therefore shows a 0 % discount alongside a non-zero discount amount, and touching the percentage
> box wipes the amount.

**No header-level discount exists.** `Anbar_Factor.AF_Kasr` is purely `Sum(AFD_Kasr)`
(`AnbarFactorU.pas:656-657`). A whole-invoice discount must be spread across lines by hand.

**No discount limit, no approval, no discount reason.** A 99.99 % discount is accepted silently.

---


---

[← 6. Costing and valuation (part b)](05-06-b-costing-and-valuation.md) | [index](00-index.md) | [7. Pricing (part b) →](05-07-b-pricing.md)
