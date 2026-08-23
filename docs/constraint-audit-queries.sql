-- Constraint audit query set — docs/phase-7-hardening-and-cutover.md §7.1.
--
-- One "violation count" query per constraint category Group C2 (specs/11-open-decisions.md) added
-- or confirmed. Every query returns 0 against this (freshly built, no legacy data) database — run
-- them yourself with `docker compose exec db psql -U arzi -d arzi -f docs/constraint-audit-queries.sql`
-- to confirm. Their real job starts at step 7.2: run this exact file against staged legacy-import
-- data BEFORE loading it, so a constraint violation is caught pre-load instead of failing mid-`COPY`.

\echo '-- accounts: duplicate (tenant, kol, moein, ta1, ta2) tuples --'
SELECT tenant_id, general_ledger_code, subsidiary_code, analytic1_code, analytic2_code, count(*)
FROM accounts
GROUP BY 1,2,3,4,5 HAVING count(*) > 1;

\echo '-- accounts: segment hierarchy violations (a deeper segment set without a shallower one) --'
SELECT count(*) FROM accounts
WHERE NOT ( (subsidiary_code = 0 AND analytic1_code = 0 AND analytic2_code = 0)
         OR (subsidiary_code > 0 AND analytic1_code = 0 AND analytic2_code = 0)
         OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code = 0)
         OR (subsidiary_code > 0 AND analytic1_code > 0 AND analytic2_code > 0) );

\echo '-- accounts.party_id: orphan references (would fail the FK) --'
SELECT count(*) FROM accounts a
WHERE a.party_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM parties p WHERE p.id = a.party_id);

\echo '-- vouchers: duplicate (tenant, fiscal_year, voucher_number) --'
SELECT tenant_id, fiscal_year_id, voucher_number, count(*)
FROM vouchers GROUP BY 1,2,3 HAVING count(*) > 1;

\echo '-- vouchers: non-draft but unbalanced (would fail vouchers_balanced_when_not_draft) --'
SELECT count(*) FROM vouchers WHERE status <> 'draft' AND total_debit <> total_credit;

\echo '-- voucher_lines: both-sides-set or both-zero (would fail one-side-only / not-both-zero) --'
SELECT count(*) FROM voucher_lines
WHERE (debit_amount <> 0 AND credit_amount <> 0) OR (debit_amount = 0 AND credit_amount = 0);

\echo '-- voucher_lines: negative amounts --'
SELECT count(*) FROM voucher_lines WHERE debit_amount < 0 OR credit_amount < 0;

\echo '-- inventory_documents: duplicate (tenant, fiscal_year, document_number) --'
SELECT tenant_id, fiscal_year_id, document_number, count(*)
FROM inventory_documents GROUP BY 1,2,3 HAVING count(*) > 1;

\echo '-- inventory_documents / inventory_document_lines: total identity violations --'
SELECT count(*) FROM inventory_documents
WHERE total_amount <> gross_amount + tax_amount - discount_amount;
SELECT count(*) FROM inventory_document_lines
WHERE total_amount <> gross_amount + tax_amount - discount_amount;

\echo '-- every *_id foreign-key column: orphan references, table by table --'
SELECT 'received_cheques.payer_account_id' src, count(*) FROM received_cheques c
  WHERE NOT EXISTS (SELECT 1 FROM accounts a WHERE a.id = c.payer_account_id)
UNION ALL
SELECT 'received_cheques.notes_receivable_account_id', count(*) FROM received_cheques c
  WHERE NOT EXISTS (SELECT 1 FROM accounts a WHERE a.id = c.notes_receivable_account_id)
UNION ALL
SELECT 'inventory_document_lines.item_id', count(*) FROM inventory_document_lines l
  WHERE NOT EXISTS (SELECT 1 FROM items i WHERE i.id = l.item_id)
UNION ALL
SELECT 'voucher_lines.account_id', count(*) FROM voucher_lines l
  WHERE NOT EXISTS (SELECT 1 FROM accounts a WHERE a.id = l.account_id);
-- (real FKs mean Postgres itself already refuses any of the above at insert time in this
-- database — these SELECTs exist for the day a bulk `COPY` of staged legacy data runs before its
-- FKs are attached, per 7.2's migration procedure.)

\echo '-- parties: duplicate national_id where present --'
SELECT tenant_id, national_id, count(*) FROM parties
WHERE national_id IS NOT NULL AND btrim(national_id) <> ''
GROUP BY 1,2 HAVING count(*) > 1;

\echo '-- parties: duplicate card_number --'
SELECT tenant_id, card_number, count(*) FROM parties GROUP BY 1,2 HAVING count(*) > 1;
