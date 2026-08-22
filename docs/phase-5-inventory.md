# Phase 5 — Inventory

The largest domain, and the one with the most legacy contradictions: two competing inventory
subsystems, three disagreeing stock-quantity formulas, two dead costing implementations, and a
pistachio deduction calculator that was never actually reachable in the legacy UI. This phase builds
**one** clean inventory system — merging what the legacy kept split — and makes every documented
defect (B1, B2, B7, B8, B9, B19) a fixed, testable behaviour.

**Read `05-01-entity-model.md` §5.0 first.** The legacy has subsystem A (local, full CRUD) and
subsystem B (external, read-mostly, structurally incompatible item master). The rebuild has **one**
inventory system, merged from the start — do not build two and reconcile later.

---

## 5.1 Item master + warehouses + units of measure

**Goal:** one item master, one warehouse model, correctly capturing what subsystem A and B each got
right — not perpetuating their split.

**Build**

- `warehouses` (`Anbar_Config`): name, VAT rate (per-warehouse, no effective-dating — historic lines
  keep their stored rate, which is correct behaviour to preserve per `05-01.md` §1.1), six posting
  account links (purchase, purchase-return, sales, sales-return, discount, VAT — all FKs to
  `accounts`). Add a real delete/deactivate action — the legacy's warehouse delete menu item
  (`N2`) was declared and never implemented, making every warehouse permanent (`05-01.md` §1.1); the
  rebuild adds `is_active` and lets a warehouse be deactivated once empty.
- `units_of_measure` (`Anbar_Vahed`): id + name. The legacy had **no maintenance screen at all** for
  this table (`05-01.md` §1.3) — the rebuild gets one, since it's needed to seed the table at all.
  No conversion factor existed either; add one now (`base_unit_id`, `conversion_factor`) since its
  absence caused real truncation damage in the legacy's quantity handling (`05-01.md` §1.3's closing
  note) — this is a deliberate improvement, not scope creep, because the alternative is reproducing
  the truncation bug on purpose.
- `pistachio_grades` (`Kinds`): id + name, seeded with the 7-value enumeration recovered from
  `05-08-a.md` §8.1 (fandoghi, badami, kalleh-ghouchi, momtaz, ahmad-aghaei, akbari, dahan-bast).
  **Keep this a genuinely separate table from `items` and `accounts`** — the legacy let one integer
  double as grade id, item code, *and* an account-code segment (`05-01.md` §1.4, §8.1.1), which
  means "renumbering a grade silently repoints both the stock ledger and the general ledger." The
  rebuild relates these three concepts by explicit foreign key, never by shared integer value.
