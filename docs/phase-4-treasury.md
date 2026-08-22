# Phase 4 — Treasury

Cheques, deposit slips, petty cash — each posts through the accounting-core engine from Phase 2.
This is the domain with the most confirmed defects (B10–B15); every one gets fixed here, not ported.

---

## 4.1 Cheque schema + state machine

**Goal:** the received-cheque lifecycle, with the legacy's state-code ambiguity resolved (B11).

**Build**

- `received_cheques` (`DCheck`) and `received_cheque_events` (`DCheck2`) per `06-01-entity-model.md`
  §1.1–§1.2. Drop the dead `TCheck` table entirely (declared, never read or written) and the three
  dead `S_Zssn`/`S_ZCR`/`S_ZName` endorsement columns from `received_cheques` — endorsement gets a
  real, separate model in step 4.3, not these leftover columns.
- Add the columns the legacy is missing outright: issuing bank, branch, account number, drawer name
  (today smuggled into free-text `S_Desc`), and real `deposited_at`/`cleared_at`/`bounced_at`/
  `returned_at` timestamps on the cheque itself, not only on the event log (`06-01.md` §1.1's
  "Missing columns" note).
- **State machine — B11 fix.** The legacy overloads state `1` for both "never deposited" and
  "deposited then bounced," leaves state `3` ("bounced," per the code comment) completely
  unreachable, and disagrees with its own event log on every bounce (event row says state 2, master
  row says state 1) — `06-02.md` §2.1's three findings. The rebuild uses a clean enum with a distinct
  value for every real state: `InHand`, `AtBank`, `Bounced`, `ReturnedToIssuer`, `Cleared`. `Bounced`
  is now genuinely reachable and distinct from `InHand`.
- Transition table, ported from `06-02.md` §2.3–§2.4 with the state-code fix applied (values renamed,
  logic unchanged) and one added transition:

  | Transition | From → To | Debit | Credit |
  |---|---|---|---|
  | Receive | *(none)* → `InHand` | notes-receivable-on-hand | payer |
  | Deposit to bank | `InHand` → `AtBank` | notes-in-collection | notes-receivable-on-hand |
  | Bounce | `AtBank` → `Bounced` | notes-receivable-on-hand | notes-in-collection |
  | Re-deposit a bounced cheque | `Bounced` → `AtBank` | *(same as Deposit)* | *(same as Deposit)* |
  | Collect/clear | `AtBank` → `Cleared` | operator-chosen bank account | notes-in-collection |
  | Return to issuer | `InHand` or `Bounced` → `ReturnedToIssuer` | payer | notes-receivable-on-hand |

  `Cleared` and `ReturnedToIssuer` are terminal, matching the legacy (`06-02.md` §2.4's reachability
  note). **B11 also implies re-deposit after a bounce is now a real, distinct transition** — the
  legacy's collapse of "in hand" and "bounced" into one state meant this couldn't be told apart from
  a first-time deposit; now it can, which is strictly more information, not a behaviour change to the
  posting itself.
- Every transition writes a `received_cheque_events` row — including the receipt itself, unlike the
  legacy, where "the history of a cheque always starts at its second event" (`06-02.md` §2.0). This
  makes the audit trail complete from day one.
- Validation: amount `> 0`, description required, payer account must be a leaf, voucher date inside
  the fiscal year — matching `06-02.md` §2.3 T1's field rules. Cheque number and due date remain
  free-text/unvalidated for now (matching legacy permissiveness — not a decided fix; note as-is).

**Spec refs:** `06-01-entity-model.md` §1.1–§1.2, §1.10; `06-02-cheque-state-machine.md`; B11 in
`11-open-decisions.md`.

**Manual test**

1. Receive a cheque → status `InHand`, one event row logged (unlike legacy, this exists from receipt).
2. Deposit it → `AtBank`.
3. Bounce it → `Bounced` (not `InHand` — confirm this is a genuinely distinct, queryable state,
   directly testing the B11 fix).
4. Re-deposit the bounced cheque → `AtBank` again.
5. Collect it → `Cleared`. Attempt any further transition → rejected, terminal.
6. Receive a second cheque and return it directly to the issuer (skipping deposit) → `ReturnedToIssuer`,
   terminal.
7. Attempt an illegal transition (e.g. `Cleared` back to `InHand`) → rejected.

**Done when:** every state in the enum is reachable, `Bounced` is queryable as its own state (not
inferred from free text), and the event log has a row for every transition including receipt.

---

## 4.2 Cheque accounting integration

**Goal:** every transition posts correctly, delete actually deletes, collection builds a real voucher
header.

