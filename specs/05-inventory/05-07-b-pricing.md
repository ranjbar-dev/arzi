_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 7.3 VAT (`مالیات`)

#### 7.3.1 Where the rate comes from

`CodeExit` (`AnbarFactorAddU.pas:136-146`):

```pascal
DM.AnbarJens.Close;
DM.AnbarJens.Open;
if DM.AnbarJens.Locate('AJ_Code', inttostr(Code.IntValue), [LoCaseInsensitive] )
Then Begin
    i := DM.AnbarJens.FieldByName('AJ_ID').AsInteger;
    DM.AnbarConfig.Close;
    Dm.AnbarConfig.Open;
    Dm.AnbarConfig.Locate('AC_ID', inttostr(i) , [LoCaseInsensitive] );
    Anbar.Text := DM.AnbarConfig.FieldByName('AC_Name').AsString;
    DMaliat := DM.AnbarConfig.FieldByName('AC_DMaliat').AsFloat;
    if Dm.AnbarJens.FieldByName('AJ_Maliat').AsInteger = 0 then DMaliat := 0;
End Else Begin …
```

```
vat_rate = if item.is_taxable then warehouses[item.home_warehouse].vat_rate_pct else 0
```

So the rate is **per warehouse**, selected via the item's **home warehouse** `AJ_ID`, and gated by
the item's own `AJ_Maliat` flag (`مشمول مالیات`, "subject to tax", `AnbarCalaAddU.dfm:35`).

Three consequences:

- **The document's warehouse is irrelevant** — subsystem A has none (§1.0). Two items on the same
  invoice can attract different VAT rates because they live in different warehouses.
- **There is no effective date on the rate.** Changing `AC_DMaliat` reprices every future line
  immediately; historic lines keep their stored `AFD_Maliat`, so the rate is snapshotted per line
  by accident rather than by design.
- **The rate is entered as a bare integer string.** `AnbarTanzimU.pas:199-206` restricts the
  keystrokes to `#8` and `'0'..'9'` — **so a fractional rate cannot be typed at all**; the decimal
  point is rejected. It is read back with `.AsFloat` (`:145`) and stored with `.AsString`
  (`AnbarTanzimU.pas:187`). The only validation on save is that the field is non-empty
  (`:171-175`) — **`0` is accepted, and so is `99999`.**

#### 7.3.2 The formula

`PhiChange` (`AnbarFactorAddU.pas:166-171`):

```pascal
kol.IntValue    := Round ( int ( Num.floatValue * Phi.IntValue ) );
Maliat.IntValue := Round ( int ( (kol.intvalue - Kasr.intvalue) * DMaliat / 100 ));
Total.IntValue  := kol.IntValue + Maliat.IntValue - kasr.IntValue ;
```

```
gross  = trunc( quantity × unit_price )
vat    = trunc( (gross − discount) × vat_rate / 100 )      ← VAT is on the NET of discount
total  = gross + vat − discount
```

**VAT is computed on the discounted amount**, which is correct Iranian VAT practice and must be
preserved.

**Rounding is truncation, three times per line.** `Int()` truncates toward zero and the result is
already integral, so the enclosing `Round()` is a no-op. There is no round-half-up anywhere.

**There is no invoice-level VAT.** `AF_Maliat` is `Sum(AFD_Maliat)` (`AnbarFactorU.pas:658-659`),
so the header VAT is the sum of per-line truncations, not the VAT of the header net. On a 40-line
invoice that is up to 40 rial of divergence from a header-level computation. Preserve the
line-level calculation.

**There is only one tax.** No second tax, no municipal levy (عوارض) separately tracked, no
withholding, no tax groups, no exemption certificates. `AC_DMaliat` is a single combined rate.

**VAT is never posted to the ledger on inbound subsystem-B documents** — the posting block is
`if false then` (§10.2.5). Whether subsystem A posts it is unknown because `Anbar_AddToFactor`'s
body is unavailable (§10.1.1).

#### 7.3.3 Worked arithmetic

Item 1001, taxable, home warehouse VAT rate 9 %. Line: 12.5 kg at 1 234 567 rial, 3.5 % discount.

```
Kol    = trunc( 12.5 × 1 234 567 )              = trunc(15 432 087.5) = 15 432 087
Kasr   = trunc( 15 432 087 × 3.5 / 100 )        = trunc(   540 123.045) =    540 123
Maliat = trunc( (15 432 087 − 540 123) × 9/100 )= trunc( 1 340 276.76 ) =  1 340 276
Total  = 15 432 087 + 1 340 276 − 540 123                              = 16 232 240
```

Stored: `AFD_Num = 12.500`, `AFD_Phi = 1 234 567`, `AFD_Kol = 15 432 087`,
`AFD_Kasr = 540 123`, `AFD_Maliat = 1 340 276`, `AFD_Total = 16 232 240`.

Header after `B_SaveClick` step 10: `AF_Mab = 15 432 087`, `AF_Kasr = 540 123`,
`AF_Maliat = 1 340 276`, `AF_Total = 16 232 240`.

The average-cost calculation of §6.2 would use `12.5 × 1 234 567 = 15 432 087.5` (recomputed, not
truncated), while the `Anbar_Mandeh` report of §6.3 would use the stored `15 432 087` — the
half-rial divergence documented there.

#### 7.3.4 A floating-point hazard in the extension

`Num.floatValue * Phi.IntValue` is a binary floating-point multiplication followed by truncation.
The width of `TEditDecimal.FloatValue` is not verifiable (`Tools` is binary-only, §8.2.4), but if
it is `Double` the classic representation error applies:

