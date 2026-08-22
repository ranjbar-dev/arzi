_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 13. PROPOSED IMPROVEMENTS (needs user approval)

> **Nothing in this section is decided.** Sections 1–12 describe the system as it is; this section
> collects changes the rebuild *could* make. **The default position is port-as-is.** Every item
> below is a deviation from the legacy behaviour and therefore needs an explicit yes from the
> business owner before it enters §11 as anything other than a commented alternative.
>
> Each item gives: **Current** (what the legacy system does, with evidence), **Proposed**, **Why**,
> **Risk if adopted**, and **Cost of not adopting**.

Categories: **A** = data-integrity constraint the legacy system lacks; **B** = structural
remodelling; **C** = behaviour visible to the operator; **D** = security; **E** = operational.
Items marked ⚠ change what the *numbers* look like and must be signed off by an accountant, not
only by IT.

---

### 13.1 (A) Enforce the account natural key `(Ko, Mo, Ta1, Ta2)`

**Current.** Every account lookup uses the four-segment code (`Dmu.pas:1152-1156`,
`EnteghalU.dfm:344-345`, `Dmu.pas:929-968`) and `is_Sarfasl_Last_Deep` asserts `RecordCount = 1`
for a full code (`Dmu.pas:920-936`) — i.e. the application *assumes* uniqueness. There is almost
certainly no constraint enforcing it (§2.5, §12.6).
**Proposed.** `UNIQUE (general_ledger_code, subsidiary_code, analytic1_code, analytic2_code)` on
`accounts`.
**Why.** The assumption is already load-bearing; making it explicit turns a silent wrong answer
into a loud failure.
**Risk.** If duplicates exist in production the migration halts (§12.11). They must be reconciled —
which is a business decision about which of two accounts is the real one, and what happens to the
postings on the loser.
**Cost of not adopting.** Two accounts with the same code continue to be indistinguishable to every
report; `is_Sarfasl_Last_Deep` keeps failing open (§9.6).

### 13.2 (A) Enforce `UNIQUE (fiscal_year_id, voucher_number)` and `(fiscal_year_id, invoice_number)`

**Current.** Both numbers are allocated with `SELECT MAX(...) + 1` outside any transaction, from
**two different tables** for the voucher number (`Dmu.pas:1247` reads `Moein`, `MoeinToRU.pas:264`
reads `DMoein` — §5.3.1), which is a documented defect. Nothing prevents two users getting the same
number (§5.6 R8).
**Proposed.** Real unique constraints plus the per-year counter allocator already described in
§5.7.
**Risk.** Existing duplicates block the migration (§12.11). ⚠ Renumbering historical vouchers is
not acceptable to an auditor, so duplicates must be resolved by merge, not by renumber.
**Cost of not adopting.** The race survives the rebuild, and the counter design in §5.7 becomes
pointless.

### 13.3 (A) ⚠ Enforce the voucher balance rule in the database

**Current.** `SanadViewU.pas:298,301` refuses the `draft → confirmed` transition unless
`DM_TBed = DM_TBes`. That is **the only balance check in the entire system** (§2.8). Drafts may be
unbalanced, imports write unbalanced vouchers with no check at all (§10.6), and the totals
themselves are denormalised and drift (§7.7 check 4).
**Proposed.** Two layers: (a) `CHECK (status = 'draft' OR total_debit = total_credit)` on
`vouchers`; (b) a `DEFERRABLE` trigger or a service-layer assertion that `total_debit`/
`total_credit` equal `SUM` of the lines at commit.
**Why.** Preserves the legacy rule exactly (drafts may be unbalanced) while making it impossible to
bypass through the import path.
**Risk.** ⚠ If production contains confirmed-but-unbalanced vouchers — likely, given the import
path — they cannot be migrated without either a business decision per voucher or a
`status = 'legacy_unbalanced'` escape hatch. **Run the probe in §12.11 before agreeing to this.**
**Cost of not adopting.** The rebuild reproduces a ledger that does not have to balance.

### 13.4 (A) ⚠ Enforce `debit = 0 OR credit = 0` and non-negativity on voucher lines

**Current.** The convention is universal in the code but unenforced (§7.2, §2.7).
**Proposed.** `CHECK (debit_amount >= 0 AND credit_amount >= 0 AND (debit_amount = 0 OR
credit_amount = 0))` (§7.7).
**Risk.** ⚠ Any row with both sides populated, or a negative amount, is a business question — a
negative debit is arithmetically a credit but semantically a correction, and reports treat them
differently. Probe first (§7.7 check 1).

### 13.5 (A) Real foreign keys throughout