**Build**

- Wire every transition from 4.1 to the Phase 2.5 automatic-voucher-generation engine, using the
  debit/credit pairs from the table above and the amount from the cheque (unchanged across its whole
  lifecycle — no partial deposits/collections/fees, matching `06-02.md` §2.5).
- **B13 fix**: the collection transition **must** call the same voucher-header path as every other
  transition. The legacy's `CheckVosoolU` is "the only treasury screen with no `DMoein_Make`/
  `Dmoein_UpdateMab` call" (`06-08.md` §8.5 defect 1) — since the rebuild's engine (2.5) always
  creates/updates the header as part of posting, this defect cannot recur structurally; confirm it in
  the manual test anyway.
- **B10 fix**: the bounce transition's posted event and the cheque's resulting state must agree — no
  possibility of the event log and the master row disagreeing on state, since the rebuild derives the
  displayed state from the same enum both places read (no denormalised label to drift, per C3).
- **B12 fix**: delete is a real operation, not a guarded-but-dead button. Only cheques in `InHand`
  state with no events beyond receipt can be deleted; deleting removes the cheque, its posting, and
  its (single) event row in one transaction. Reject with a clear reason otherwise — do not silently
  no-op like the legacy's bare `Exit;` (`06-02.md` §2.3 T3).
- One shared voucher per day is **not** carried forward — the legacy's `Get_NewSanad_DateID` pattern
  (all treasury documents dated the same Jalali day sharing one voucher, `06-08.md` §8.2) was a
  historical space-saving trick, not a business rule; each treasury document gets its own voucher via
  the Phase 2 engine. Note this explicitly as a deliberate simplification, not an oversight.
- Fix the account-hierarchy denormalisation entirely by not having one — `account_id` is the only
  reference on a voucher line (per Phase 2.3); there is no `M_Ko/M_Mo/M_Ta1/M_Ta2` copy to keep in
  sync, so the three inconsistent denormalisation strategies documented in `06-08.md` §8.4 (set-based
  join, client-side string-paste, post-hoc UPDATE-the-whole-shared-voucher) simply don't exist as a
  problem class in the rebuild.

**Spec refs:** `06-08-accounting-integration.md` §8.1–§8.5; B10, B12, B13 in `11-open-decisions.md`.

**Manual test**

1. Receive, deposit, and collect a cheque → confirm three distinct vouchers exist (not one shared
   daily voucher), each correctly balanced, and the collection voucher has a real header with correct
   totals (directly re-testing the B13 fix).
2. Bounce a deposited cheque → confirm the event log and the cheque's current state agree (B10 fix) —
   query both and compare.
