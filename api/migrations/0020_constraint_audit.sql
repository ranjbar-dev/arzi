-- Step 7.1 (docs/phase-7-hardening-and-cutover.md §7.1 / specs/02-data-model/02-13-a/b-improvements-
-- integrity-and-keys.md, -security-and-audit.md; C2 in specs/11-open-decisions.md — "add all
-- constraints, audit data first"): the one Group-C2 constraint the audit found genuinely missing.
--
-- Every other item on 7.1's Build-bullet checklist (accounts' natural-key UNIQUE, voucher/invoice
-- number UNIQUEs, the voucher balance CHECK, voucher-line debit-xor-credit + non-negativity, every
-- account/party/cheque/deposit-slip source FK, accounts.party_id's FK, no denormalised name/code
-- columns) already existed from the phase that built it — confirmed by reading every migration
-- 0002-0019, not assumed. Two were deliberately NOT ported, both pre-existing, already-documented
-- judgment calls this step does not re-litigate:
--   * accounts.parent_id/level (§13.6) — 2.1 chose the positional 4-segment encoding instead, with
--     `level` derived from it (still C3-compliant); adding a redundant adjacency-list pointer on
--     top would itself be a new denormalisation, not a fix.
--   * cheque-number uniqueness (§13.14) — the DDL doc's own proposal
--     (02-11-e-ddl-treasury-1.md:58-61) is explicit that the correct scope is
--     `(drawer_bank_account_id, cheque_number)`, and that this "depends on §13.13 [a banks/
--     branches/cheque-book model] and should not be adopted alone" (02-13-b.md:29-30). Phase 4.1's
--     own Build bullet already chose free-text issuing-bank/branch/account-number columns over a
--     real `banks` subsystem (`received_cheques.issuing_account_number`) — §13.13 was never built,
--     by a decision already made two phases ago, not one 7.1 should reopen on its own authority.
--     A `UNIQUE (tenant_id, issuing_account_number, cheque_number)` over the free-text column would
--     be a materially weaker, self-invented substitute for the spec's actual proposal, not an
--     enablement of it — left undone and flagged here rather than faked.
--
-- What WAS actually missing: 02-11-g-ddl-inventory.md:199-205's "invoice total identity"
-- (`total_amount = subtotal + total_tax - total_deduction`), deliberately commented out in the
-- legacy-migration DDL draft because °7.7 check 3's truncation-compounding concern could reject real
-- historical invoices. This build has no historical invoices — it computes gross/tax/discount/total
-- with exact integer arithmetic from day one (api/src/inventory_documents.rs's `LineAmounts::total =
-- gross + tax - discount`, header totals maintained as pure incremental sums of those same per-line
-- deltas) — so the identity holds by construction, and the audit query below confirms zero
-- violations on the live database before the constraint is added, per the Build bullet's own
-- "audit first" instruction.

ALTER TABLE inventory_document_lines
    ADD CONSTRAINT inventory_document_lines_total_identity
        CHECK (total_amount = gross_amount + tax_amount - discount_amount);

ALTER TABLE inventory_documents
    ADD CONSTRAINT inventory_documents_total_identity
        CHECK (total_amount = gross_amount + tax_amount - discount_amount);