- `items` (merging `Anbar_Jens` + `Cala`): code (unique), name, specification, unit of measure,
  **home warehouse becomes a real many-to-many `item_warehouses` junction** (subsystem B's
  `Cala.C_Anbar` delimited-string design was closer to correct — one item can genuinely exist in
  multiple warehouses — but implemented properly with a junction table, not a CSV column,
  `05-01.md` §1.6's explicit call-out that the two item masters are "structurally incompatible" and
  merging requires exactly this). Sale price, min-stock threshold, VAT-applicable flag,
  negative-stock-allowed flag, tax authority item code (`SSTID`). Add `is_active` — the legacy had no
  discontinue flag, only hard delete (`05-01.md` §1.2's "absent concept" table).
- `AJ_Alarm` (min-stock) becomes an actual alert, not a display-only number the legacy never checked
  (`05-01.md` §1.2's note that it's "displayed, never checked") — surface it as a real low-stock
  indicator once 5.3's stock query exists; this is a decided, cheap improvement, not scope creep,
  flagged here so it doesn't get silently dropped.

**Spec refs:** `05-01-entity-model.md`; `05-02-a/b-item-master-crud-rules.md`.

**Manual test**

1. Create two warehouses, each with its own six posting accounts and VAT rate.
2. Create a unit of measure with a conversion factor (e.g. "tonne" = 1000 × "kg") and confirm it's
   usable from the UI (no direct-SQL-only table, unlike the legacy).
3. Create an item, assign it to both warehouses via the junction table → confirm it's queryable from
   either warehouse context, unlike the legacy's single-scalar `AJ_ID`.
4. Set an item's min-stock threshold, bring its computed on-hand (once 5.3 exists) below it → confirm
   a real alert appears, not just a passive number next to the balance.

**Done when:** the item/warehouse model supports genuine multi-warehouse items through a real
relational structure, with no legacy CSV-column or single-scalar limitation carried forward.

---

## 5.2 Invoice (Factor) documents

**Goal:** purchase/sale/production/transfer document types, with the counterparty-validation bug
fixed (B7).

**Build**

- `inventory_documents` (merging `Anbar_Factor` + `FactorMaster`) + `inventory_document_lines`
  (merging `Anbar_FactorD` + `FactorDetail`), with a real **status column** — `draft`/`posted`/
  `frozen`. The legacy had no status column at all on subsystem A; its lifecycle was "entirely
  derived" from whether a row exists, the linked voucher's `M_Tx`, and whether treasury links exist
  (`05-04-a.md` §4.0). The rebuild makes this explicit rather than derived from three other tables —
  it's simpler and it's what subsystem B already had the right instinct for (`FM_Lock`, per the same
  section).
- Document types: `receipt` (purchase), `issue` (sale), `purchase_return`, `sales_return` — one
  vocabulary, not the legacy's three different label sets for the same four type codes across
  different screens (`05-05-a.md` §5.1.1's "same numbers, three vocabularies" finding).
- **B7 fix**: counterparty is **required** and validated as a leaf account before save, full stop.
  The legacy's guard was `if not S_Bed.tag=0`, which Pascal parses as `(not S_Bed.tag)=0` — an
  operator-precedence bug that made the check unreachable, so invoices saved with `AF_Customer=0`
  (`05-04-a.md` doesn't cover this directly but `11-open-decisions.md` B7 does — cross-check against
  `05-10-a.md` §10.1.3 defect 1, which confirms "the counterparty can be 0" downstream). Write the
  check as a straightforward boolean, not a precedence-sensitive one-liner, and add a test that would
  have caught the original bug (assert the check actually rejects a zero counterparty).
- State transitions: draft (fully editable/deletable) → posted (voucher exists, still editable per
  legacy's actual model, not deletable if settled) → frozen (voucher left draft state — read-only).
  Matches `05-04-a.md` §4.1's real state diagram, made explicit rather than derived.
- Create/edit/delete permission checks (1404–1408, 1414) from `05-04-a.md` §4.1's table — enforced
  server-side (per Phase 1.3), not only by disabling buttons like the legacy.

**Spec refs:** `05-01-entity-model.md` §1.5; `05-03-a/b-document-types.md`;
`05-04-a/b/c-invoice-factor-lifecycle.md`; B7 in `11-open-decisions.md`.

**Manual test**

1. Attempt to save an invoice with no counterparty selected → rejected (directly re-testing the B7
   fix — the legacy would have let this through).
2. Create a purchase invoice (`receipt`) with a valid counterparty and line items → saves as draft.
3. Edit the draft → succeeds. Post it (once linked to accounting in 5.8) → becomes non-deletable if
   settled, read-only once its voucher leaves draft — confirm both transitions.
4. Confirm the same four document types (receipt/issue/purchase_return/sales_return) are labelled
   consistently everywhere in the UI — no screen shows a different name for the same type code.

**Done when:** no invoice can be saved without a valid counterparty, and the document's status is a
real, queryable column — not something every screen has to re-derive.

---

## 5.3 Stock quantity mathematics + stock card

**Goal:** one canonical on-hand formula — the legacy has three that disagree (`05-05-a.md` §5.1).

**Build**

- **Pick the `Anbar_Mandeh` report's formula as canonical** (`05-05-a.md` §5.1.2): direction from
  document type (`receipt`/`sales_return` = inbound, `issue`/`purchase_return` = outbound, quantities
  always stored positive), date-windowed opening + period movement. This is the more complete of the
  two live formulas (it has a real date window; the line-entry lookup does not) — the rebuild
  computes on-hand the same way everywhere, not once per screen with different rules. Do **not** port
  the third, dead implementation (hard-coded fiscal year, latest-transaction pricing, `05-05-a.md`
  §5.1.4) — it was never live.
- Compute on demand from `inventory_document_lines`, same as the legacy (no stored balance column
  anywhere, and that's fine — keep it that way; it's the one legacy design choice that was already
  correct and simple). Use real `date` comparisons, not the legacy's lexicographic string comparison
  on zero-padded Jalali strings (`05-05-a.md` §5.1.2's note on this).
- When editing an existing document, exclude that document's own lines from the on-hand calculation
  (the legacy did this correctly, if by accident of an empty-string comparison on new invoices,
  `05-05-a.md` §5.1.3) — implement it deliberately: exclude by the document's real id, always.
- Stock card (item ledger): a query surface, not a stored artifact — running balance per item over
  time, computed from the same canonical formula.
- Warehouse scoping: unlike subsystem A's "one global stock pool per item code," use the real
  `item_warehouses` relationship from 5.1 to scope on-hand queries per warehouse when a warehouse
  filter is requested — this is possible now because the schema is relational, not a hazard the
  legacy could avoid.

**Spec refs:** `05-05-a/b-stock-quantity-mathematics.md`; `05-11-stock-card-and-balance.md`.

**Manual test**

1. Post a receipt of 100 units, a sale of 30 units, a sales return of 5 units for the same item →
   on-hand = 75. Confirm this matches manual arithmetic using the canonical formula.
2. Query on-hand as of a date **before** the last movement → confirm the date-windowed opening
   balance is correct (this directly exercises the date-window feature the legacy's other, inferior
   implementation lacked).
3. Open the item for editing mid-transaction (simulate an in-progress edit) → confirm the document's
   own lines are excluded from the on-hand shown, by id, not by an empty-string accident.
4. View the stock card for the item → running balance matches the on-hand query at every point in
   time.

**Done when:** there is exactly one on-hand formula used everywhere in the system, and it is
provably correct against hand-computed arithmetic.

---

## 5.4 Costing & valuation

**Goal:** implement the legacy's one real costing signal (weighted-average-of-purchases, advisory
only) correctly and singularly — the legacy has two implementations that disagree (`05-06-a.md`
§6.2–§6.3).

**Build**

- **There is no automatic costing engine, and the rebuild doesn't invent one** — this matches the
  legacy's actual, if informal, design: unit price is manually entered on every line, with a
  **weighted-average-of-purchases** figure offered as a suggestion (`05-06-a.md` §6.0's four
  headline facts). Don't build FIFO or perpetual average costing that was never there — that would be
  scope creep beyond what the spec establishes as the domain's real behaviour.
- Pick **one** average-cost formula (the `Anbar_Mandeh`-style date-windowed version, matching 5.3's
  canonical stock formula, for consistency) rather than the legacy's two implementations that use
  different amount sources (recomputed `qty × price` vs. the stored, truncated gross amount) and
  different exclusion rules (`05-06-a.md` §6.3's four-row differences table).
- Use `rust_decimal` for the average computation, round to whole rials only at the point the figure
  is offered to the operator (per `10-target-architecture.md` §2.3's money-handling stance) — do
  **not** reproduce the legacy's silent truncation-not-rounding on every line amount
  (`05-06-a.md` §6.1's "systematic downward bias" finding); round correctly, once, at the boundary.
- The average-cost suggestion must be an explicit, visible action in the UI (a real button) — not the
  legacy's undiscoverable click-a-read-only-label interaction (`05-06-a.md` §6.2's closing note
  flags this explicitly as something "the React rebuild must replace with a real button").
- Unit price still defaults sensibly — but **not** to the item's sale price on every document type
  including purchases, which was a real legacy bug (`05-06-a.md` §6.1: "on a goods receipt the price
  box is pre-filled with the *selling* price"). Default purchase-type documents to the average-cost
  suggestion (once purchases exist) or leave blank, never to `sale_price`.

**Spec refs:** `05-06-a/b-costing-and-valuation.md`.

**Manual test**

1. Reproduce the worked example from `05-06-a.md` §6.2.1: three purchases of 100/60/40 units at
   50,000/65,000/72,500 rial → confirm the average-cost suggestion is exactly 59,000 rial/kg (trunc
   of 11,800,000/200).
2. Create a new receipt (purchase) line for the same item → confirm the price box does **not**
   default to the item's sale price (directly re-testing the fixed default).
3. Click the "use average cost" button → confirm it fills the price field, and confirm the action is
   a visible, discoverable button, not a disguised label click.

**Done when:** the average-cost figure is correct, uses one formula everywhere, and the purchase-line
default-to-sale-price bug cannot recur.

---

## 5.5 Pricing

**Goal:** the item's list/sale price and how it flows onto a line.

**Build**

- `sale_price` on `items` (5.1) is the base price; line-level price can be overridden per the
  original design intent (operator can always overtype). Read `05-07-a/b-pricing.md` in full before
  implementing — it is not summarised further here since it's a smaller, more self-contained section
  than costing.
- Discount: entered as either an absolute amount or a percentage (matching the legacy's dual entry
  mode, `05-06-a.md` §6.1) — `discount_amount = round(gross_amount × discount_pct / 100)` when
  entered as a percentage, computed with correct rounding (not truncation).

**Spec refs:** `05-07-a/b-pricing.md`.

**Manual test**

1. Create a line with a percentage discount → confirm the computed discount amount is correctly
   rounded, not truncated.
2. Override the line price manually → confirm it doesn't get silently reset to the item's list price.

**Done when:** pricing and discount entry match the spec's documented dual-mode behaviour with
correct rounding.

---

## 5.6 Pistachio deduction calculator

**Goal:** the domain's signature formula, finally reachable in the UI (B19) — the legacy wrote the
correct arithmetic (`PestehD_U.pas`) but never shipped a way to use it (dead panel, dead Save
button, `05-08-a.md` §8.0.1).

**Build**

- Implement the deduction formula **exactly** as specified in `05-08-a.md` §8.2.2 — this is a
  "preserve exactly" case, not a "fix the defect" case; the arithmetic itself was never wrong, only
  unreachable:

  ```
  tare_deduction     = bale_count × tare_allowance_per_bale     (allowance ∈ {0.1, 0.2, 1.0} kg)
  moisture_deduction = moisture_pct × gross_weight / 100
  blank_deduction    = blank_pct    × gross_weight / 100
  total_deductions   = tare_deduction + moisture_deduction + blank_deduction + other_deductions_kg
  net_weight         = gross_weight − total_deductions
  if gross_weight < total_deductions: net_weight = 0        -- only net_weight is floored, not the deduction total
  line_amount        = round(net_weight × unit_price)
  ```

  **Preserve precisely**: percentages apply to gross weight independently (not compounded — moisture
  and blanks are each computed off the same base and added, not chained), `other_deductions` is
  entered in kilograms directly (not a percentage), and only `net_weight` is floored at zero — the
  deduction total itself can legitimately exceed gross weight and still display as-is.
- **Rounding**: the legacy's `Round` is Delphi's round-half-to-even; PostgreSQL/`rust_decimal`
  default to round-half-away-from-zero. `05-08-a.md` §8.2.2 flags this explicitly as "a real
  behavioural difference." **Decide and document which rounding rule the rebuild uses** — recommend
  matching Postgres/`rust_decimal`'s round-half-away-from-zero (the more common accounting
  convention) rather than emulating Delphi's banker's rounding, and note the change explicitly in the
  UI copy/changelog since it shifts results by ±1 rial on exact-half amounts.
- **Reachability — the actual fix.** Build this as a first-class step in the purchase-invoice flow
  for pistachio-grade items, not a hidden panel behind an unwired Save button. Every field from the
  legacy's `PestehD_U` dictionary (`05-08-a.md` §8.2.1) is present and editable; the three "mandatory"
  fields (bale count, gross weight, unit price) are enforced as real required fields — not the
  legacy's cosmetic red-label-only validation that let a zero-value save through unchecked.
- Link grade (`pistachio_grades`), item, and account by explicit foreign key (per 5.1's decision),
  never by shared integer convention.

**Spec refs:** `05-08-a/b/c-pesteh-pistachio-specialisation.md` §8.0–§8.2; B19 in `11-open-decisions.md`.

**Manual test**

1. Reproduce Example A from `05-08-a.md` §8.2.3 exactly: grade Ahmad-Aghaei, 40 bales @ 0.2 kg tare,
   2000 kg gross, 3.5% moisture, 2% blanks, 5 kg other, unit price 1,250,000 rial/kg → confirm
   `net_weight = 1877.0 kg` and `line_amount = 2,346,250,000 rial`.
2. Reproduce Example B (the deduction floor): 40 bales @ 1.0 kg, 500 kg gross, 60% moisture, 45%
   blanks → confirm total deductions (565 kg) exceeds gross (500 kg), net weight floors to 0, line
   amount is 0 — and confirm the UI **does** let you see this before saving (not a silent block, but
   not silently saved unnoticed either — surface it clearly).
3. Attempt to save with bale count, gross weight, or unit price missing/zero → rejected (this is the
   validation-not-cosmetic fix — the legacy would have let this save).
4. Confirm this entire flow is reachable by a normal user through the purchase-invoice screen for a
   pistachio-grade item — no hidden panels, no dead buttons (this is the core B19 fix, verify it
   end-to-end).

**Done when:** both worked examples reproduce exactly, required-field validation actually blocks an
invalid save, and the whole calculator is reachable through the ordinary UI flow.

---

## 5.7 Settlement (Tasfieh)

**Goal:** attach treasury instruments (deposit slips, cheques) to an invoice — matching the legacy's
actual (thin) scope, not inventing an allocation algorithm it never had.

**Build**

- Link table/query: given an invoice, list every deposit slip and cheque whose `source_module`/
  `source_id` point back to it (per Phase 4's `deposit_slips`/`received_cheques` `source_kind`/
  `source_id` columns). **There is no settlement algorithm to build** — no FIFO matching, no
  running balance, no partial/full status (`05-09-a.md` §9.0's explicit "there is no settlement
  algorithm" finding). Do not invent one; port the thin linking behaviour faithfully.
- **Do** fix the one clear gap the legacy left unaddressed: show the invoice total and the sum of
  linked instruments **side by side with an actual computed difference** (an "outstanding" figure).
  The legacy displayed both figures and "never compares" them (`05-09-a.md` §9.3's closing note) —
  this is a cheap, obviously-correct addition, not new business logic, and it directly serves the
  screen's whole purpose. Still no over-payment *block* — just visibility, matching the spirit of
  "port as-is, add clarity" rather than inventing validation the business never asked for.
- Sort the combined list by date (the legacy's `UNION` had no `ORDER BY`, so deposit slips always
  sorted before cheques regardless of date, `05-09-a.md` §9.2's note) — a real `ORDER BY date` is a
  correctness fix, not a feature addition.

**Spec refs:** `05-09-a/b-settlement-tasfieh.md`.

**Manual test**

1. Create an invoice, attach a deposit slip and two cheques to it (via the treasury documents'
   source linkage from Phase 4).
2. Open the invoice's settlement view → confirm all three instruments appear, sorted by date, with a
   computed outstanding-amount figure (invoice total minus sum of linked instruments) — something the
   legacy screen never showed.
3. Over-settle the invoice (link instruments totalling more than the invoice amount) → confirm it's
   still permitted (matching legacy behaviour — no block) but the outstanding figure clearly shows a
   negative/over-paid amount rather than silence.

**Done when:** the settlement view shows every linked instrument correctly sorted with a real
outstanding-balance calculation, without introducing an allocation algorithm the legacy never had.

---

## 5.8 Inventory → accounting integration

**Goal:** every inventory document type posts a correctly balanced voucher — fixing B1 (purchase/
opening-stock imbalance) and B2 (production/transfer post nothing at all) structurally, via the
Phase 2.5 engine.

**Build**

- Posting rules per document type, using the six `warehouses` posting-account roles from 5.1 (this
  is the inferred shape from `05-10-a.md` §10.1.2 — the legacy's actual stored-procedure body was
  never recoverable from source, so this is the rebuild's own clean design, not a verified port):

  | Document type | Debit | Credit |
  |---|---|---|
  | `receipt` (purchase) | purchase account ← gross amount | counterparty ← line total |
  | `issue` (sale) | counterparty ← line total | sales account ← gross amount |
  | `purchase_return` | counterparty | purchase-return account |
  | `sales_return` | sales-return account | counterparty |

  Discount and VAT post to the warehouse's discount/VAT accounts on whichever side makes the voucher
  balance — **and it must balance, checked by the Phase 2.5 engine, which rejects unbalanced
  postings outright.** This is the structural fix for B1: the legacy's `MakeSanadU` "has no balance
  check" at all, and its purchase/opening-stock postings are confirmed out of balance by
  `2·discount − VAT` because input VAT was disabled behind `if false then` and the discount posted to
  the wrong side (`00-overview.md` fact 5, `11-open-decisions.md` B1). In the rebuild this class of
  bug cannot ship: the engine refuses to persist a posting that doesn't balance, so a wrong-side
  discount or a disabled tax line surfaces immediately as a rejected transaction, not as silently
  corrupted historical data.
- **B2 fix**: production and transfer document types get real posting rules, not "not implemented
  yet." Define them explicitly (production: debit finished-goods/inventory, credit raw-materials/
  work-in-progress at the appropriate warehouse accounts; transfer: a wash entry between the source
  and destination warehouse's inventory accounts, net zero across both legs) — read `05-03-b.md`'s
  document-type catalogue for the exact scope before implementing, and confirm with the posting rule
  table above extended to these two types. The acceptance bar is simple: **no inventory document
  type is allowed to exist in `posted` status with zero voucher lines.**
- **B8/B9 fix**: un-posting (deleting a posted document) removes *all* of its posting lines, not a
  hard-coded subset of `M_Id` values that missed opening-stock lines (B8, `11-open-decisions.md`) —
  since the rebuild's voucher lines reference their source document by real foreign key
  (`source_kind`/`source_id`, per Phase 2.3), deleting the source document's postings is a single
  `WHERE source_kind = ? AND source_id = ?` delete, not an enumerated list of magic id ranges that
  can miss a value. This also closes B9 (unremovable orphans from a reverse-voucher handler with an
  empty body) — there is no separate reverse-voucher code path to have an empty body; un-posting
  always goes through the same one deletion mechanism.
- Idempotent re-save: matches the legacy's correct instinct (delete-then-reinsert by source reference,
  `05-10-a.md` §10.0's engine table) but done as one transaction via the Phase 2.5 engine, not the
  legacy's one-stored-procedure-call-per-line pattern with no surrounding transaction
  (`05-10-a.md` §10.1.3 defects 3–4).
- Narration is accurate per document type — not the legacy's "goods sale" label hard-coded for all
  four types including receipts and returns (`05-10-a.md` §10.1.3 defect 6).

**Spec refs:** `05-10-a/b-accounting-integration.md`; B1, B2, B8, B9 in `11-open-decisions.md`.

**Manual test**

1. Post a purchase (`receipt`) invoice with a discount and VAT → confirm the resulting voucher is
   perfectly balanced (debit total = credit total) — this is the direct B1 regression test; construct
   a case that would have produced the legacy's `2·discount − VAT` imbalance and confirm it's
   impossible to save unbalanced.
2. Post a production document and a transfer document → confirm both produce real, balanced voucher
   postings, not silence (direct B2 test).
3. Un-post (delete) a posted purchase invoice that included an opening-stock line → confirm **every**
   posting line tied to that document is removed, including any that the legacy's hard-coded
   `M_Id in (32,33,35)` set would have missed (direct B8/B9 test).
4. Re-save an already-posted document (edit and save again) → confirm no duplicate lines, and confirm
   the operation is atomic (simulate a failure mid-save if feasible, confirm no partial state).
5. Check narration text on a receipt, an issue, and both return types → confirm each has an accurate,
   distinct description, not a shared generic "goods sale" label.

**Done when:** B1, B2, B8 and B9 each have a manual test that would have failed against the legacy
behaviour and passes against the rebuild.

---

## 5.9 Inventory screens

**Goal:** the on-screen equivalents of the item master, invoice editor, warehouse settings, and
invoice list.

**Build**

- Item master screen (create/edit item, assign warehouses via the junction table, set price/
  min-stock/VAT flags).
- Invoice editor: header + line grid, counterparty (required, per 5.2), item picker, quantity/price/
  discount entry, running total, average-cost-suggestion button (per 5.4), pistachio deduction
  calculator step for pistachio-grade purchase lines (per 5.6).
- Warehouse settings screen (the six posting-account links, VAT rate).
- Invoice list: filter by type/status/date, settlement view link (per 5.7), print/export entry points
  (full print/export pipeline is Phase 6).

**Spec refs:** `05-13-a/b/c-screen-specifications.md`.

**Manual test**

1. Walk through creating an item, a warehouse, and a full purchase invoice — including the pistachio
   deduction step for a pistachio-grade line — entirely in the browser.
2. Post the invoice, confirm the voucher is visible and balanced (per 5.8).
3. Filter the invoice list by type and status, confirm correct results.
4. Open the settlement view for the invoice from the list (per 5.7).

**Done when:** the full inventory workflow — item setup, warehouse setup, invoice entry with
pistachio deduction, posting, settlement — works end to end in the browser.
