# Phase 2 — Accounting Core

The double-entry engine. Every other domain (treasury, inventory) posts through what's built here —
build it solid before anything depends on it.

---

## 2.1 Chart of accounts schema + API

**Goal:** the 4-level `accounts` hierarchy (Kol/Moein/Tafsil1/Tafsil2), leaf-only posting, correct
uniqueness.

**Build**

- `accounts` table per `02-11-c-ddl-parties-and-accounts.md` (read it before migrating — this step
  only summarises): `id`, `tenant_id`, `level1_code`/`level2_code`/`level3_code`/`level4_code`
  (`int`, `0` above that level), `name`, `child_count`, `is_locked`, `party_id` (nullable, filled in
  Phase 3), address/phone/national-id/registration/economic-code/postal-code columns (these live on
  the account row itself per the glossary's `Taraf` correction — there is no separate counterparty
  table). Drop every legacy denormalised column (`S_Bed`, `S_Bes`, `S_Remi`, `S_Count`, `M_R`, `M_L`,
  `FullName`, `LineName`, `NeedUpdate`, the four unwritten `S_IS_*` flags per A12) — compute
  everything derivable at query time.
- **Unique index on `(tenant_id, level1_code, level2_code, level3_code, level4_code)`** — the legacy
  never had a real constraint here, only a client-side grid check (`03-01-a.md` §1.5's "Gap"). This
  is a `[NEW]` constraint the rebuild adds from day one.
- Level is derived from which trailing codes are zero — implement as a computed/generated column or
  a check constraint matching `03-01-a.md` §1.2's four predicates exactly.
- `child_count = 0` is what makes a node postable — enforce "only leaves accept postings" as a
  service-layer rule now; it's exercised for real once vouchers exist (2.3).
- Code digit widths come from `account_code_format` (seeded per tenant in Phase 1) — the code itself
  is never zero-padded in storage, only for display (`03-01-a.md` §1.3–§1.4). Implement one function
  that renders all the display formats (dash-joined LTR, RTL padded, name path) from the tuple —
  don't store any of them.
- CRUD API: create (with the duplicate-tuple rejection above), rename, recode (change the tuple —
  reject if it collides), promote/demote level, lock/unlock. Deletion rules from `03-02-a/b` (a node
  with children or with any posted lines cannot be deleted — read that section before implementing
  delete).

**Spec refs:** `03-01-a-account-hierarchy-model.md`, `03-01-b-...md`; `03-02-a/b-account-crud-rules.md`.

**Manual test**

1. Create a Kol node (e.g. `1` "Assets"), a Moein under it (`1-11`), a Tafsil1 (`1-11-1`), a Tafsil2
   (`1-11-1-1`). Confirm each parent's `child_count` increments correctly.
2. Attempt to create a duplicate tuple → rejected.
3. Attempt to "post" against the Moein node (not a leaf, `child_count > 0`) → rejected once 2.3
   exists; for now, confirm the API correctly reports it as non-postable.
4. Rename a node, confirm the display-format renderer reflects the new name in all four formats.
5. Attempt to delete a node with children → rejected.

**Done when:** the hierarchy can be built, queried in all four legacy display formats, and the
uniqueness/leaf-posting rules are enforced by the database, not just the client.

---

## 2.2 Chart of accounts UI

**Goal:** a tree editor covering everything `SNewu` did, minus its dead-code duplicates.

**Build**

- Tree view (kol → moein → tafsil1 → tafsil2), create/rename/recode/promote/demote/lock/delete
  actions, using the `ui-ux-pro-max` design tokens established in Phase 1.
- Merge the legacy's two competing pickers (`Sarfasl_SelectU` and `SelectSarfasl`) into **one**
  reusable account-picker component from the start (C5) — every later phase's "pick an account"
  field uses this component, not a per-screen reimplementation.

**Spec refs:** `03-12-a/b/c-screen-by-screen-ui-specification.md` (SNewu section); C5 in
`11-open-decisions.md`.

**Manual test**

1. Build the same 4-level example from 2.1 entirely through the UI.
2. Promote a Tafsil2 node to Tafsil1, confirm the tuple and tree position update correctly.
3. Lock a node, confirm a later posting attempt against it (once 2.3 exists) is blocked for
   non-admins and allowed for admins.
4. Use the account picker from two different contexts (e.g. account search and a voucher line stub)
   and confirm it's the same component, not two.

**Done when:** the chart of accounts is fully manageable through the browser with no direct DB access.

---

## 2.3 Voucher model + state machine + balance rule

**Goal:** vouchers (`DMoein`→`vouchers`) and lines (`Moein`→`voucher_lines`), state machine
`0 (draft) → 1 (issued) → 2 (permanent)`, balance enforced only on `0→1`.

**Build**

- `vouchers` and `voucher_lines` per `02-11-d-ddl-accounting-core.md`. Unlike the legacy, header
  totals (`DM_TBed`/`DM_TBes`/`DM_Count`) are **not stored** — compute `SUM(debit)`/`SUM(credit)`/
  `COUNT(*)` at read time (C3 — derive, don't store; this also structurally prevents B4/B5-style
  drift later). Keep `account_id` on each line as the single account reference; do not also store the
  4-part tuple redundantly (`03-03-a.md` §3.3's stated redundancy to resolve).
- `source_kind`/`source_id` (legacy `M_Id`/`M_Link`) mark lines generated by another domain
  (inventory, treasury). **Lines with `source_kind != 'manual'` are immutable from the voucher
  editor** — enforce this in the API, not just the UI (`03-03-a.md` §3.4's whole immutability
  table). Editing/deleting a voucher only ever touches its manual lines.
- State machine `0→1→2` with reverse transitions `1→0`, `2→1`, matching `03-03-b/c.md`. **Debit ≠
  credit is rejected only on `0→1`** — a draft is allowed to be unbalanced while being edited.
  Compare `SUM(debit)` and `SUM(credit)` as integers server-side — do **not** reproduce the legacy's
  formatted-string footer comparison (`03-04.md` §4.1 check #7's explicit rebuild instruction).
- Header validations from `03-04.md` §4.1 (voucher number required + not duplicate within the fiscal
  year, date required, narration required, at least one line) plus the **new** check the legacy
  never actually enforced despite having the helper for it: **voucher date must fall inside the
  active fiscal year's range** (`03-04.md` §4.1 "Missing validations" — `Dm.isValidDate` existed but
  `SanadEditU` never called it; the rebuild calls it).
- Line validations from `03-04.md` §4.2: account must resolve to a leaf, exactly one of debit/credit
  is nonzero and positive (both-sides-filled is prevented structurally — one field disables the
  other, not just validated after the fact), description required.
- Voucher-number allocation: `fiscal_years.next_voucher_number` (from Phase 1's schema), an atomic
  `UPDATE ... RETURNING`, not the legacy's racy `SELECT MAX(...)+1` (`10-target-architecture.md`
  §2.6 "numbering procedures").
- Deletion: only drafts (`status = 0`) with no non-manual lines can be deleted, matching
  `03-04.md` §4.5's guard order (fiscal year open → status check → confirm → delete both tables in
  one transaction).

**Spec refs:** `03-03-a/b/c-voucher-sanad-model.md`; `03-04-voucher-validation-rules.md`.

**Manual test**

1. Create a draft voucher with two unbalanced lines (debit 100, credit 50) → saves successfully as a
   draft.
2. Attempt to transition it to issued (`0→1`) → rejected, "voucher is not balanced."
3. Add a matching credit line so debit = credit → transition to issued succeeds.
4. Attempt to edit a line → succeeds (still draft... wait, it's now issued — attempt to edit a line
   on an *issued* voucher and confirm the appropriate state-based restriction applies per
   `03-03-b/c.md`).
5. Attempt to create a voucher dated outside the active fiscal year's date range → rejected.
6. Attempt to delete an issued (non-draft) voucher → rejected.
7. Create a voucher with a line whose `source_kind` is set to something other than manual (simulate,
   since no domain posts automatically yet) → confirm the API refuses to edit/delete that line
   through the voucher-editor endpoints.

**Done when:** the balance rule, state machine and immutability-of-generated-lines rule all hold
under direct API calls, matching the legacy behaviour exactly except where a spec-documented defect
is being fixed.

---

## 2.4 Voucher editor UI

**Goal:** the on-screen equivalent of `SanadEditU` — the single most-used accounting screen.

**Build**

- Line grid: add/edit/delete manual lines, running debit/credit totals, balance indicator.
  Non-manual lines render read-only with a visible marker (mirrors the legacy's disabled-edit/
  disabled-delete behaviour from `03-03-a.md` §3.4's table, but as an intentional UI state, not a
  silently-ignored click).
- State transition controls (approve/permanent-post/revert), gated by the permissions seeded in
  Phase 1 (`1113`–`1120`, `1145`).
- Account picker reused from 2.2.

**Spec refs:** `03-12-a/b/c-screen-by-screen-ui-specification.md` (SanadEditU section);
`03-13-permissions.md`.

**Manual test**

1. Walk through the same scenario as 2.3's manual test, entirely in the browser.
2. Confirm a user without the "amend voucher" permission (1114) cannot edit, and the UI reflects
   that (disabled, not hidden-but-reachable).
3. Confirm the balance indicator updates live as lines are added/edited before save.

**Done when:** a full voucher entry/approval/posting cycle works end to end in the browser.

---

## 2.5 Automatic voucher generation engine

**Goal:** the generic service other domains (treasury, inventory) will call to post their own
vouchers — built once here, wired to real callers in Phases 4–5.

**Build**

- A Rust function: given a set of `(account_id, debit_or_credit, amount, description)` tuples plus
  a `source_kind`/`source_id`, creates a voucher + lines in one transaction, marking every line
  non-manual. **Rejects if the tuples don't balance** — unlike the legacy's `MakeSanadU`, which "has
  no balance check" at all (`00-overview.md` fact 5, first bullet) and is the direct cause of B1.
  This engine is where B1 and B2 get fixed structurally: nothing calling it can produce an
  out-of-balance voucher, and "not implemented yet" is not an option this engine allows — every
  caller must supply a real posting.
- Runs inside the caller's transaction (passed `&mut Transaction`, per
  `10-target-architecture.md` §2.4) — the source document and its voucher commit together or not at
  all, closing the "half-written state" class of defect the target architecture doc calls out.
