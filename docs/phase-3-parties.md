# Phase 3 — Parties

A "party" in the legacy system is not a table — it's a leaf account node plus, optionally, a linked
person/legal-entity record. This phase builds that model cleanly, plus the shareholder-equity logic
the legacy never had (A4).

---

## 3.1 Party register schema + CRUD

**Goal:** person/legal-entity records that, on creation, get a matching leaf account node — fixing
the legacy's two-inconsistent-links problem (`07-04-a.md` §4.3) by making it one reliable link.

**Build**

- `parties` table (`Sahamdar`→`parties`): `card_number` (business key), `kind` (`person`/
  `legal_entity`), name/surname/father's-name, ID numbers, birth/incorporation date + place,
  national ID, postal code, registration code, address, mobile, `tax_status` (enum — see below),
  `is_locked`. Drop dead columns (`S_Phone`, `S_Siba`, `S_Shanas` — confirmed unused by any live
  screen, `07-04-a.md` §4.1).
- `tax_status` enum matching the 5-value legacy combo exactly (`07-04-a.md` §4.2): not specified,
  taxpayer-required-to-register, natural-persons-article-81, not-required-to-register,
  final-consumer.
- **`party_account_config`** (`SahamdarConfig`→this table): seed the control-account rows from
  `07-07.md` §7.3 (trade receivables/payables, personnel, tenants, notes — separate seed sets for
  person vs legal-entity kind, per tenant since the chart of accounts is per-tenant). Columns:
  `control_kol_code`, `control_moein_code`, `fixed_tafsil1_code` (nullable — null means the party
  card occupies Tafsil-1; set means Tafsil-1 is fixed and the card occupies Tafsil-2), `for_person`,
  `for_legal_entity`, `offered_by_default`, `counts_toward_balance`. **Drop `SC_Tik` entirely** — it
  was a global mutable scratch column causing real cross-user corruption (B18); compute
  "does this party already have an account under this control account" per request, not as stored
  state.
- **One link, not two.** Creating a party with a ticked set of control accounts creates the
  corresponding leaf `accounts` rows (level3 or level4 = card number, per the fixed-Tafsil1 rule)
  in the same transaction, and that's the only linkage — no separate `S_Card` FK to maintain in
  parallel (the legacy's "Link 2," written from exactly one manual tool and never kept consistent
  with the primary linkage, `07-04-a.md` §4.3, is not carried forward).
- Unticking a control account in the edit UI **deletes** the corresponding account node if it has no
  postings — fixing the legacy gap where unticking never actually removed anything (`07-07.md` §7.5
  closing note, §12-Q17).
- CRUD validations from `07-03-counterparty-person-crud-validations.md` (read before implementing —
  not reproduced here in full).

**Spec refs:** `07-01-a/b`, `07-02-a/b-counterparty-taraf-model.md`, `07-03`,
`07-04-a/b-person-legal-entity-sahamdar-model.md`, `07-07-sahamdarconfig-...md`; B18 in
`11-open-decisions.md`.

**Manual test**

1. Create a natural-person party with two control accounts ticked (e.g. trade receivable, notes
   receivable) → confirm two leaf `accounts` rows are created automatically, correctly coordinated
   (card number at Tafsil-1, since no `fixed_tafsil1_code` was set for these two).
2. Create a legal-entity party with the fixed-Tafsil1 case ticked (a control account where
   `fixed_tafsil1_code` is set) → confirm the resulting leaf lands at Tafsil-2 with the card number
   there and the fixed code at Tafsil-1.
3. Edit the party, untick one control account that has no postings against it → confirm the leaf
   account is deleted, not just hidden.
4. Have two admin sessions open the same party's edit screen concurrently, one for a different
   party — confirm neither session's tick state leaks into the other's (this directly re-tests the
   B18 fix; the legacy would fail this test).

**Done when:** every party's account nodes are derivable purely from `parties` +
`party_account_config` + `accounts`, with no separate, driftable FK to keep in sync.

---

## 3.2 Party current account (Jari) + `SahamdarConfig`

**Goal:** a party's running balance — the net of every control account flagged
`counts_toward_balance`, for a chosen fiscal year.

**Build**

- Implement `Jari_Rem`'s exact algorithm (`07-06-a.md` §6.2) as a Rust function, not a stored
  procedure: for every `party_account_config` row with `counts_toward_balance = true`, resolve the
  coordinate for this party's card number (Tafsil-1 or Tafsil-2 per the fixed-code rule), sum
  `credit − debit` from posted voucher lines in the requested fiscal year, sum across all
  coordinates. **Sign convention: positive = the entity owes the party (party is a creditor);
  negative = the party owes the entity.** Preserve this exactly — don't flip it while renaming.