**Current.** No FK appears to be enforced anywhere: `M_Code → Sarfasl.S_SSN` is written as `0` by
the file importer (`SanadMoeinu.pas:328`, §10.6), `M_User` is hard-coded to `68` by the
carry-forward routine (`EnteghalU.pas:254`, §10.5), `DCheck2.S_Link → DCheck.S_SSN` is undeclared
(`06-treasury.md` §1.2), and the (unused) `Delete_Check` orphans history.
**Proposed.** Declare every FK identified in §2 and §11.
**Risk.** The orphan probes in §12.11 will return rows. Each orphan class needs a decision: repair,
quarantine, or a nullable FK. In particular `M_Code = 0` cannot be an FK value — those rows must be
resolved from their four-segment code first.
**Cost of not adopting.** The rebuild inherits a ledger whose lines can point at accounts that do
not exist.

### 13.6 (B) Give `accounts` an explicit `parent_id`

**Current.** The hierarchy is encoded positionally in four integer segments; a node is a leaf when
`S_Child = 0`, where `S_Child` is a **denormalised counter** maintained by application code
(`Dmu.pas:300-318`) and possibly by `Sarfasl_ADD` (§2.5). Sort order comes from `M_L`/`M_R`, whose
maintenance is **commented out** (`Dmu.pas:274-296`) and which are therefore stale in production.
**Proposed.** Add `parent_id bigint REFERENCES accounts(id)` and `level smallint`, keep the four
segments as the display/natural key, and derive `is_leaf` with `NOT EXISTS (SELECT 1 FROM accounts
c WHERE c.parent_id = a.id)` or a maintained-by-trigger counter. Drop `M_L`/`M_R`/`FullName`
entirely and derive ordering with a recursive CTE or `ltree`.
**Why.** Removes three stale denormalised columns and two undumpable UDFs (`Make_L`, `Make_R`,
§12.4) from the critical path.
**Risk.** Report ordering may change subtly where `M_L` sorting differs from segment-wise sorting —
which it will, because `M_L` is stale. ⚠ That means the *current* order is arguably already wrong;
adopting this makes the change visible in one release rather than silently.
**Cost of not adopting.** `Make_L`/`Make_R` must be reimplemented in Rust from a body that may not
be recoverable.

### 13.7 (B) Model the party↔account link explicitly

**Current.** A party gets an account by `Sarfasl_Add(Ko, Mo, S_Card, 0, name)` —
**`Sahamdar.S_Card` is written into the `Sarfasl.S_Ta1` segment** (`SahamdarEditU.pas:294-297`).
That positional encoding is the *only* link between the two tables, and `SahamdarEditU.pas:288-300`
loops over a `Coding` dataset creating one account per Kol/Moein pair (§2.6).
**Proposed.** `accounts.party_id bigint REFERENCES parties(id)`, populated by migration from the
`S_Ta1 = S_Card` correspondence, and used thereafter.
**Why.** Makes "which accounts belong to this party" a join instead of a convention, and stops
`S_Card` from being simultaneously a business key and a chart-of-accounts segment.
**Risk.** The migration must decide what to do with `S_Ta1` values that match no party and with
parties whose accounts were created by hand. Keep `analytic1_code` as-is either way — do **not**
renumber accounts.
**Cost of not adopting.** Party lookups stay positional, and changing a card number silently
detaches the accounts.

### 13.8 (B) Replace the polymorphic `(M_ID, M_Link)` pointer

**Current.** `Moein.M_ID` names a source module and `M_Link` holds the primary key **in the table
implied by `M_ID`** (§2.7). It cannot be constrained, two of the observed codes (`15`, `35`) are
unidentified, and the list is certainly incomplete (§12.9).
**Proposed.** One of: (a) nullable per-source FK columns (`cheque_id`, `deposit_slip_id`,
`inventory_invoice_id`, `petty_cash_document_id`, `cheque_payment_document_id`) with a `CHECK` that
at most one is non-null; or (b) a `voucher_line_sources` link table. Keep `source_module` as a
denormalised discriminator for reporting either way.
**Why.** Makes the drill-down from a ledger line to its source document verifiable.
**Risk.** Requires resolving `M_ID = 15` and `M_ID = 35` first (§12.9), and any unmatched `M_Link`
becomes a null.
**Cost of not adopting.** Option (a)'s absence is survivable — the legacy pattern ports directly as
`source_module smallint` + `source_id bigint` with no constraint, which is what §11 does by
default.

### 13.9 (B) ⚠ Split `Base` into `fiscal_years` + `organization` + `account_code_format`