- No caller exists yet in this phase — test it directly with a stub input.

**Spec refs:** `03-06-a/b-automatic-voucher-generation.md`; `10-target-architecture.md` §2.4, §2.6.

**Manual test**

1. Call the engine directly (test endpoint or integration test) with a balanced tuple set → voucher
   + lines created, lines marked non-manual, immediately visible via 2.4's UI as read-only.
2. Call it with an unbalanced tuple set → rejected, nothing persisted (confirm no partial voucher
   row exists).
3. Force an error partway through a multi-line call (e.g. a bad account id on the third line) →
   confirm the whole transaction rolls back, no voucher row, no lines.

**Done when:** the engine cannot produce an unbalanced or partial voucher under any input.

---

## 2.6 Journal (Rooznameh) generation

**Goal:** roll a range of permanently-posted vouchers up to Kol level into a journal voucher, and
make the general ledger actually show data (fixes B6).

**Build**

- Range selection by voucher-number or by date, target voucher number/date/narration — same inputs
  as `MoeinToRU.pas` (`03-08.md` §8.1).
- Validations from `03-08.md` §8.1's table: range required and ordered, target date required and
  inside the fiscal year, target number required and not duplicate, narration required
  (`Length(Trim) > 3`), range must contain vouchers, and the key rule — **every voucher in the range
  must already be permanently posted (`status = 2`)** before it can be journalised.
