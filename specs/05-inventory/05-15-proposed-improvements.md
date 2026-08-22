_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 15. PROPOSED IMPROVEMENTS (needs user approval)

> **Nothing in this section is part of the specification.** The default decision is
> **port-as-is**: reproduce the legacy behaviour exactly, defects included, so that the rebuilt
> system agrees with the old one on historic data. Every item below is a *proposal* that changes
> observable behaviour and therefore requires explicit sign-off. They are grouped by whether they
> are (A) defect fixes, (B) missing capability, or (C) structural changes.
>
> The "Risk" column assumes the change is made **without** a compensating migration; most risks
> are mitigable, and the mitigation is stated.

---

### 15.A Defect fixes

#### A1. Close the unreachable counterparty validation

| | |
|---|---|
| **Current** | `if not S_Bed.tag=0 then` parses as `(not S_Bed.Tag) = 0`, true only when `Tag = -1`, which never happens. An invoice saves with `AF_Customer = 0`. `AnbarFactorU.pas:579-583`, §4.2.2. |
| **Proposed** | Require a resolved counterparty account before saving any inventory document. |
| **Why** | The voucher lines produced for such an invoice point at account id 0. It is a silent data-integrity hole with no upside. |
| **Risk** | **Historic data will contain `AF_Customer = 0` rows (Q8).** A `NOT NULL` FK rejects the migration. Mitigation: make the column nullable in the schema, enforce the rule only in the service layer for *new* documents, and report the legacy rows for manual repair. |

#### A2. Make posting and un-posting symmetric

| | |
|---|---|
| **Current** | Posting writes `M_Id ∈ {31,32,33,34,35}`; un-posting deletes only `{32,33,35}`. `M_Id = 31` and `34` lines survive as orphans; `34` orphans are **permanently unremovable** because no code path covers them. §5.3, §8.4. |
| **Proposed** | One `unpost(document)` operation over one declared set of document types, deleting by `voucher_lines.source_document_id` with a real foreign key. |
| **Why** | Orphaned voucher lines silently misstate the trial balance and nothing in the UI can find them. |
| **Risk** | Low for new data. **Existing orphans must be found and cleaned before migration (Q7)** or they will be carried into the new ledger with no owner. |

#### A3. Balance every voucher before writing it

| | |
|---|---|
| **Current** | `MakeSanadU.B_OkClick` begins with the comment `// Control data` and no validation. `init11` and `init12` produce vouchers that are out of balance by `2·Kasr − Maliat`: the discount is debited when it should be credited, and the VAT block is `if false then`. §10.2.5. |
| **Proposed** | (i) assert `Σ debit = Σ credit` in the posting engine and refuse to write otherwise; (ii) credit the discount on inbound documents; (iii) re-enable the input-VAT line. |
| **Why** | An accounting system that can write an unbalanced journal entry is not an accounting system. |
| **Risk** | **High if Q6 returns non-zero.** Fixing (ii) and (iii) changes the accounting treatment of every future inbound document with a discount or VAT, and makes new vouchers disagree with historic ones. Mitigation: fix (i) unconditionally; make (ii) and (iii) an accountant's decision, applied from a cut-over date. |

#### A4. Remove the table-wide UPDATE from the activity report

| | |
|---|---|
| **Current** | `Anbar_Amalkard.makequery1/2/3` each run `Update Anbar_FactorD Set AFD_Customer = (…)` with **no `WHERE` clause** before selecting. A report rewrites every row of the movement table, in every fiscal year. §13.10. |
| **Proposed** | Delete `AFD_Customer` entirely; join the header for the counterparty. |
| **Why** | A denormalised column that drifts and is repaired by a report is not a design. It is also a long blocking write triggered by a read-only user. |
| **Risk** | Low. Verify with Q14 that nothing depends on a stale value. |

#### A5. Re-apply the discount percentage when quantity or price changes

| | |
|---|---|
| **Current** | `KasrDChange` computes the discount from the *current* `Kol`; `PhiChange` recomputes `Kol` but never re-derives the discount. Entering the percentage before the quantity yields a zero discount, silently — and the tab order reaches the percentage first. §7.2. |
| **Proposed** | Store the discount as `(mode, value)` — percentage or amount — and derive the amount on every recomputation. |
| **Risk** | Low. The stored `AFD_Kasr` is unchanged; only new entry behaves differently. |