3. Delete a freshly-received (never transitioned) cheque → confirm the cheque, its voucher, and its
   event row are all gone (B12 fix — this must actually work, unlike the legacy's dead button).
4. Attempt to delete a cheque that has been deposited → rejected with a clear reason.

**Done when:** B10, B12 and B13 are each independently verified fixed by a manual test that would
have failed against the legacy behaviour.

---

## 4.3 Cheque endorsement

**Goal:** a genuine third-party-transfer feature — the legacy never built one despite reserving
columns for it (B14).

**Build**

- New transition: `InHand` or `Bounced` → `EndorsedToThirdParty` (a new terminal state, not reusing
  the dead legacy `S_Zssn`/`S_ZCR`/`S_ZName` columns — those are dropped per 4.1).
- Posting: debit the beneficiary/third-party account, credit notes-receivable-on-hand — structurally
  the mirror of "return to issuer" (T7 in `06-02.md` §2.3) but pointing at a chosen third party
  instead of the original payer, which is the design `06-04.md` §4.3 infers from the abandoned schema
  shape (this is the inference the rebuild acts on, since nothing in the legacy code confirms or
  denies it — it's a new feature, not a port).
- Requires selecting a beneficiary account (leaf, via the account picker from Phase 2.2) and an
  endorsement date/description, same shape as the other cheque transitions.

**Spec refs:** `06-04-endorsement-transfer-third-party.md`; B14 in `11-open-decisions.md`.

**Manual test**

1. Receive a cheque, endorse it to a third-party account → status `EndorsedToThirdParty`, terminal.
2. Confirm the posted voucher debits the beneficiary and credits notes-receivable-on-hand for the
   cheque's amount.
3. Attempt any further transition on an endorsed cheque → rejected.
4. Confirm this is reachable through the UI (once 4.5 exists) — the legacy's version of this concept
   was invisible to the system entirely; this one must be a first-class, visible action.

**Done when:** endorsement is a real transition with a real posting, not present in the legacy at all.

---

## 4.4 Deposit slips + petty cash

**Goal:** the two flat (non-lifecycle) treasury documents — `Fish` and `Tankhah` — each posting
correctly.

**Build**

- **Deposit slips** (`deposit_slips`): a flat record — date, amount, payer, receiving bank account,
  channel (`PosTerminal`/`CashSlip`/`CardToCard`/`WireTransfer` — descriptive only, per `06-06.md`
  §6.2, affects narration text, nothing else). One posting: debit the bank account, credit the payer.
  **Fix the legacy's swapped narration** (`06-08.md` §8.5 defect 2 — the "by X" and "to Y" strings
  were pasted onto the wrong lines) — get the narration right on the correct side from the start.
  No line items — a deposit slip is always one amount, matching `06-06.md` §6.1's "groups nothing"
  finding; don't build a batching feature that was never there.
- **Petty cash** (`petty_cash_claims` + `petty_cash_claim_lines`): header (custodian account, date,
  description) + N expense lines (expense account, amount, description). Total is computed from the
  lines, not entered (matching `06-07.md` §7.2). One posting: N debit lines (one per expense line) +
  one credit line to the custodian account for the total. Fix the copy-paste "N persons" narration
  bug (`06-07.md` §7.3's note) — write a narration that actually describes expense lines, not people.
  There is still no fund/float/advance/replenishment concept — that remains genuinely out of scope
  per `06-07.md` §7.1's exhaustive "none of the following exists" list; the custodian's balance is
  just whatever the ledger says about their account (Phase 6 reporting covers reading it).
- Both documents use the Phase 2.5 engine for posting, so both get real transactional all-or-nothing
  posting — fixing the legacy's "separate transactions in the same batch" hazard (`06-08.md` §8.5
  defect 6) structurally, the same way 4.2 fixed it for cheques.

**Spec refs:** `06-06-deposit-slips-fish.md`; `06-07-petty-cash-tankhah.md`; `06-08.md` §8.3, §8.5.

**Manual test**

1. Create a deposit slip → confirm the posted voucher's narration correctly identifies the payer on
   the credit line and the bank on the debit line (not swapped).
2. Create a petty-cash claim with three expense lines → confirm three debit lines + one credit line
   to the custodian, total matches the sum of the lines, and the narration doesn't say "N persons."
3. Delete a deposit slip / petty-cash claim → confirm the voucher and lines are fully removed, no
   orphaned rows (unlike the legacy's partial-delete defects).

**Done when:** both documents post correctly-narrated, correctly-balanced vouchers and clean up fully
on delete.

---

## 4.5 Treasury registers/filters UI

**Goal:** working list screens for both cheque registers, deposit slips, and petty cash — the legacy
had unreachable filters and no fiscal-year scoping on several of these lists (B15).

**Build**

- Received-cheque register and issued-cheque register (kept as two distinct lists per the legacy's
  correct instinct there — `06-03-received-versus-issued-cheques.md` — just give them correct,
  distinct titles from the start rather than the legacy's shared stale caption).
- **B15 fix**: every list filter (status, date range, due-date aging) actually works. The legacy's
  cheque and deposit-slip lists had filter clauses "present but commented out" and state-filter
  buttons whose handler was `exit;` on the first line (`06-06.md` §6.5, `06-02.md` §2.1's dead
  `State3` button) — none of that ships; every visible filter control is wired to a real query
  parameter.
- Due-date aging view using the now-reliable `Bounced`/`InHand`/`AtBank` states (4.1) — the legacy's
  due-date filter silently excluded returned/endorsed cheques by hard-coded state number
  (`06-05-due-date-logic.md` — read before building the exact aging buckets).
- Petty-cash list stays fiscal-year-scoped and date-ordered (the one legacy list that already got
  this right, per `06-07.md` §7.6) — keep that behaviour.
- Endorsement (4.3) gets a visible action on the received-cheque register — unlike the legacy, where
  it had no UI at all.

**Spec refs:** `06-11-a/b/c-screen-specifications.md`; `06-05-due-date-logic.md`; B15 in
`11-open-decisions.md`.

**Manual test**

1. Filter the received-cheque register by each status → confirm results are correct and complete for
   each (directly re-testing B15 — the legacy would show unfiltered results here).
2. Filter by due-date range → confirm the aging view matches `06-05.md`'s documented logic.
3. Trigger every transition (deposit, bounce, collect, return, endorse, delete) from the register UI
   → confirm each opens the correct screen and updates the list on completion.

**Done when:** every visible filter control on every treasury list actually filters, and every
transition including endorsement is reachable through the browser.