- **Fix the legacy's re-run hazard**: the source predicate must exclude vouchers that are themselves
  journal vouchers (`voucher_kind = 2`) — the legacy generator's range query "does not filter on
  `M_Kind`" and can summarise a previously-generated journal voucher again (`03-08.md` §8.1 note).
  Also record, on each source voucher, that it has been journalised (a `journalised_at` timestamp or
  similar) so **re-running an overlapping range does not silently double-count** — the legacy has "no
  protection" against this beyond the target-number duplicate check; this is a decided fix, not
  optional.
- Generation logic: group by Kol, one debit line and one credit line per Kol with nonzero turnover
  (gross, not net — matching `03-08.md` §8.1's "What it produces"), `voucher_kind = 2`, resolved
  `account_id` at Kol level.
- **B6 fix**: the general ledger (Phase 6, step 6.2) must read posted (`status = 2`) voucher lines
  directly — it must not depend on someone having run this journal-generation step first. Journal
  generation is a *summary* document, not a prerequisite for the ledger to show data. Confirm this
  when building 6.2.

**Spec refs:** `03-08-journal-rooznameh-generation.md` §8.1, §8.3; B6 in `11-open-decisions.md`.

**Manual test**

1. Issue and permanently-post three vouchers touching two different Kol accounts.
2. Run journal generation over their date range → succeeds, produces one journal voucher with
   correct gross debit/credit lines per Kol.
3. Attempt to run it again over an overlapping range → either rejected or demonstrably does not
   double-count (confirm which behaviour was implemented and that it matches "fix, don't replicate").
4. Include a still-draft voucher in the range → rejected with the "must be permanently posted" message.
5. `RooznamehViewU` equivalent screen: list journal vouchers, change date/number/narration, lock,
   delete a draft one — confirm delete is blocked once posted or locked, matching `03-08.md` §8.3's
   guard table.

**Done when:** journal generation can't be re-run over the same data to inflate figures, and it is
proven **not** to be a prerequisite for the general ledger (test 6.2 against un-journalised posted
vouchers once that step exists).

---

## 2.7 Period close / year-end

**Goal:** `NewFinalu` (close income-statement accounts to a summary account) then `EnteghalU`
(carry balances forward into the new year), in that enforced order (A7).

**Build**

- **Close** (`NewFinalu` equivalent): pick a set of Kol accounts with nonzero net balance, pick a
  destination account, emit two lines per underlying leaf account (debit the destination / credit
  the source, or the reverse, per the net-balance sign) so the voucher balances by construction —
  the exact pairing algorithm in `03-09-a.md` §9.2 steps 4–5. **Fix the legacy's string-containment
  bug**: "destination must not be one of the source accounts" is checked by comparing integer
  account ids/tuples, not the legacy's sentinel-padded substring match that "silently fails open"
  when the destination code has no dash (`03-09-a.md` §9.2's validation 7 note).
- **Carry forward** (`EnteghalU` equivalent): creates the closing voucher for the outgoing year and
  the opening voucher for the incoming year. **A7 fix**: this step must verify the close (above) has
  already run for the fiscal year being carried forward — reject if it hasn't, rather than silently
  carrying forward income-statement accounts that were never zeroed (the legacy's undocumented
  assumption per A7's ruling). Both `fiscal_years.closing_voucher_id` and `opening_voucher_id`
  (from the Phase 1 schema) get populated here.
- Both operations require the superuser/admin flag, matching `03-09-a.md`'s opening note that both
  require supervisor rights.
- Drop `FinalU` (the single-Kol close) entirely — C5, superseded.

**Spec refs:** `03-09-a/b/c-period-close-and-year-end.md`; A7 in `11-open-decisions.md`.

**Manual test**

1. As a non-admin, attempt either operation → rejected.
2. As admin, attempt carry-forward on a fiscal year that has **not** been closed yet → rejected with
   a clear "close the year first" error (this is the A7 enforcement — confirm it's a hard error, not
   a warning).
3. Run close on a fiscal year with a couple of income-statement accounts holding balances → produces
   a balanced voucher zeroing those accounts into the chosen destination.
4. Post that closing voucher (via 2.3/2.4's state machine).
5. Run carry-forward → succeeds now, produces opening balances in the new fiscal year matching the
   prior year's closing balance-sheet accounts.
6. Attempt to pick a destination account that is itself one of the ticked source accounts (including
   one whose code has no dash) → rejected in both cases (this specifically re-tests the string-bug
   fix).

**Done when:** carry-forward is provably blocked until close has run for that year, and the
destination-not-in-source check can't be bypassed by code shape.

---

## 2.8 Accounting-core permissions wiring

**Goal:** every route built in 2.1–2.7 is gated by the permission catalogue seeded in Phase 1.

**Build**

- Map every accounting-core action to its permission id from `03-13-permissions.md` (chart of
  accounts: 1100–1108; subsidiary documents: 1112–1121, 1125–1127, 1142–1145; journal documents:
  1132–1140; reports: 1122–1124, 1128, 1131, 1141 — reports themselves are built in Phase 6, but
  register the ids now).
- Resolve the legacy's documented ambiguities cleanly rather than porting them: 1102/1103 ("create
  account"/"amend account") were dead/overwritten in the legacy (`08-04.md` §4.4 note) — give the
  rebuild real, distinct, working permissions for create vs amend vs delete on accounts, don't carry
  the collision forward.

**Spec refs:** `03-13-permissions.md`; `08-04-authorization.md` §4.4.

**Manual test**

1. For each accounting-core route, confirm a session with the matching permission succeeds and a
   session without it gets `403` (reuse the pattern from step 1.3's manual test, now applied to every
   route from this phase).
2. Specifically verify create-account and amend-account are independently grantable and enforced
   (the fixed version of the legacy's 1102/1103 collision).

**Done when:** no accounting-core route is reachable without its permission, and every permission id
maps to exactly the behaviour its label describes.