**Current.** One `Base` row per fiscal year carries the year bounds **and** the letterhead **and**
the four account-code display widths **and** the two system-account pointers (§2.3, §8.3). So the
organisation's name, address, national ID and the code widths **can legally differ per year**.
**Proposed.** The split described in §8.6.
**Why.** A global chart of accounts with per-year display widths is incoherent (§8.6); the
letterhead is not a property of a fiscal period.
**Risk.** ⚠ **Behaviour change if the data actually differs.** Run the probe in §12.15 item 4
first. If old years genuinely carry an older company name, printing a historical document after the
change would show the *current* name — which may be legally wrong on a reprinted tax invoice.
Mitigation: keep a `fiscal_years.organization_snapshot jsonb` for historical reprints.
**Cost of not adopting.** Carry `organization_id` on `fiscal_years` and accept the duplication.

### 13.10 (B) Merge `Base.C1081`/`C1082` into `Base_Config` as one `system_accounts` table

**Current.** Two hard-coded columns on `Base` (cash, cheques-in-transit) plus a slot table
`Base_Config` keyed by an opaque integer `BC_ID` whose full value set is unknown (§2.4, §12.9).
The `C1081`/`C1082` names are almost certainly recycled permission-key numbers (§8.3).
**Proposed.** One `system_accounts (role text PRIMARY KEY, account_id bigint REFERENCES
accounts(id), label_fa text, is_enabled boolean)` with readable roles (`cash`,
`cheques_in_transit`, `notes_receivable`, `notes_in_collection`, `notes_payable`, …).
**Risk.** Needs the full `BC_ID` set from live data first, and role `11` is currently unidentified.
**Cost of not adopting.** Two settings mechanisms for one concept survive into the rebuild.

### 13.11 (B) Drop every denormalised `*CR` / `*Name` / `*StateName` column

**Current.** `Sarfasl` is denormalised into every transactional table as an account-code string and
an account-name string (`S_BedCR`, `S_BedName`, `S_BesCR`, `S_BesName`, `CM_CodeCR`, `CM_CodeName`,
`CD_BedCR`, `CD_BedName`, `TM_CodeCR`, `TM_CodeName`, `TD_BedCR`, `TD_BedName`, `M_CR`, `M_Name`,
`AFD_Name` …). §2.1 calls this "the single largest source of stale data in the schema: nothing
updates them when an account is renamed." The same applies to `S_StateName`, a Persian label
written by whichever screen last transitioned the row, with **different strings on `DCheck` and
`DCheck2` for the same event** (`06-treasury.md` §1.2).
**Proposed.** Drop them; join. Render state labels from the enum in the frontend.
**Why.** They are, by construction, wrong for any account renamed since the row was written.
**Risk.** ⚠ **This changes historical documents.** A voucher printed today would show the account's
*current* name, not the name at the time of posting. For most sites that is the desired fix; for
some it is a compliance regression. Mitigation if rejected: keep the columns but populate them from
a real snapshot at post time and document them as immutable historical facts, not as a cache.
**Cost of not adopting.** Carry the columns forward with the same staleness. **Note the asymmetry
that must be preserved either way**: some columns hold the *last segment* name
(`Taraf.Get_LastName`, `CheckDaryaftU.pas:183`, `TankhahEdit.pas:211`) and some the *full path*
(`Taraf.Get_FullName`, `CheckEditU.pas:234`, `TankhahEdit.pas:262`) — inconsistently, within the
same module.

### 13.12 (C) ⚠ Split the overloaded cheque state 1

**Current.** `S_State = 1` means both "received, never deposited" and "deposited, then bounced".
The two are distinguishable only by the free-text `S_StateName`. State `3` (the value the source
comment says means "bounced") is **never written by any code path**; the bounce screen sets the
cheque back to `1` (`CheckBargashtu.pas:209`) so it re-enters the in-hand pool. Meanwhile the
`DCheck2` audit row for that same bounce records `S_State = 2` (`CheckBargashtu.pas:214`) — the
history and the master row **disagree on every bounce** (`06-treasury.md` §2.1).
**Proposed.** A real enum: `in_hand`, `at_bank`, `bounced`, `returned_to_issuer`, `cleared`, with
`bounced` a distinct state that can transition back to `in_hand` on re-deposit; and a `bounce_count`
column so the "has this cheque bounced before?" question is answerable.
**Why.** Today the ageing report and the in-hand cheque list cannot tell a good cheque from one
that has already bounced once.
**Risk.** ⚠ Migration must classify every existing `S_State = 1` row by inspecting its `DCheck2`
history. Rows with no history are unambiguous; rows with a bounce event are `bounced`. The
classification rule needs sign-off. Any report that counts "state 1" changes its numbers.
**Cost of not adopting.** Port `status smallint` with the ambiguity intact.


---

[← 02-12-b-open-questions-schema-and-volume.md](02-12-b-open-questions-schema-and-volume.md) | [02-13-b-improvements-security-and-audit.md →](02-13-b-improvements-security-and-audit.md)
