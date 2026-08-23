-- Step 6.7 (docs/phase-6-reporting.md §6.7 / specs/04-reporting/04-01-a.md):
-- three report permission ids the legacy never had at all, not even an
-- unenforced one -- 03-13-permissions.md's catalogue only covers
-- accounting-core screens, and these three reports left NO permission key
-- whatsoever in the legacy ("Report5"/6-column trial balance, "Report8"/
-- debtors-creditors, and "EBC"/the tax-authority export all show "no
-- permission key" in 04-01-a.md's own catalogue table -- B24's exact
-- defect class, not a gap this migration ports, one it closes).
--
-- No legacy Pass_Config numeric id exists for any of the three (unlike
-- every other id in this catalogue, which is the legacy's own key,
-- preserved for traceability) -- ids 3001-3003 are a clearly-marked "new,
-- no legacy precedent" block, the same honesty 2.3's migration already
-- applied when it invented id 0 for the missing 'manual' journal source.
INSERT INTO permissions (id, code, label_fa) VALUES
  (3001, 'trial_balance_6_column',   'تراز آزمایشی ۶ ستونی'),
  (3002, 'debtors_creditors_report', 'بدهکاران و بستانکاران'),
  (3003, 'tax_authority_export',     'خروجی الکترونیکی دارایی');