#### A6. Replace every silent `Exit` with a real validation message

| | |
|---|---|
| **Current** | `AnbarCardJensiU.B_OKClick` fails silently four times (`:83-104`); `AnbarReportKharidU` three times; `AnbarFactorAddU.B_OKClick` discards a line whose total is zero with no message (`:184`). |
| **Proposed** | Every rejected action reports why. |
| **Risk** | None. |

#### A7. Fix the `Kh4_Del` and `B_Delete` permission bugs

| | |
|---|---|
| **Current** | `Kharid_BU.pas:267` clears `Kh3_Code.Tag` instead of `Kh4_Code.Tag`. `TasfiehFactor.pas:96` enables the Delete button on permission `2118` (deposit slip) only, so a user holding `2104` (delete cheque) cannot delete a cheque. §8.5, §9.4.3. |
| **Proposed** | Fix both. |
| **Risk** | None. `Kharid_BU` is unreachable anyway (§8.0.1). |

#### A8. Gate renumbering on the voucher being unposted

| | |
|---|---|
| **Current** | `AR_ReNoClick` never calls `Moein_TX`. A frozen invoice can be renumbered, rewriting `Moein.M_Link` on a finalised voucher. Every other mutation is gated. §4.4. |
| **Proposed** | Apply the same `M_Tx = 0` gate. |
| **Risk** | Low; it removes an ability that should not exist. |

---

### 15.B Missing capability

#### B1. Fractional stock must survive the year boundary

| | |
|---|---|
| **Current** | The year-opening generator reads quantities with `.AsInteger` and drops items with `Remi <= 0`. 12.75 kg carries forward as 12 kg; a negative balance carries forward as nothing. §6.4. |
| **Proposed** | Exact decimal throughout; carry negatives forward as negatives. |
| **Why** | This is silent, permanent, annual data loss on a kilogram-traded commodity. |
| **Risk** | Opening balances in the new system will not match the old system's for any item with fractional stock. That is the point, but it must be expected. |

#### B2. A real year-end carry-forward

| | |
|---|---|
| **Current** | Both stock and cost reset to zero at the start of each fiscal year. The only bridge is an operator remembering to run a menu item on the invoice screen. Running it twice doubles stock; forgetting leaves the year empty. Nothing detects either. §6.5. |
| **Proposed** | A first-class, idempotent `period_open` operation with its own document type, run once, detectable and reversible. |
| **Risk** | Medium — it changes an established operational procedure. |

#### B3. An `is_active` flag on items

| | |
|---|---|
| **Current** | Items can only be hard-deleted, and only if never invoiced. **There is therefore no way to retire a used item at all** — it stays in every picker and report forever. §2.4. |
| **Proposed** | `items.is_active`, filtered out of pickers, retained in history. |
| **Risk** | None. |

#### B4. An adjustment / stock-count document type

| | |
|---|---|
| **Current** | Subsystem A has only receipt / issue / purchase-return / sales-return. A stock correction must be booked as a receipt or issue **against some counterparty account**, forcing the operator to misattribute it. §3.1. |
| **Proposed** | An `adjustment` type posting to a stock-variance account. |
| **Risk** | Requires a new account in the chart of accounts and an accountant's decision on its treatment. |

#### B5. Post production and transfer

| | |
|---|---|
| **Current** | `FM_ID ∈ {15,25,16,26}` move stock with **no accounting entry whatsoever** — `' Not implemented yet. '`. Raw materials are consumed and finished goods appear and the ledger never learns. §3.2.4, §10.5. |
| **Proposed** | Post transfers between warehouse accounts and production between materials, WIP and finished goods. |
| **Risk** | **High.** This is a new accounting model, not a fix. Requires Q26 and an accountant. It also means the new system's ledger will not reconcile to the old one for any period containing production. |

#### B6. Derived settlement status

| | |
|---|---|
| **Current** | "Is this invoice paid?" is not answerable without summing `DFish` and `DCheck` by hand; the settlement screen shows the invoice total and the settled total side by side and never subtracts them; over-settlement is unlimited. §9.3, §9.5. |
| **Proposed** | `settled_amount`, `outstanding_amount`, `settlement_status`, plus an over-settlement rule (reject / warn / allow — a decision). |
| **Risk** | Low, additive. |