```
Num = 4.35, Phi = 100
  4.35 as an IEEE double is 4.3499999999999996447...
  × 100                    = 434.99999999999994
  Int(434.99999999999994)  = 434          ← expected 435
```

The line silently loses 1 rial and the invoice total is 1 rial short. `Num` allows two decimals
(`DecimalLength = 2`), so quantities like `4.35`, `8.7`, `1.15` are ordinary. If `FloatValue` is
`Extended` (80-bit) the same class of error occurs at different values, just less often.

**This is a real, reproducible discrepancy class and the rebuild must not inherit it.** Use exact
decimal arithmetic (`NUMERIC`/`rust_decimal`) throughout and define the rounding mode explicitly
(§15). It also affects §8.2's pistachio `Round(NabV × Phi)`.

---

### 7.4 Line acceptance rules

`B_OKClick` (`AnbarFactorAddU.pas:173-187`) is the only gate between the price boxes and the
invoice grid:

```pascal
if AnbarFactor.AF_Type.Tag = 2  then
if DM.Anbar_Jens_Phi1.fieldByName('AJ_Manfi').Asinteger <> 1 then
if Rem1.Tag - Num.FloatValue < 0 then
Begin
   MessageDlg('تعداد وارد شده بیشتر از موجودی انبار است', mterror, [mbok], 0);
   …
End;
if Total.IntValue =0 then Exit;
tag := 1;
Close;
```

| Rule | Effect |
|---|---|
| Negative-stock check | §5.2 — sales only, opt-out by item, integer-truncated, per-line |
| **`if Total.IntValue = 0 then Exit;`** | A line whose computed total is zero is **silently discarded** — the dialog closes nothing, `Tag` stays `0`, and the caller (`AnbarFactorU.pas:500`) simply returns. **No message is shown.** The operator sees the OK button do nothing. |
| — | **There is no item-code validation.** If `CodeExit` failed to find the item it clears `Code.Tag` (`:148`) but `B_OKClick` never checks it. A line for a non-existent item passes as long as `Total ≠ 0` — which requires a non-zero price, which the operator can type. |
| — | **There is no quantity validation.** Negative or zero quantity is not rejected directly; a zero quantity produces `Total = 0` and hits the silent-discard path. A **negative** quantity produces a negative total, which is `≠ 0`, so **it is accepted** — inverting the line's stock direction while `AFD_Type` still says otherwise. |
| — | **There is no price validation.** Zero price → `Total = 0` → silent discard. |

> **The silent `Exit` on `Total = 0` is exactly the "handler guarded by a bare `Exit;`" pattern.**
> It is reachable and it does something (it prevents the line), but the user gets no feedback at
> all. In the rebuild this must be an explicit validation message.

---

### 7.5 Pricing on the other paths

| Path | Price source | Discount | VAT |
|---|---|---|---|
| Invoice line editor (§7.1) | `AJ_Phi`, overtypable; or average cost by clicking `Phi1` | `Kasr` / `KasrD` | `AC_DMaliat` if `AJ_Maliat = 1` |
| Opening-stock generator `OP1` (§6.4) | `Anbar_Mandeh.AVin`, average inbound cost | **hard `0`** (`AnbarFactorU.pas:228`) | **hard `0`** (`:229`) |
| INI import `OP3` (§4.3) | whatever is in the file | whatever is in the file | whatever is in the file — **not recomputed** |
| Pistachio receipt (§8.3) | `NR_Phi`, from the weighbridge application | **always 0** (`FactorPesteh_U.pas:201, 210`) | **always 0** |
| Pistachio deduction calculator (§8.2, dead) | `Phi` keyed per lot | n/a — deductions are by weight, not by money | none |
| Subsystem B documents | `FD_Phi`, written by the external warehouse application | `FD_Kasr` | `FD_Maliat` |

**Two of these six paths bypass the pricing rules entirely** (`OP3` and the external application),
and one bypasses them by design (`OP1`).

---

### 7.6 Rounding: the complete picture

| Operation | Legacy behaviour | Site |
|---|---|---|
| line gross | `trunc(qty × price)` after a binary FP multiply | `AnbarFactorAddU.pas:168` |
| line discount from % | `trunc(gross × pct / 100)` | `:107` |
| line VAT | `trunc((gross − discount) × rate / 100)` | `:169` |
| line total | exact integer arithmetic | `:170` |
| header totals | exact sums of the truncated line values | `AnbarFactorU.pas:654-661` |
| average cost | `Cast(… as int)` — SQL Server float→int truncation | `Dmu.dfm:708`; `Anbar_MandehU.dfm:142` |
| pistachio line total | `Round(net × price)` — Delphi **banker's rounding**, half-to-even | `PestehD_U.pas:129` |
| invoice grand total | **no rounding to a currency unit** — rial to the unit | — |

**Note the inconsistency**: everything in the generic invoice truncates, while the pistachio module
rounds half-to-even. Two rounding modes in one application. The rebuild must pick one per
operation and state it; see §15 for the proposal and §8.2.2 for why the pistachio choice matters.

There is **no** "round the invoice total to the nearest 1 000 rial" behaviour, which Iranian
retail invoices often have. If the business expects it, it is a new requirement, not a port.


---

[← 7. Pricing (part a)](05-07-a-pricing.md) | [index](00-index.md) | [8. The Pesteh (pistachio) specialisation (part a) →](05-08-a-pesteh-pistachio-specialisation.md)