- The balance is computed **for a chosen fiscal year**, not implicitly the session's active one — the
  legacy form lets a user inspect any year (`07-06-a.md` §6.2 note); keep that as an explicit
  parameter.
- API endpoint returning a party's balance + the per-control-account breakdown (useful for the UI's
  running-account statement in 3.4 and the Card Jari report in Phase 6).

**Spec refs:** `07-06-a/b-party-current-account-jari.md`; `07-07-sahamdarconfig-...md`.

**Manual test**

1. Reproduce the worked example from `07-06-a.md` §6.3: seed the three control accounts (103-1,
   104-1, 301-1) for a test party, post vouchers producing debit/credit sums of
   (50,000,000 / 12,000,000), (0 / 0), (3,000,000 / 20,000,000) respectively for fiscal year 1397.
2. Call the balance endpoint for that party/year → expect exactly `−21,000,000` (party is a debtor).
3. Add postings to a Tafsil-2 coordinate (the fixed-Tafsil1 case) with credit 5,000,000 → confirm the
   total updates to `−16,000,000`, proving the Tafsil-2 path (which the legacy's write side could
   never actually reach, `07-06-a.md` §6.3's "currently unreachable" note) works end to end here.

**Done when:** the worked example from the spec reproduces exactly, for both the Tafsil-1-only and
the fixed-Tafsil1/Tafsil-2 cases.

---

## 3.3 Shareholder equity module

**Goal:** real equity logic — the legacy has none (`07-05.md`'s "derivation of absence" — confirmed
by exhaustive search, zero hits for profit/loss/capital/percent in a business sense). A4 already
decided this is in scope for the rebuild; this step designs it fresh rather than porting anything.

**Build**

- `shareholdings` table: `party_id`, `share_count`, `nominal_value`, computed `ownership_percentage`
  (share_count / total issued shares for the tenant), `join_date`, `exit_date` (nullable — active
  holding when null).
- Profit-allocation calculation: given a profit figure and a fiscal year, distribute proportionally
  to each active shareholder's `ownership_percentage` as of that year — this is genuinely new logic,
  not a port; write it with a worked example of your own and unit-test it (per
  `10-target-architecture.md` §6, this is exactly the kind of rule that "must not drift").
- Do **not** try to reverse-engineer anything from the external `Saham.Dbo` database (`\\pesteh\
  SahamData\`) — that system is not being integrated in this phase; this is a clean new subsystem
  that happens to live next to the party register.
- Keep this cleanly separated from `parties`/current-account logic (3.1–3.2) — a shareholder *is* a
  party (FK to `parties`), but owning shares and having a trade current-account balance are
  independent facts about them.

**Spec refs:** `07-05-shareholder-equity-profit-distribution.md` (read for the "what does not exist"
context); A4 in `11-open-decisions.md`.

**Manual test**

1. Create three shareholder records against existing parties with share counts 500/300/200 (total
   1000) → confirm computed ownership percentages are 50%/30%/20%.
2. Run profit distribution for a fiscal year with profit figure 100,000,000 → confirm each
   shareholder's allocation is 50,000,000/30,000,000/20,000,000.
3. Set one shareholder's `exit_date` before the fiscal year in question → confirm they're excluded
   from that year's distribution and the remaining two are reproportioned (decide and document
   whether reproportioning happens or the excluded share simply goes unallocated — pick one, test it).

**Done when:** the allocation arithmetic is correct and covered by a unit test using the worked
example above, independent of any external system.

---

## 3.4 Party UI screens

**Goal:** the on-screen equivalent of `SahamdarU` (party master list), `SahamdarEditU`/`CompanyEditU`
(person/company editors), and the party-linkage parts of `CardJariU`.

**Build**

- Party master list: persons and companies together, matching `07-10.md`'s screen spec, with
  create/edit/lock actions.
- Person/legal-entity editor: the tabbed form with the control-account tick grid from 3.1, computed
  per-request (no `SC_Tik`).
- Party card / current-account view: balance from 3.2, drill-in to the subsidiary ledger (full
  ledger rendering is Phase 6 — this screen just needs the balance and a link out).
- Attribution: every create/edit records the acting `user_id` correctly — fixes B22 (the legacy's
  hard-coded `userId = 68` in party save paths).

**Spec refs:** `07-10-screen-by-screen-ui-specification.md`; B22 in `11-open-decisions.md`.

**Manual test**

1. Create a person and a company through the UI, confirm both appear correctly labelled in the
   master list.
2. Edit an existing party as two different logged-in users (sequentially), confirm each edit's
   `audit_log` entry (from Phase 1 step 1.4) attributes the correct user, never a hard-coded one.
3. Open a party's current-account card, confirm the balance shown matches the API result from 3.2's
   manual test.

**Done when:** the full party lifecycle — create, configure control accounts, edit, view balance —
works in the browser with correct attribution on every change.
