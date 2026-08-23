-- Step 2.7 (docs/phase-2-accounting-core.md §2.7 / specs/03-accounting-core/03-09-a/b/c.md):
-- period close / year-end. `fiscal_years.closing_voucher_id`/`opening_voucher_id` already exist
-- (1.1's schema, FK'd to `vouchers` since 0006) and are `EnteghalU`'s (carry-forward) output.
--
-- This adds the ONE new column the step needs: a durable record that `NewFinalu` (the books-close,
-- §9.2) has actually run for a fiscal year, so `EnteghalU` (carry-forward, §9.3) can enforce A7's
-- decided ordering ("close must run before carry-forward") instead of inferring it from account
-- balances, which can't distinguish "already zeroed by a close" from "never had a balance at all".
ALTER TABLE fiscal_years
    ADD COLUMN books_closed_voucher_id bigint REFERENCES vouchers(id);

COMMENT ON COLUMN fiscal_years.books_closed_voucher_id IS
  'Set by POST .../close-books (NewFinalu equivalent) to the voucher it created. carry-forward '
  '(EnteghalU equivalent) refuses to run unless this is set AND that voucher''s status is posted '
  '-- specs/11-open-decisions.md A7.';
