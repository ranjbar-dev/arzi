-- Removes journal (Rooznameh) generation (step 2.6) entirely: the feature is dropped, not just its
-- UI. `kind` never had a second live value once `generate_journal_voucher` is gone (every writer
-- left it at the `ledger` default), so the column and its enum go too rather than sit unused.

DROP INDEX vouchers_year_status_idx;
CREATE INDEX vouchers_year_status_idx ON vouchers (tenant_id, fiscal_year_id, status);

DROP INDEX voucher_lines_trial_balance_idx;
CREATE INDEX voucher_lines_trial_balance_idx
    ON voucher_lines (tenant_id, fiscal_year_id, status, line_date)
    INCLUDE (account_id, debit_amount, credit_amount);

ALTER TABLE vouchers DROP COLUMN journalised_at;
ALTER TABLE vouchers DROP COLUMN kind;
ALTER TABLE voucher_lines DROP COLUMN kind;

DROP TYPE journal_kind;
