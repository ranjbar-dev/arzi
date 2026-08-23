-- Step 2.6 (docs/phase-2-accounting-core.md §2.6 / specs/03-accounting-core/03-08-journal-
-- rooznameh-generation.md §8.1): journal (Rooznameh) generation. The legacy's `MoeinToRU` has "no
-- protection" against re-running an overlapping range and double-counting turnover (§8.1's closing
-- note) — this column is the fix: once a voucher's lines have been rolled into a journal voucher,
-- it's excluded from every later run, not just guarded by the target-number duplicate check.

ALTER TABLE vouchers ADD COLUMN journalised_at timestamptz;

COMMENT ON COLUMN vouchers.journalised_at IS
  '[NEW] Set when this voucher''s turnover has been summarised into a journal (daybook) voucher. '
  'A later journal-generation run excludes it, closing the legacy''s silent double-count hazard '
  '(03-08-journal-rooznameh-generation.md #8.1).';