#### B7. A maintenance screen for units of measure

| | |
|---|---|
| **Current** | `Anbar_Vahed` has no screen; rows can only be added by direct SQL. §1.3. |
| **Risk** | None. |

#### B8. Enforce the minimum-stock level

| | |
|---|---|
| **Current** | `AJ_Alarm` is displayed in a read-only box on the line editor and checked by nothing. §1.2. |
| **Proposed** | A warning on issue, and a below-minimum report. |
| **Risk** | Low; the data already exists. |

---

### 15.C Structural changes

#### C1. Merge the two inventory subsystems

| | |
|---|---|
| **Current** | Two item masters (`Anbar_Jens`, `Cala`) with separate code spaces, two document tables, two document-type enumerations, two posting-account tables (`Anbar_Config`, `Anbar.dbo.Anbar`), two posting engines, and only one of them has a warehouse dimension or a weight column. They are two implementations of the same idea that disagree. §5.0, §1.6, §3.4. |
| **Proposed** | One `items`, one `warehouses`, one `inventory_documents`, one `inventory_document_lines` with both `quantity` and `weight`, one `document_types`, one posting-rule table, one posting engine. `Cala.C_Anbar`'s comma-delimited warehouse list becomes an `item_warehouses` junction table. |
| **Why** | Nearly every defect in this document is a consequence of the split. |
| **Risk** | **Highest-effort item here.** Subsystem B is owned by an application not in this repository and its stock rules are unavailable (§5.1.5); they must be recovered or re-specified. Item codes must be reconciled across two code spaces. Do not attempt without Q17, Q18 and the other application's source. |

#### C2. Link by surrogate key, everywhere

| | |
|---|---|
| **Current** | Five different link conventions coexist: `Moein.M_Link` = `AF_Factor` (number) for subsystem A, = `FM_SSN` (key) for subsystem B, = `FM_Factor` (number) for pistachio; `DFish`/`DCheck.S_LinkSSN` = document number despite the name; `Moadian.M_Link` = `AF_SSN`. `Anbar_FactorD` links to its header by number. `FM_Link` points at the paired document by number. §10.4, §3.2.3. |
| **Proposed** | Every foreign key is a surrogate id with a declared referential action. `invoice_number` becomes a plain unique attribute. |
| **Why** | The renumber screen (§4.2.6) exists solely to keep five tables in step with a mutable business key. Any direct SQL renumber orphans records silently. |
| **Risk** | Migration must resolve every number-based link to an id, and must handle links that resolve to nothing or to more than one row (`FM_Factor` is only unique per `(COID, Anbar)` — §3.2.3). Expect a reconciliation report. |

#### C3. An explicit document status

| | |
|---|---|
| **Current** | Subsystem A has no status column; a document is "frozen" when `max(Moein.M_Tx)` for its voucher is `> 0`. Because vouchers are **merged by date** (§4.2.2 step 4), finalising an unrelated invoice's voucher freezes yours. Subsystem B has `FM_Lock` 0/1/2. §4.0, §4.5. |
| **Proposed** | `documents.status` = `draft` → `posted` → `reversed`, plus `posted_voucher_id`. |
| **Risk** | Changes when a document becomes read-only. Migration must derive an initial status from `M_Tx`. |

#### C4. A stable movement sequence

| | |
|---|---|
| **Current** | `AFD_SSN` is destroyed and recreated on every invoice edit (§5.4). Same-date ordering on the stock card is therefore unstable, and the running balance is an accumulation over an undefined order (§11.1.3). |
| **Proposed** | An append-only movement ledger with an immutable monotonic `sequence_no`; edits produce reversing entries rather than deletions. |
| **Risk** | This is the biggest conceptual change in the list. It also **fixes the audit-trail absence** noted in `docs/01-glossary.md` §6b. |

#### C5. One balance function, computed in SQL

| | |
|---|---|
| **Current** | Four expressions of "how much stock is there" — one unreadable (a stored procedure), one dead, and two live ones that disagree on the date window and on whether the current invoice counts. The running balance is computed in a FastReport script. §5.1, §11.3. |
| **Proposed** | One service function; running balances via a window function. |
| **Risk** | Low; the canonical formula is already established (§5.1.2). |

#### C6. Exact decimal arithmetic and one declared rounding mode

| | |
|---|---|
| **Current** | Money is truncated three times per line; average cost is truncated by `CAST(… AS int)`; the pistachio module uses Delphi `Round` (banker's, half-to-even); quantity × price is a binary floating-point multiply, so `4.35 × 100` can yield `434`. Two rounding modes in one application, plus a representation-error class. §6.1, §7.3.4, §7.6, §8.2.2. |
| **Proposed** | `NUMERIC` in PostgreSQL, `rust_decimal` in Rust, one declared rounding mode per operation. |
| **Why** | Not doing this guarantees that the rebuilt system will disagree with the old one by small amounts, unpredictably. Doing it guarantees it will disagree **predictably**, which is the only manageable outcome. |
| **Risk** | Recomputed historic totals will differ from stored ones by a few rial. Migration should **carry the stored values across unchanged** and apply exact arithmetic only to new data. |

#### C7. Decide: periodic or perpetual inventory

| | |
|---|---|
| **Current** | The warehouse computes perpetual quantities; the ledger has no stock asset account and no COGS. Stock quantity lives in `Anbar_FactorD`; stock **value** lives nowhere. §10.5. |
| **Proposed** | If perpetual: a stock account per warehouse, COGS posted on issue at the costing method chosen below. |
| **Risk** | **This is an accounting-policy change, not a technical one.** It requires the business's accountant and probably their auditor. |

#### C8. Decide the costing method explicitly

| | |
|---|---|
| **Current** | Cost is whatever the operator typed. The advisory average excludes returns, ignores discounts and VAT, has no date cut-off, no warehouse dimension and resets every fiscal year. Purchases default to the **selling** price. §6, §7.1. |
| **Proposed** | Pick one — moving weighted average, FIFO, or standard cost — and compute it as of the movement date. At minimum, default purchase lines to the average purchase price rather than the sale price. |
| **Risk** | Changes reported margins. The purchase-default fix (A-grade, low risk) can be made independently of the method decision. |

#### C9. Transactions, everywhere

| | |
|---|---|
| **Current** | The invoice save is five separate round trips with no transaction; `MakeSanadU.B_OkClick` is three; the pistachio receipt opens a transaction with no rollback path; `DMoein_Make` always runs outside. §5.4, §8.3.4, §10.1.3, §10.2.6. |
| **Proposed** | One transaction per business operation. |
| **Risk** | None. |

#### C10. Non-negative stock — **decide, do not default**

| | |
|---|---|
| **Current** | Negative stock is permitted by default (`AJ_Manfi = 1` on every new item, §2.2.2). The only check fires on sales, is opt-out per item, truncates fractions, ignores lines already in the current invoice, and is bypassed entirely by three of the five ways lines get created. The actual policy is "allow it, then let a human find it in a report". §5.2. |
| **Proposed** | Enforce at the service boundary, per item, honouring `allow_negative_stock`. |
| **Risk** | **Historic data contains negative balances (Q13).** Any hard database constraint rejects the migration. Mitigation: enforce in the service layer only, never as a check constraint; and keep the per-item opt-out. |

---

### 15.D Recommended sequencing

| Phase | Items | Rationale |
|---|---|---|
| **0 — before any code** | Q1–Q5 answered; Q6, Q7, Q8, Q13 measured | The specification is not complete without them |
| **1 — port as-is** | everything in this document except §15 | Establishes an oracle: the new system must agree with the old on historic data |
| **2 — zero-risk fixes** | A4, A5, A6, A7, B3, B7, C9 | No behaviour change visible in the data |
| **3 — integrity fixes** | A1, A2, A3(i), A8, B1, B6, C2, C3, C4, C5 | Each needs a migration step; each is independently shippable |
| **4 — policy decisions** | A3(ii)(iii), B2, B4, B5, C6, C7, C8, C10 | Require the business and an accountant |
| **5 — structural** | C1 | Requires another application's source; do not start earlier |


---

[← 14. Open questions](05-14-open-questions.md) | [index](00-index.md) | [16. Naming map (part a) →](05-16-a-naming-map.md)
